//! Between the client and the webview.
//!
//! The client (`construct-client`) runs here, in Rust: keys, sessions, the stream, the store. The
//! webview draws what this module tells it — typed view events, plain data — and sends back what
//! the person did, as commands. What crosses is only what is on screen: contact names, message
//! text, a QR code as an image, account and device ids. No key, token, certificate or protocol
//! state crosses, and nothing the webview sends is more than an intent the client checks
//! (decisions/desktop-web-interface.md).

use construct_client::{
    ClientCommand, ClientConfig, ClientEvent, ClientHandle,
    bridge::BridgeEvent,
    config::SessionState,
    local_store::{Attachment, MediaInfo, StoredMessage},
    media::OpenedMedia,
    recovery::RecoveryState,
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
        version: String,
    },
    /// Where the account's recovery key stands for this device.
    Recovery {
        state: RecoveryView,
    },
    /// The phrase, shown because the person asked to make its copy. The window drops it when the
    /// dialog closes; the copy is checked by the client.
    RecoveryPhrase {
        words: Vec<String>,
    },
    RecoveryFailed {
        reason: String,
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
    /// A message on screen changed: edited, or its reactions. Drawn in place.
    MessageChanged {
        peer_id: String,
        message: MessageRow,
    },
    MessageDeleted {
        peer_id: String,
        message_id: String,
    },
    /// File `index` of a message, opened. `url` is a data URI of a picture or a sound the page
    /// shows; `None` for anything else, which is saved instead. Pictures and sounds are named by
    /// their bytes, never by what the sender declared.
    Media {
        message_id: String,
        index: usize,
        url: Option<String>,
        error: Option<String>,
    },
    /// File `index` of a message was saved in the downloads folder under `name`.
    MediaSaved {
        message_id: String,
        index: usize,
        name: String,
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
    /// Devices of this contact's account that none of its trusted devices vouched for: the
    /// moment to tell the person. The chat row keeps `unvouched` for the banner.
    Unvouched {
        peer_id: String,
        devices: usize,
    },
    /// A one-line message: `error` for failures.
    Notice {
        text: String,
        error: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RecoveryView {
    Unknown,
    CopyPending,
    Ready,
    NeedsPhrase,
}

impl From<RecoveryState> for RecoveryView {
    fn from(state: RecoveryState) -> Self {
        match state {
            RecoveryState::Unknown => Self::Unknown,
            RecoveryState::CopyPending => Self::CopyPending,
            RecoveryState::Ready => Self::Ready,
            RecoveryState::NeedsPhrase => Self::NeedsPhrase,
        }
    }
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
    /// A device of this account nobody it trusts vouched for was seen while the app ran.
    pub unvouched: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageRow {
    pub id: String,
    /// The text, or an attachment's caption.
    pub text: String,
    pub sent: bool,
    pub time: i64,
    pub delivered: bool,
    pub edited: bool,
    /// The message this one answers.
    pub quote: Option<QuoteRow>,
    /// What it carries that this window does not show yet: `photo`, `video`, `audio`, `file`,
    /// `voice`, `sticker`, `location`, `contact`, `poll`, `other`.
    pub attachment: Option<&'static str>,
    pub reactions: Vec<ReactionRow>,
    /// The files it carries, fetched one by one by index.
    pub media: Vec<MediaRow>,
}

/// A file of a message, before it is fetched: enough to draw its place.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaRow {
    /// As `attachment`.
    pub kind: &'static str,
    pub name: Option<String>,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub duration_ms: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteRow {
    pub id: String,
    pub text: String,
}

/// One emoji on a message: how many people put it there, and whether we are one of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactionRow {
    pub emoji: String,
    pub count: usize,
    pub mine: bool,
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
    Send {
        peer_id: String,
        text: String,
    },
    Reply {
        peer_id: String,
        text: String,
        to: String,
    },
    Edit {
        message_id: String,
        text: String,
    },
    React {
        message_id: String,
        emoji: String,
    },
    DeleteMessage(String),
    /// Send a file the person chose; `caption` goes under it.
    SendFile {
        peer_id: String,
        name: String,
        bytes: Vec<u8>,
        caption: String,
    },
    /// Show file `index` of a message.
    LoadMedia {
        message_id: String,
        index: usize,
    },
    /// Save file `index` of a message in the downloads folder.
    SaveMedia {
        message_id: String,
        index: usize,
    },
    Find(String),
    AddContact {
        user_id: String,
        name: String,
    },
    DeleteContact(String),
    SafetyNumber(String),
    MintInvite,
    RevealRecoveryPhrase,
    ConfirmRecoveryCopy(Vec<(usize, String)>),
    EnterRecoveryPhrase(Zeroizing<String>),
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
    recovery: RecoveryView,
    /// Our account, to tell our reactions from others'.
    me: String,
    /// Where saved files go.
    downloads: std::path::PathBuf,
    /// Files asked for to be saved, not shown: (message, index).
    saving: std::collections::HashSet<(String, usize)>,
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
            unvouched: false,
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
pub async fn run(
    config: ClientConfig,
    downloads: std::path::PathBuf,
    mut intents: mpsc::UnboundedReceiver<Intent>,
) {
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
        recovery: RecoveryView::Unknown,
        me: String::new(),
        downloads,
        saving: Default::default(),
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
        Intent::Reply { peer_id, text, to } => {
            let text = text.trim().to_string();
            if !text.is_empty() {
                client.send(ClientCommand::Reply {
                    contact_id: peer_id,
                    text,
                    to,
                });
            }
        }
        Intent::Edit { message_id, text } => {
            let text = text.trim().to_string();
            if !text.is_empty() {
                client.send(ClientCommand::EditMessage { message_id, text });
            }
        }
        Intent::React { message_id, emoji } => {
            client.send(ClientCommand::React { message_id, emoji })
        }
        Intent::DeleteMessage(message_id) => {
            client.send(ClientCommand::DeleteMessage { message_id })
        }
        Intent::SendFile {
            peer_id,
            name,
            bytes,
            caption,
        } => client.send(ClientCommand::SendFile {
            contact_id: peer_id,
            name,
            bytes,
            caption,
        }),
        Intent::LoadMedia { message_id, index } => {
            client.send(ClientCommand::LoadMedia { message_id, index })
        }
        Intent::SaveMedia { message_id, index } => {
            model.saving.insert((message_id.clone(), index));
            client.send(ClientCommand::LoadMedia { message_id, index })
        }
        Intent::Find(query) => client.send(ClientCommand::FindContact { query }),
        Intent::AddContact { user_id, name } => client.send(ClientCommand::AddContact {
            user_id,
            username: name,
        }),
        Intent::DeleteContact(peer_id) => client.send(ClientCommand::DeleteContact { peer_id }),
        Intent::SafetyNumber(contact_id) => client.send(ClientCommand::SafetyNumber { contact_id }),
        Intent::MintInvite => client.send(ClientCommand::MintInvite),
        Intent::RevealRecoveryPhrase => client.send(ClientCommand::RevealRecoveryPhrase),
        Intent::ConfirmRecoveryCopy(answers) => {
            client.send(ClientCommand::ConfirmRecoveryCopy { answers })
        }
        Intent::EnterRecoveryPhrase(phrase) => {
            client.send(ClientCommand::EnterRecoveryPhrase(phrase))
        }
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
    events.push(ViewEvent::Recovery {
        state: model.recovery,
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
            model.me = info.user_id.clone();
            let account = ViewEvent::Account {
                user_id: info.user_id,
                device_id: info.device_id,
                server: server.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
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
                    unvouched: false,
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
            quote,
        } => {
            let time = now_ms();
            let row = MessageRow {
                id: message_id,
                text,
                sent: true,
                time,
                delivered: false,
                edited: false,
                quote: quote.map(|q| QuoteRow {
                    id: q.message_id,
                    text: q.preview,
                }),
                attachment: None,
                reactions: Vec::new(),
                media: Vec::new(),
            };
            let chat = model.chat_mut(&contact_id);
            chat.preview = preview(&row);
            chat.time = time;
            vec![
                ViewEvent::Message {
                    peer_id: contact_id,
                    message: row,
                },
                ViewEvent::Chats {
                    chats: model.sorted_chats(),
                },
            ]
        }
        ClientEvent::History { peer_id, messages } => {
            let me = model.me.clone();
            let messages: Vec<MessageRow> =
                messages.into_iter().map(|m| message_row(m, &me)).collect();
            if let Some(last) = messages.last() {
                let chat = model.chat_mut(&peer_id);
                if last.time >= chat.time {
                    chat.preview = preview(last);
                    chat.time = last.time;
                }
            }
            vec![
                ViewEvent::History { peer_id, messages },
                ViewEvent::Chats {
                    chats: model.sorted_chats(),
                },
            ]
        }
        ClientEvent::Media {
            message_id,
            index,
            result,
        } => {
            let save = model.saving.remove(&(message_id.clone(), index));
            vec![match result {
                Err(reason) => ViewEvent::Media {
                    message_id,
                    index,
                    url: None,
                    error: Some(reason),
                },
                Ok(media) if save => match save_media(&model.downloads, &media) {
                    Ok(name) => ViewEvent::MediaSaved {
                        message_id,
                        index,
                        name,
                    },
                    Err(e) => notice(format!("Not saved: {e}"), true),
                },
                Ok(media) => ViewEvent::Media {
                    message_id,
                    index,
                    url: data_uri(&media),
                    error: None,
                },
            }]
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
            model.recovery = RecoveryView::Unknown;
            vec![
                screen(model, Screen::Onboarding),
                ViewEvent::Chats { chats: Vec::new() },
            ]
        }
        ClientEvent::Notice(text) => vec![notice(text, false)],
        ClientEvent::Recovery(state) => {
            model.recovery = state.into();
            vec![ViewEvent::Recovery {
                state: model.recovery,
            }]
        }
        ClientEvent::RecoveryPhrase(Ok(phrase)) => vec![ViewEvent::RecoveryPhrase {
            words: phrase.split_whitespace().map(String::from).collect(),
        }],
        ClientEvent::RecoveryPhrase(Err(reason)) | ClientEvent::RecoveryFailed(reason) => {
            vec![ViewEvent::RecoveryFailed { reason }]
        }
    }
}

fn on_bridge_event(event: BridgeEvent, model: &mut Model) -> Vec<ViewEvent> {
    let incoming = |model: &mut Model, peer_id: String, row: MessageRow| {
        let open = model.open_chat.as_deref() == Some(peer_id.as_str());
        let chat = model.chat_mut(&peer_id);
        chat.preview = preview(&row);
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
        BridgeEvent::NewMessage { message } | BridgeEvent::OwnMessage { message } => {
            let peer_id = message.peer_id.clone();
            let row = message_row(message, &model.me);
            incoming(model, peer_id, row)
        }
        BridgeEvent::MessageChanged { message } => vec![ViewEvent::MessageChanged {
            peer_id: message.peer_id.clone(),
            message: message_row(message, &model.me),
        }],
        BridgeEvent::MessageDeleted {
            peer_id,
            message_id,
        } => vec![ViewEvent::MessageDeleted {
            peer_id,
            message_id,
        }],
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
        // Only told, never blocked: the devices still receive messages (owner, 2026-10-23).
        BridgeEvent::UnvouchedDevices {
            peer_id,
            device_ids,
        } => {
            model.chat_mut(&peer_id).unvouched = true;
            vec![
                ViewEvent::Unvouched {
                    peer_id,
                    devices: device_ids.len(),
                },
                ViewEvent::Chats {
                    chats: model.sorted_chats(),
                },
            ]
        }
    }
}

/// A stored message as the transcript draws it; `me` is our account.
fn message_row(m: StoredMessage, me: &str) -> MessageRow {
    let mut reactions: Vec<ReactionRow> = Vec::new();
    for r in &m.reactions {
        let mine = r.reactor_id == me;
        match reactions.iter_mut().find(|row| row.emoji == r.emoji) {
            Some(row) => {
                row.count += 1;
                row.mine |= mine;
            }
            None => reactions.push(ReactionRow {
                emoji: r.emoji.clone(),
                count: 1,
                mine,
            }),
        }
    }
    MessageRow {
        sent: m.direction == "sent",
        delivered: m.delivery_status == "delivered",
        edited: m.edited,
        quote: m.quote.map(|q| QuoteRow {
            id: q.message_id,
            text: q.preview,
        }),
        attachment: m.attachment.map(attachment_name),
        reactions,
        media: m.media.into_iter().map(media_row).collect(),
        id: m.id,
        text: m.text,
        time: m.timestamp_ms,
    }
}

fn media_row(m: MediaInfo) -> MediaRow {
    MediaRow {
        kind: attachment_name(m.kind.unwrap_or(Attachment::File)),
        name: m.filename,
        size: m.size,
        width: m.width,
        height: m.height,
        duration_ms: m.duration_ms,
    }
}

/// Larger than this, a file is saved rather than put in the page.
const MAX_INLINE_BYTES: usize = 25 * 1024 * 1024;

/// What a file is by its first bytes, among what the page shows: a picture or a sound.
fn sniff(bytes: &[u8], declared_mime: &str) -> Option<&'static str> {
    let at = |offset: usize, magic: &[u8]| bytes.get(offset..offset + magic.len()) == Some(magic);
    if at(0, &[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if at(0, b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if at(0, b"GIF87a") || at(0, b"GIF89a") {
        Some("image/gif")
    } else if at(0, b"RIFF") && at(8, b"WEBP") {
        Some("image/webp")
    } else if at(0, b"RIFF") && at(8, b"WAVE") {
        Some("audio/wav")
    } else if at(0, b"OggS") {
        Some("audio/ogg")
    } else if at(0, b"ID3") {
        Some("audio/mpeg")
    // A voice note is AAC in an MP4 box, as a video is; the declared type tells them apart, and
    // calling a video a sound only fails to play it.
    } else if at(4, b"ftyp") && !declared_mime.starts_with("video/") {
        Some("audio/mp4")
    } else {
        None
    }
}

fn data_uri(media: &OpenedMedia) -> Option<String> {
    use base64::Engine;
    if media.bytes.len() > MAX_INLINE_BYTES {
        return None;
    }
    let mime = sniff(&media.bytes, &media.mime)?;
    let body = base64::engine::general_purpose::STANDARD.encode(&media.bytes);
    Some(format!("data:{mime};base64,{body}"))
}

/// Write `media` into `dir` under its own name, or a made-up one, never over a file there.
fn save_media(dir: &std::path::Path, media: &OpenedMedia) -> std::io::Result<String> {
    use std::io::Write;
    let name = media.filename.clone().unwrap_or_else(|| {
        let ext = match sniff(&media.bytes, &media.mime) {
            Some(mime) => mime.rsplit('/').next().unwrap_or("bin"),
            None => "bin",
        };
        let ext = if ext == "jpeg" {
            "jpg"
        } else if ext == "mp4" {
            "m4a"
        } else {
            ext
        };
        format!("konstruct-{}.{ext}", now_ms())
    });
    let (stem, ext) = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => (stem.to_string(), format!(".{ext}")),
        _ => (name.clone(), String::new()),
    };
    std::fs::create_dir_all(dir)?;
    for n in 0..1000 {
        let candidate = if n == 0 {
            name.clone()
        } else {
            format!("{stem} ({n}){ext}")
        };
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.join(&candidate))
        {
            Ok(mut file) => {
                file.write_all(&media.bytes)?;
                return Ok(candidate);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::other("too many files of that name"))
}

fn attachment_name(a: Attachment) -> &'static str {
    match a {
        Attachment::Photo => "photo",
        Attachment::Video => "video",
        Attachment::Audio => "audio",
        Attachment::File => "file",
        Attachment::Voice => "voice",
        Attachment::Sticker => "sticker",
        Attachment::Location => "location",
        Attachment::Contact => "contact",
        Attachment::Poll => "poll",
        Attachment::Other => "other",
    }
}

/// The chat list's line for a message. An attachment is marked, the window words it.
fn preview(row: &MessageRow) -> String {
    match row.attachment {
        Some(_) if row.text.is_empty() => "📎".into(),
        Some(_) => format!("📎 {}", row.text),
        None => row.text.clone(),
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
            recovery: RecoveryView::Unknown,
            me: "me".into(),
            downloads: std::env::temp_dir(),
            saving: Default::default(),
        }
    }

    fn received(peer: &str, id: &str, text: &str, time: i64) -> BridgeEvent {
        BridgeEvent::NewMessage {
            message: StoredMessage {
                id: id.into(),
                peer_id: peer.into(),
                text: text.into(),
                direction: "received".into(),
                timestamp_ms: time,
                ..Default::default()
            },
        }
    }

    #[test]
    fn a_message_to_a_closed_chat_counts_as_unread_and_moves_it_up() {
        let mut m = model();
        m.chat_mut("old").time = 5;
        m.chat_mut("bob").time = 1;
        m.open_chat = Some("old".into());

        on_bridge_event(received("bob", "m1", "hi", 10), &mut m);

        let chats = m.sorted_chats();
        assert_eq!(chats[0].peer_id, "bob");
        assert_eq!(chats[0].unread, 1);
        assert_eq!(chats[0].preview, "hi");
    }

    #[test]
    fn a_message_to_the_open_chat_is_not_unread() {
        let mut m = model();
        m.open_chat = Some("bob".into());
        on_bridge_event(received("bob", "m1", "hi", 10), &mut m);
        assert_eq!(m.chats[0].unread, 0);
    }

    #[test]
    fn an_unvouched_device_is_told_and_marks_the_chat() {
        let mut m = model();
        let out = on_bridge_event(
            BridgeEvent::UnvouchedDevices {
                peer_id: "bob".into(),
                device_ids: vec!["d1".into(), "d2".into()],
            },
            &mut m,
        );
        assert!(matches!(
            &out[0],
            ViewEvent::Unvouched { peer_id, devices: 2 } if peer_id == "bob"
        ));
        assert!(m.sorted_chats()[0].unvouched);
    }

    #[test]
    fn the_recovery_state_is_kept_for_a_reloaded_page_and_the_phrase_is_not() {
        let mut m = model();
        on_client_event(
            ClientEvent::Recovery(RecoveryState::CopyPending),
            &mut m,
            "s",
        );
        let out = on_client_event(
            ClientEvent::RecoveryPhrase(Ok(Zeroizing::new("a b c".into()))),
            &mut m,
            "s",
        );
        assert!(
            matches!(&out[0], ViewEvent::RecoveryPhrase { words } if words == &["a", "b", "c"])
        );
        let replay = snapshot(&m);
        assert!(replay.iter().any(|e| matches!(
            e,
            ViewEvent::Recovery {
                state: RecoveryView::CopyPending
            }
        )));
        assert!(
            !replay
                .iter()
                .any(|e| matches!(e, ViewEvent::RecoveryPhrase { .. }))
        );
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
                edited: false,
                quote: None,
                attachment: None,
                reactions: Vec::new(),
                media: Vec::new(),
            },
        })
        .unwrap();
        assert_eq!(json["type"], "message");
        assert_eq!(json["peerId"], "bob");
        assert_eq!(json["message"]["sent"], true);
    }

    #[test]
    fn reactions_are_counted_by_emoji_and_ours_is_marked() {
        use construct_client::local_store::StoredReaction;
        let reaction = |who: &str, emoji: &str| StoredReaction {
            reactor_id: who.into(),
            emoji: emoji.into(),
        };
        let row = message_row(
            StoredMessage {
                reactions: vec![
                    reaction("bob", "👍"),
                    reaction("me", "👍"),
                    reaction("carol", "❤️"),
                ],
                ..Default::default()
            },
            "me",
        );
        assert_eq!(
            row.reactions,
            [
                ReactionRow {
                    emoji: "👍".into(),
                    count: 2,
                    mine: true
                },
                ReactionRow {
                    emoji: "❤️".into(),
                    count: 1,
                    mine: false
                },
            ]
        );
    }

    #[test]
    fn an_attachment_is_marked_in_the_chat_list() {
        let mut m = model();
        let mut event = received("bob", "m1", "", 10);
        if let BridgeEvent::NewMessage { message } = &mut event {
            message.attachment = Some(Attachment::Photo);
        }
        let out = on_bridge_event(event, &mut m);
        assert_eq!(m.chats[0].preview, "📎");
        let json = serde_json::to_value(&out[0]).unwrap();
        assert_eq!(json["message"]["attachment"], "photo");
    }

    #[test]
    fn the_link_code_becomes_an_image_the_csp_allows() {
        let image = qr_image("konstruct://link-to-me?id=abc&fp=00");
        assert!(image.starts_with("data:image/svg+xml;utf8,"));
        assert!(!image.contains('<') && !image.contains('#') && !image.contains('"'));
    }

    fn opened(mime: &str, name: Option<&str>, bytes: &[u8]) -> OpenedMedia {
        OpenedMedia {
            mime: mime.into(),
            filename: name.map(Into::into),
            bytes: bytes.to_vec(),
        }
    }

    #[test]
    fn a_file_is_shown_by_what_its_bytes_are_not_what_the_sender_said() {
        let png = b"\x89PNG\r\n\x1a\n....";
        let uri = data_uri(&opened("text/html", None, png)).unwrap();
        assert!(uri.starts_with("data:image/png;base64,"), "{uri}");
        assert_eq!(
            data_uri(&opened("image/jpeg", None, b"<svg onload=x>")),
            None
        );
        let m4a = b"\0\0\0\x20ftypM4A ....";
        assert!(
            data_uri(&opened("audio/m4a", None, m4a))
                .unwrap()
                .starts_with("data:audio/mp4;")
        );
        assert_eq!(
            data_uri(&opened("video/mp4", None, m4a)),
            None,
            "a video is saved"
        );
        let huge = vec![0xFFu8, 0xD8, 0xFF]
            .into_iter()
            .chain(std::iter::repeat_n(0, MAX_INLINE_BYTES))
            .collect::<Vec<_>>();
        assert_eq!(data_uri(&opened("image/jpeg", None, &huge)), None);
    }

    #[test]
    fn a_saved_file_never_replaces_one_there() {
        let dir = std::env::temp_dir().join(format!("konstruct-save-{}", now_ms()));
        let media = opened("application/pdf", Some("report.pdf"), b"new");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("report.pdf"), b"mine").unwrap();
        assert_eq!(save_media(&dir, &media).unwrap(), "report (1).pdf");
        assert_eq!(save_media(&dir, &media).unwrap(), "report (2).pdf");
        assert_eq!(std::fs::read(dir.join("report.pdf")).unwrap(), b"mine");
        assert_eq!(std::fs::read(dir.join("report (1).pdf")).unwrap(), b"new");
        let unnamed = save_media(&dir, &opened("", None, &[0xFF, 0xD8, 0xFF, 0])).unwrap();
        assert!(
            unnamed.starts_with("konstruct-") && unnamed.ends_with(".jpg"),
            "{unnamed}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_file_asked_to_be_saved_is_not_put_in_the_page() {
        let dir = std::env::temp_dir().join(format!("konstruct-save-asked-{}", now_ms()));
        let mut model = model();
        model.downloads = dir.clone();
        model.saving.insert(("m".into(), 0));
        let media = |index| ClientEvent::Media {
            message_id: "m".into(),
            index,
            result: Ok(opened("image/png", Some("p.png"), b"\x89PNG\r\n\x1a\n")),
        };
        let out = on_client_event(media(0), &mut model, "");
        assert!(matches!(&out[..], [ViewEvent::MediaSaved { name, .. }] if name == "p.png"));
        let out = on_client_event(media(0), &mut model, "");
        assert!(
            matches!(&out[..], [ViewEvent::Media { url: Some(_), .. }]),
            "once only"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
