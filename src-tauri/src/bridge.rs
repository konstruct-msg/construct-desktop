//! Between the client and the webview.
//!
//! The client (`construct-client`) runs here, in Rust: keys, sessions, the stream, the store. The
//! webview draws what this module tells it — typed view events, plain data — and sends back what
//! the person did, as commands. What crosses is only what is on screen: contact names, message
//! text, a QR code as an image, account and device ids. No key, token, certificate or protocol
//! state crosses, and nothing the webview sends is more than an intent the client checks
//! (decisions/desktop-web-interface.md).

use construct_client::{
    ClientCommand, ClientConfig, ClientEvent, ClientHandle, bridge::BridgeEvent,
    config::SessionState, local_store::StoredMessage,
};
use serde::Serialize;
use tauri::ipc::Channel;
use tokio::sync::mpsc;
use zeroize::Zeroizing;

/// What the webview shows. One message per change; `Snapshot` after a (re)load.
#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ViewEvent {
    Screen {
        screen: Screen,
    },
    Account {
        user_id: String,
        device_id: String,
        server: String,
    },
    Chats {
        chats: Vec<ChatRow>,
    },
    History {
        peer_id: String,
        messages: Vec<MessageRow>,
    },
    Message {
        peer_id: String,
        message: MessageRow,
    },
    Delivered {
        message_id: String,
    },
    Connection {
        connected: bool,
    },
    RegistrationStep {
        done: usize,
        total: usize,
        label: String,
    },
    /// The code the phone scans, as an SVG data URI the webview puts in an `<img>`.
    LinkCode {
        image: String,
    },
    LinkFailed {
        reason: String,
    },
    SearchResults {
        results: Vec<ChatRow>,
    },
    SafetyNumber {
        peer_id: String,
        number: String,
    },
    Invite {
        link: Option<String>,
        /// The link as a QR image, like `LinkCode`.
        image: Option<String>,
        error: Option<String>,
    },
    /// A one-line message: `error` for failures.
    Notice {
        text: String,
        error: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Screen {
    Starting,
    Onboarding,
    Registering,
    Link,
    Unlock,
    SetPassphrase,
    Main,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRow {
    pub peer_id: String,
    pub name: String,
    pub preview: String,
    /// Unix milliseconds of the last message, 0 when none.
    pub time: i64,
    pub unread: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageRow {
    pub id: String,
    pub text: String,
    pub sent: bool,
    pub time: i64,
    pub delivered: bool,
}

/// What the person did in the webview.
pub enum Intent {
    Attach(Channel<ViewEvent>),
    Register(String),
    StartLink,
    CancelLink,
    Unlock(Zeroizing<Vec<u8>>),
    SetPassphrase(Zeroizing<Vec<u8>>),
    OpenChat(String),
    Send { peer_id: String, text: String },
    Find(String),
    AddContact { user_id: String, name: String },
    DeleteContact(String),
    SafetyNumber(String),
    MintInvite,
    Logout,
}

const REGISTRATION_STEPS: &[&str] = &[
    "Generating signing key",
    "Generating identity key",
    "Generating signed prekey",
    "Signing prekey",
    "Connecting",
    "Solving proof of work",
    "Registering",
];

/// The model the webview is drawn from: kept here, so a reloaded page gets everything back.
struct Model {
    screen: Screen,
    account: Option<ViewEvent>,
    chats: Vec<ChatRow>,
    open_chat: Option<String>,
    connected: bool,
    link_code: Option<String>,
}

impl Model {
    fn chat_mut(&mut self, peer_id: &str) -> &mut ChatRow {
        if let Some(i) = self.chats.iter().position(|c| c.peer_id == peer_id) {
            return &mut self.chats[i];
        }
        self.chats.push(ChatRow {
            peer_id: peer_id.to_string(),
            name: short_id(peer_id),
            preview: String::new(),
            time: 0,
            unread: 0,
        });
        self.chats.last_mut().expect("just pushed")
    }

    /// Most recent first, chats with no message after them by name.
    fn sorted_chats(&self) -> Vec<ChatRow> {
        let mut chats = self.chats.clone();
        chats.sort_by(|a, b| b.time.cmp(&a.time).then_with(|| a.name.cmp(&b.name)));
        chats
    }
}

/// Run the client and the model until the window closes.
pub async fn run(config: ClientConfig, mut intents: mpsc::UnboundedReceiver<Intent>) {
    let (event_tx, mut events) = mpsc::unbounded_channel();
    let server = config.server_url.clone();
    let client = construct_client::spawn(config, event_tx);
    let mut model = Model {
        screen: Screen::Starting,
        account: None,
        chats: Vec::new(),
        open_chat: None,
        connected: false,
        link_code: None,
    };
    model.screen = match construct_client::stored_session_state() {
        SessionState::Encrypted => Screen::Unlock,
        SessionState::Plaintext => {
            client.send(ClientCommand::RestoreFromDisk);
            Screen::Starting
        }
        SessionState::None => Screen::Onboarding,
    };
    let mut view: Option<Channel<ViewEvent>> = None;

    loop {
        tokio::select! {
            intent = intents.recv() => {
                let Some(intent) = intent else { break };
                handle_intent(intent, &client, &mut model, &mut view);
            }
            event = events.recv() => {
                let Some(event) = event else { break };
                // Each chat's last message, for the list's preview.
                if let ClientEvent::Started(contacts) = &event {
                    for contact in contacts {
                        client.send(ClientCommand::LoadHistory {
                            peer_id: contact.user_id.clone(),
                            limit: 1,
                        });
                    }
                }
                for out in on_client_event(event, &mut model, &server) {
                    if let Some(view) = &view {
                        // A closed channel is a page being reloaded; it attaches again.
                        let _ = view.send(out);
                    }
                }
            }
        }
    }
}

fn handle_intent(
    intent: Intent,
    client: &ClientHandle,
    model: &mut Model,
    view: &mut Option<Channel<ViewEvent>>,
) {
    let send = |view: &Option<Channel<ViewEvent>>, event: ViewEvent| {
        if let Some(view) = view {
            let _ = view.send(event);
        }
    };
    match intent {
        Intent::Attach(channel) => {
            for event in snapshot(model) {
                let _ = channel.send(event);
            }
            *view = Some(channel);
        }
        Intent::Register(username) => {
            client.send(ClientCommand::Register { username });
            model.screen = Screen::Registering;
            send(
                view,
                ViewEvent::Screen {
                    screen: model.screen,
                },
            );
        }
        Intent::StartLink => {
            model.link_code = None;
            client.send(ClientCommand::LinkByQr);
            model.screen = Screen::Link;
            send(
                view,
                ViewEvent::Screen {
                    screen: model.screen,
                },
            );
        }
        Intent::CancelLink => {
            client.send(ClientCommand::CancelLink);
            model.screen = Screen::Onboarding;
            send(
                view,
                ViewEvent::Screen {
                    screen: model.screen,
                },
            );
        }
        Intent::Unlock(passphrase) => client.send(ClientCommand::Unlock(passphrase)),
        Intent::SetPassphrase(passphrase) => client.send(ClientCommand::SetPassphrase(passphrase)),
        Intent::OpenChat(peer_id) => {
            model.chat_mut(&peer_id).unread = 0;
            model.open_chat = Some(peer_id.clone());
            client.send(ClientCommand::LoadHistory {
                peer_id,
                limit: 200,
            });
            send(
                view,
                ViewEvent::Chats {
                    chats: model.sorted_chats(),
                },
            );
        }
        Intent::Send { peer_id, text } => {
            let text = text.trim().to_string();
            if !text.is_empty() {
                client.send(ClientCommand::SendText {
                    contact_id: peer_id,
                    text,
                });
            }
        }
        Intent::Find(query) => client.send(ClientCommand::FindContact { query }),
        Intent::AddContact { user_id, name } => client.send(ClientCommand::AddContact {
            user_id,
            username: name,
        }),
        Intent::DeleteContact(peer_id) => client.send(ClientCommand::DeleteContact { peer_id }),
        Intent::SafetyNumber(contact_id) => client.send(ClientCommand::SafetyNumber { contact_id }),
        Intent::MintInvite => client.send(ClientCommand::MintInvite),
        Intent::Logout => client.send(ClientCommand::Logout),
    }
}

fn snapshot(model: &Model) -> Vec<ViewEvent> {
    let mut events = vec![ViewEvent::Screen {
        screen: model.screen,
    }];
    if let Some(account) = &model.account {
        events.push(account.clone());
    }
    events.push(ViewEvent::Chats {
        chats: model.sorted_chats(),
    });
    events.push(ViewEvent::Connection {
        connected: model.connected,
    });
    if let Some(code) = &model.link_code {
        events.push(ViewEvent::LinkCode {
            image: qr_image(code),
        });
    }
    events
}

/// A client event as what changes on screen.
fn on_client_event(event: ClientEvent, model: &mut Model, server: &str) -> Vec<ViewEvent> {
    let screen = |model: &mut Model, screen: Screen| {
        model.screen = screen;
        ViewEvent::Screen { screen }
    };
    match event {
        ClientEvent::Account(info) => {
            if info.user_id.is_empty() {
                return Vec::new();
            }
            let account = ViewEvent::Account {
                user_id: info.user_id,
                device_id: info.device_id,
                server: server.to_string(),
            };
            model.account = Some(account.clone());
            vec![account]
        }
        ClientEvent::Started(contacts) => {
            for contact in contacts {
                model.chat_mut(&contact.user_id).name = contact.display_name;
            }
            vec![
                screen(model, Screen::Main),
                ViewEvent::Chats {
                    chats: model.sorted_chats(),
                },
            ]
        }
        ClientEvent::NeedsPassphrase => vec![screen(model, Screen::SetPassphrase)],
        ClientEvent::AuthFailed { reason, no_session } => {
            let mut out = vec![screen(model, Screen::Onboarding)];
            if !no_session {
                out.push(notice(reason, true));
            }
            out
        }
        ClientEvent::SaveFailed(reason) => vec![notice(reason, true)],
        ClientEvent::UnlockFailed(reason) | ClientEvent::PassphraseFailed(reason) => {
            vec![notice(reason, true)]
        }
        ClientEvent::RegistrationStep(step) => {
            let done = step.index();
            vec![ViewEvent::RegistrationStep {
                done,
                total: REGISTRATION_STEPS.len(),
                label: REGISTRATION_STEPS
                    .get(done)
                    .copied()
                    .unwrap_or_default()
                    .to_string(),
            }]
        }
        ClientEvent::Bridge(bridge) => on_bridge_event(bridge, model),
        ClientEvent::SearchStarted(_) => Vec::new(),
        ClientEvent::ContactSearchResult(results) => vec![ViewEvent::SearchResults {
            results: results
                .into_iter()
                .map(|r| ChatRow {
                    peer_id: r.user_id,
                    name: r.display_name,
                    preview: String::new(),
                    time: 0,
                    unread: 0,
                })
                .collect(),
        }],
        ClientEvent::ContactSearchError(reason) => vec![notice(reason, true)],
        ClientEvent::ContactAdded { user_id, username } => {
            model.chat_mut(&user_id).name = username;
            vec![ViewEvent::Chats {
                chats: model.sorted_chats(),
            }]
        }
        ClientEvent::ContactRemoved { peer_id } => {
            model.chats.retain(|c| c.peer_id != peer_id);
            if model.open_chat.as_deref() == Some(peer_id.as_str()) {
                model.open_chat = None;
            }
            vec![ViewEvent::Chats {
                chats: model.sorted_chats(),
            }]
        }
        ClientEvent::MessageQueued {
            contact_id,
            message_id,
            text,
        } => {
            let time = now_ms();
            let chat = model.chat_mut(&contact_id);
            chat.preview = text.clone();
            chat.time = time;
            vec![
                ViewEvent::Message {
                    peer_id: contact_id,
                    message: MessageRow {
                        id: message_id,
                        text,
                        sent: true,
                        time,
                        delivered: false,
                    },
                },
                ViewEvent::Chats {
                    chats: model.sorted_chats(),
                },
            ]
        }
        ClientEvent::History { peer_id, messages } => {
            if let Some(last) = messages.last() {
                let chat = model.chat_mut(&peer_id);
                if last.timestamp_ms >= chat.time {
                    chat.preview = last.text.clone();
                    chat.time = last.timestamp_ms;
                }
            }
            vec![
                ViewEvent::History {
                    peer_id,
                    messages: messages.into_iter().map(message_row).collect(),
                },
                ViewEvent::Chats {
                    chats: model.sorted_chats(),
                },
            ]
        }
        ClientEvent::SafetyNumber { contact_id, number } => vec![ViewEvent::SafetyNumber {
            peer_id: contact_id,
            number,
        }],
        ClientEvent::InviteMinted(result) => vec![match result {
            Ok(link) => ViewEvent::Invite {
                image: Some(qr_image(&link)),
                link: Some(link),
                error: None,
            },
            Err(e) => ViewEvent::Invite {
                link: None,
                image: None,
                error: Some(e),
            },
        }],
        ClientEvent::LinkCode(code) => {
            let image = qr_image(&code);
            model.link_code = Some(code);
            vec![ViewEvent::LinkCode { image }]
        }
        ClientEvent::LinkFailed(reason) => {
            model.link_code = None;
            vec![ViewEvent::LinkFailed { reason }]
        }
        ClientEvent::LoggedOut => {
            model.chats.clear();
            model.open_chat = None;
            model.account = None;
            model.link_code = None;
            vec![
                screen(model, Screen::Onboarding),
                ViewEvent::Chats { chats: Vec::new() },
            ]
        }
        ClientEvent::Notice(text) => vec![notice(text, false)],
    }
}

fn on_bridge_event(event: BridgeEvent, model: &mut Model) -> Vec<ViewEvent> {
    let incoming = |model: &mut Model, peer_id: String, row: MessageRow| {
        let open = model.open_chat.as_deref() == Some(peer_id.as_str());
        let chat = model.chat_mut(&peer_id);
        chat.preview = row.text.clone();
        chat.time = row.time;
        if !open && !row.sent {
            chat.unread += 1;
        }
        vec![
            ViewEvent::Message {
                peer_id,
                message: row,
            },
            ViewEvent::Chats {
                chats: model.sorted_chats(),
            },
        ]
    };
    match event {
        BridgeEvent::NewMessage {
            peer_id,
            message_id,
            text,
            timestamp_ms,
        } => incoming(
            model,
            peer_id,
            MessageRow {
                id: message_id,
                text,
                sent: false,
                time: timestamp_ms,
                delivered: false,
            },
        ),
        BridgeEvent::OwnMessage {
            peer_id,
            message_id,
            text,
            timestamp_ms,
        } => incoming(
            model,
            peer_id,
            MessageRow {
                id: message_id,
                text,
                sent: true,
                time: timestamp_ms,
                delivered: false,
            },
        ),
        BridgeEvent::MessageDelivered { message_id } => vec![ViewEvent::Delivered { message_id }],
        BridgeEvent::StreamStatus { connected } => {
            model.connected = connected;
            vec![ViewEvent::Connection { connected }]
        }
        BridgeEvent::StreamReconnecting { .. } => {
            model.connected = false;
            vec![ViewEvent::Connection { connected: false }]
        }
        BridgeEvent::Error(text) => {
            tracing::warn!(error = %text, "client error");
            vec![notice(text, true)]
        }
        BridgeEvent::SessionReady { .. } => Vec::new(),
    }
}

fn message_row(m: StoredMessage) -> MessageRow {
    MessageRow {
        sent: m.direction == "sent",
        delivered: m.delivery_status == "delivered",
        id: m.id,
        text: m.text,
        time: m.timestamp_ms,
    }
}

fn notice(text: String, error: bool) -> ViewEvent {
    ViewEvent::Notice { text, error }
}

fn short_id(id: &str) -> String {
    format!("{}…", &id[..8.min(id.len())])
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// `code` as a QR in an SVG data URI: drawn here, so the webview needs no QR library.
fn qr_image(code: &str) -> String {
    use qrcode::render::svg;
    let svg = match qrcode::QrCode::new(code.as_bytes()) {
        Ok(qr) => qr
            .render::<svg::Color<'_>>()
            .min_dimensions(280, 280)
            .quiet_zone(true)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#ffffff"))
            .build(),
        Err(e) => {
            tracing::error!(error = %e, "link code does not fit a QR");
            return String::new();
        }
    };
    format!("data:image/svg+xml;utf8,{}", percent_encode_svg(&svg))
}

/// Encode what a `data:` URI cannot carry as is.
fn percent_encode_svg(svg: &str) -> String {
    svg.chars()
        .map(|c| match c {
            '%' => "%25".to_string(),
            '#' => "%23".to_string(),
            '<' => "%3C".to_string(),
            '>' => "%3E".to_string(),
            '"' => "'".to_string(),
            '\n' | '\r' => String::new(),
            c => c.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> Model {
        Model {
            screen: Screen::Main,
            account: None,
            chats: Vec::new(),
            open_chat: None,
            connected: true,
            link_code: None,
        }
    }

    #[test]
    fn a_message_to_a_closed_chat_counts_as_unread_and_moves_it_up() {
        let mut m = model();
        m.chat_mut("old").time = 5;
        m.chat_mut("bob").time = 1;
        m.open_chat = Some("old".into());

        on_bridge_event(
            BridgeEvent::NewMessage {
                peer_id: "bob".into(),
                message_id: "m1".into(),
                text: "hi".into(),
                timestamp_ms: 10,
            },
            &mut m,
        );

        let chats = m.sorted_chats();
        assert_eq!(chats[0].peer_id, "bob");
        assert_eq!(chats[0].unread, 1);
        assert_eq!(chats[0].preview, "hi");
    }

    #[test]
    fn a_message_to_the_open_chat_is_not_unread() {
        let mut m = model();
        m.open_chat = Some("bob".into());
        on_bridge_event(
            BridgeEvent::NewMessage {
                peer_id: "bob".into(),
                message_id: "m1".into(),
                text: "hi".into(),
                timestamp_ms: 10,
            },
            &mut m,
        );
        assert_eq!(m.chats[0].unread, 0);
    }

    #[test]
    fn view_events_are_camel_case_and_tagged() {
        let json = serde_json::to_value(ViewEvent::Message {
            peer_id: "bob".into(),
            message: MessageRow {
                id: "m".into(),
                text: "t".into(),
                sent: true,
                time: 1,
                delivered: false,
            },
        })
        .unwrap();
        assert_eq!(json["type"], "message");
        assert_eq!(json["peerId"], "bob");
        assert_eq!(json["message"]["sent"], true);
    }

    #[test]
    fn the_link_code_becomes_an_image_the_csp_allows() {
        let image = qr_image("konstruct://link-to-me?id=abc&fp=00");
        assert!(image.starts_with("data:image/svg+xml;utf8,"));
        assert!(!image.contains('<') && !image.contains('#') && !image.contains('"'));
    }
}
