//! Between the client and the webview.
//!
//! The client (`construct-client`) runs here, in Rust: keys, sessions, the stream, the store. The
//! webview draws what this module tells it — typed view events, plain data — and sends back what
//! the person did, as commands. What crosses is only what is on screen: contact names, message
//! text, a QR code as an image, account and device ids. No key, token, certificate or protocol
//! state crosses, and nothing the webview sends is more than an intent the client checks
//! (decisions/desktop-web-interface.md).

use construct_client::media::AutoDownload;
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
        /// The name generated from the account id, as every client computes it.
        name: String,
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
    /// The code the phone scans, as an SVG data URI the webview puts in an `<img>`, and as text
    /// to copy to another computer: it names this device and its public keys, nothing secret.
    LinkCode {
        image: String,
        code: String,
    },
    /// The code a new device scans to join this account, as an image only: it carries a secret
    /// (Flow A), which stays out of the page's text. Valid until `expires_at`, Unix ms.
    LinkOffer {
        image: String,
        expires_at: i64,
    },
    /// The offer ended: `device` linked, or `error` says why not.
    LinkOfferEnded {
        device: Option<String>,
        error: Option<String>,
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
    /// A person's picture, as a data URI of the image their profile carried; `None`: none.
    /// Sent once per change, apart from the chat list, which is redrawn far more often.
    Avatar {
        peer_id: String,
        url: Option<String>,
    },
    /// The account as the settings screen shows it.
    Settings {
        /// The name we chose to share; empty when none.
        display_name: String,
        /// Our picture, as `Avatar`'s.
        avatar: Option<String>,
        username: Option<String>,
        /// What this device calls itself to the account's others.
        device_name: String,
        devices: Vec<DeviceRow>,
        delivery_receipts: bool,
        discoverable: Option<bool>,
        /// Our identity key, short, to compare out of band.
        fingerprint: Option<String>,
        /// What the fetched attachments take.
        media_bytes: u64,
        media_files: usize,
        /// `never`, `small` or `always`.
        auto_download: &'static str,
        /// Bytes; `None`: no limit.
        cache_limit: Option<u64>,
        /// `None`: kept always.
        keep_days: Option<u32>,
        invites: Vec<InviteRow>,
    },
    /// A history transfer by file ended: `exported` (written to `file`) or imported; `error` when
    /// it did not happen.
    HistoryTransfer {
        exported: bool,
        error: Option<String>,
        messages: usize,
        media: usize,
        file: Option<String>,
    },
    /// A transfer over the local network, this device `sending` or receiving: `state` is
    /// `waiting`, `moving`, `done`, `skipped` or `failed` (with `error`); what was sent, when done.
    HistoryNearby {
        sending: bool,
        state: &'static str,
        error: Option<String>,
        messages: usize,
        media: usize,
    },
    /// The app lock closed the window: the page drops what it shows and asks for the code.
    Locked,
    /// The code matched; the page is drawn again from what follows.
    Unlocked,
    UnlockFailed {
        reason: String,
    },
    /// Whether the app lock is on, and its delay in seconds.
    LockSetting {
        enabled: bool,
        delay_secs: u32,
    },
    /// The server did not delete the account; nothing was wiped.
    DeleteFailed {
        reason: String,
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
    /// We share our profile with them.
    pub sharing: bool,
    /// The name we gave them ourselves.
    pub alias: Option<String>,
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

/// A device of our account in the settings screen.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRow {
    pub id: String,
    /// Unix milliseconds; 0 when the server does not say.
    pub created: i64,
    pub current: bool,
    pub primary: bool,
    /// What it calls itself, when a copy was sealed to this device.
    pub name: Option<String>,
    /// `ios`, `android`, `desktop`, … or empty.
    pub platform: &'static str,
}

/// An invite this device handed out that may still be redeemed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteRow {
    pub jti: String,
    pub kind: String,
    /// Unix milliseconds.
    pub issued: i64,
    pub expires: i64,
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
    /// The window gained or lost the focus.
    Focus(bool),
    Notifications(Notifications),
    LoadSettings,
    ChangeUsername(String),
    SetDiscoverable(bool),
    SetDeliveryReceipts(bool),
    /// Sign out another device; `None`: every other one.
    RevokeDevices(Option<String>),
    /// Our shared name, picture, or both; `None` keeps the name.
    SetProfile {
        display_name: Option<String>,
        avatar: construct_client::AvatarChange,
    },
    ShareProfile {
        peer_id: String,
        on: bool,
    },
    SetAlias {
        peer_id: String,
        alias: Option<String>,
    },
    RenameDevice(String),
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
    LogoutEverywhere,
    LockNow,
    /// Write this device's history for `device_id` into the downloads folder.
    ExportHistory(String),
    /// A history file the person chose, its bytes.
    ImportHistory(Vec<u8>),
    /// Send this device's history to `device_id` over the local network.
    OfferHistoryNearby(String),
    /// Take history another device of the account offers over the local network.
    ReceiveHistoryNearby,
    CancelHistoryNearby,
    RevokeInvites(Vec<String>),
    UnlockApp(Zeroizing<String>),
    SetAppLock {
        current: Option<Zeroizing<String>>,
        code: Option<Zeroizing<String>>,
        remove: bool,
        delay_secs: u32,
    },
    /// Show a code for a new device to scan.
    OfferLink,
    CancelLinkOffer,
    /// Let the device whose `link-to-me` code was pasted join this account.
    ApproveDevice(String),
    DeleteAccount {
        local_only: bool,
    },
    ClearMediaCache,
    SetMediaPrefs(construct_client::local_store::MediaPrefs),
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
    /// How a message that arrives is announced to the system, as the page last set it.
    notify: Notifications,
    /// The window has the focus: a message in the open chat is then seen, not announced.
    focused: bool,
    /// People's pictures as data URIs, for a reloaded page.
    avatars: std::collections::HashMap<String, String>,
    lock: Lock,
    /// The copy of a history file the client is reading.
    importing: Option<std::path::PathBuf>,
    /// A transfer over the local network under way: whether this device sends.
    nearby: Option<bool>,
}

/// The app lock as this window keeps it. While `locked`, the page is sent nothing of the account
/// and what it asks is not done, but for the code; that is the lock, not the page's overlay.
#[derive(Debug, Default)]
struct Lock {
    enabled: bool,
    delay_secs: u32,
    locked: bool,
    /// Whether the client has said yet: the first word, at a start, closes the window.
    told: bool,
    /// When the window lost the focus.
    away_since: Option<std::time::Instant>,
}

impl Lock {
    fn set(&mut self, lock: construct_client::AppLock) {
        if lock.enabled && !self.told {
            self.locked = true;
        }
        self.told = true;
        self.enabled = lock.enabled;
        self.delay_secs = lock.delay_secs;
        if !lock.enabled {
            self.locked = false;
        }
    }

    /// Away for longer than the delay, and not locked yet.
    fn due(&self, now: std::time::Instant) -> bool {
        self.enabled
            && !self.locked
            && self.away_since.is_some_and(|t| {
                now.duration_since(t) >= std::time::Duration::from_secs(self.delay_secs.into())
            })
    }

    /// What the page may be sent while the window is locked.
    fn lets_through(event: &ViewEvent) -> bool {
        matches!(
            event,
            ViewEvent::Screen { .. }
                | ViewEvent::Connection { .. }
                | ViewEvent::Locked
                | ViewEvent::Unlocked
                | ViewEvent::UnlockFailed { .. }
                | ViewEvent::LockSetting { .. }
        )
    }
}

/// What of `outs` the page is sent: everything, or while locked only what the lock lets through.
fn visible(model: &Model, outs: Vec<ViewEvent>) -> Vec<ViewEvent> {
    if !model.lock.locked {
        return outs;
    }
    outs.into_iter().filter(Lock::lets_through).collect()
}

/// Close the window if the lock is due.
fn lock_if_due(model: &mut Model) -> Vec<ViewEvent> {
    if model.lock.due(std::time::Instant::now()) {
        model.lock.locked = true;
        vec![ViewEvent::Locked]
    } else {
        Vec::new()
    }
}

/// The person's choice of what a notification of a new message says. Kept by the page with its
/// other settings and handed here at each start and change.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notifications {
    pub enabled: bool,
    /// `nameAndMessage`, `nameOnly` or `none`, as iOS names them.
    pub preview: String,
    /// "New message", in the page's language.
    pub new_message: String,
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            enabled: true,
            preview: "nameAndMessage".into(),
            new_message: "New message".into(),
        }
    }
}

fn auto_download_name(policy: AutoDownload) -> &'static str {
    match policy {
        AutoDownload::Never => "never",
        AutoDownload::Small => "small",
        AutoDownload::Always => "always",
    }
}

/// The page's name for a fetch-on-arrival choice; anything else is the default.
pub fn auto_download(name: &str) -> AutoDownload {
    match name {
        "never" => AutoDownload::Never,
        "always" => AutoDownload::Always,
        _ => AutoDownload::Small,
    }
}

/// What the system is asked to show for a message that arrived, or nothing: notifications off,
/// our own message, or one in the chat the person is looking at.
fn notification(model: &Model, peer_id: &str, row: &MessageRow) -> Option<(String, String)> {
    let n = &model.notify;
    let watching = model.focused && model.open_chat.as_deref() == Some(peer_id);
    if !n.enabled || row.sent || watching {
        return None;
    }
    // A locked window says nothing of who wrote or what.
    if model.lock.locked {
        return Some(("Konstruct".into(), n.new_message.clone()));
    }
    let name = model
        .chats
        .iter()
        .find(|c| c.peer_id == peer_id)
        .map_or_else(
            || construct_client::names::generated(peer_id),
            |c| c.name.clone(),
        );
    Some(match n.preview.as_str() {
        "nameAndMessage" => (name, preview(row)),
        "nameOnly" => (name, n.new_message.clone()),
        _ => ("Konstruct".into(), n.new_message.clone()),
    })
}

/// Off the async runtime: on Linux the call goes over D-Bus and may wait on the daemon.
#[cfg(not(target_os = "macos"))]
fn show_notification(title: String, body: String) {
    std::thread::spawn(move || {
        match notify_rust::Notification::new()
            .appname("Konstruct")
            .summary(&title)
            .body(&body)
            .show()
        {
            Ok(_) => tracing::debug!("notification shown"),
            Err(e) => tracing::warn!("notification not shown: {e}"),
        }
    });
}

/// macOS is not a target; the window there is for development.
#[cfg(target_os = "macos")]
fn show_notification(_title: String, _body: String) {
    tracing::debug!("notification left out on macOS");
}

impl Model {
    fn chat_mut(&mut self, peer_id: &str) -> &mut ChatRow {
        if let Some(i) = self.chats.iter().position(|c| c.peer_id == peer_id) {
            return &mut self.chats[i];
        }
        self.chats.push(ChatRow {
            peer_id: peer_id.to_string(),
            name: construct_client::names::generated(peer_id),
            preview: String::new(),
            time: 0,
            unread: 0,
            unvouched: false,
            sharing: false,
            alias: None,
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
        notify: Notifications::default(),
        focused: true,
        avatars: Default::default(),
        lock: Lock::default(),
        importing: None,
        nearby: None,
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
    // While the window is away, the lock's delay runs out without anything happening.
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));

    loop {
        tokio::select! {
            _ = tick.tick() => {
                for out in lock_if_due(&mut model) {
                    if let Some(view) = &view {
                        let _ = view.send(out);
                    }
                }
            }
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
                let outs = on_client_event(event, &mut model, &server);
                for out in visible(&model, outs) {
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
    if model.lock.locked
        && !matches!(
            intent,
            Intent::Attach(_)
                | Intent::UnlockApp(_)
                | Intent::Focus(_)
                | Intent::Notifications(_)
                | Intent::Logout
        )
    {
        return tracing::info!("locked: intent not done");
    }
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
        Intent::Focus(focused) => {
            model.focused = focused;
            let now = std::time::Instant::now();
            if focused {
                // Back after the delay ran out between two ticks.
                for event in lock_if_due(model) {
                    send(view, event);
                }
                model.lock.away_since = None;
            } else {
                model.lock.away_since = Some(now);
                if model.lock.delay_secs == 0 {
                    for event in lock_if_due(model) {
                        send(view, event);
                    }
                }
            }
        }
        Intent::LockNow => {
            if model.lock.enabled && !model.lock.locked {
                model.lock.locked = true;
                send(view, ViewEvent::Locked);
            }
        }
        Intent::UnlockApp(code) => client.send(ClientCommand::UnlockApp(code)),
        Intent::ExportHistory(device_id) => {
            let stem = format!("konstruct-history-{}", &device_id[..8.min(device_id.len())]);
            match free_path(&model.downloads, &stem, "cthf") {
                Ok(path) => client.send(ClientCommand::ExportHistory { device_id, path }),
                Err(e) => send(view, notice(format!("History not written: {e}"), true)),
            }
        }
        Intent::ImportHistory(bytes) => {
            // Read by the client from a file: written aside, deleted once it is read.
            let path = std::env::temp_dir().join(format!("konstruct-import-{}.cthf", now_ms()));
            match std::fs::write(&path, &bytes) {
                Ok(()) => {
                    model.importing = Some(path.clone());
                    client.send(ClientCommand::ImportHistory(path));
                }
                Err(e) => send(view, notice(format!("History not imported: {e}"), true)),
            }
        }
        Intent::OfferHistoryNearby(device_id) => {
            model.nearby = Some(true);
            client.send(ClientCommand::OfferHistoryNearby { device_id });
        }
        Intent::ReceiveHistoryNearby => {
            model.nearby = Some(false);
            client.send(ClientCommand::ReceiveHistoryNearby);
        }
        Intent::CancelHistoryNearby => {
            model.nearby = None;
            client.send(ClientCommand::CancelHistoryNearby);
        }
        Intent::RevokeInvites(jtis) => client.send(ClientCommand::RevokeInvites(jtis)),
        Intent::SetAppLock {
            current,
            code,
            remove,
            delay_secs,
        } => client.send(ClientCommand::SetAppLock {
            current,
            code,
            remove,
            delay_secs,
        }),
        Intent::Notifications(notify) => model.notify = notify,
        Intent::LoadSettings => client.send(ClientCommand::LoadSettings),
        Intent::ChangeUsername(name) => client.send(ClientCommand::ChangeUsername(name)),
        Intent::SetDiscoverable(on) => client.send(ClientCommand::SetDiscoverable(on)),
        Intent::SetDeliveryReceipts(on) => client.send(ClientCommand::SetDeliveryReceipts(on)),
        Intent::RevokeDevices(id) => client.send(ClientCommand::RevokeDevices(id)),
        Intent::SetProfile {
            display_name,
            avatar,
        } => client.send(ClientCommand::SetProfile {
            display_name,
            avatar,
        }),
        Intent::ShareProfile { peer_id, on } => client.send(ClientCommand::ShareProfile {
            contact_id: peer_id,
            on,
        }),
        Intent::SetAlias { peer_id, alias } => client.send(ClientCommand::SetAlias {
            contact_id: peer_id,
            alias,
        }),
        Intent::RenameDevice(name) => client.send(ClientCommand::RenameDevice(name)),
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
        Intent::LogoutEverywhere => client.send(ClientCommand::LogoutEverywhere),
        Intent::OfferLink => client.send(ClientCommand::OfferLink),
        Intent::CancelLinkOffer => client.send(ClientCommand::CancelLinkOffer),
        Intent::ApproveDevice(code) => match construct_client::device_certs::parse_join_code(&code)
        {
            Some(join) => client.send(ClientCommand::ApproveJoin {
                device_id: join.device_id,
                fingerprint: join.fingerprint,
            }),
            None => send(
                view,
                notice("That is not a device's link code".into(), true),
            ),
        },
        Intent::DeleteAccount { local_only } => {
            client.send(ClientCommand::DeleteAccount { local_only })
        }
        Intent::ClearMediaCache => client.send(ClientCommand::ClearMediaCache),
        Intent::SetMediaPrefs(prefs) => client.send(ClientCommand::SetMediaPrefs(prefs)),
    }
}

fn snapshot(model: &Model) -> Vec<ViewEvent> {
    let mut events = vec![ViewEvent::Screen {
        screen: model.screen,
    }];
    if model.lock.told {
        events.push(ViewEvent::LockSetting {
            enabled: model.lock.enabled,
            delay_secs: model.lock.delay_secs,
        });
    }
    if model.lock.locked {
        events.push(ViewEvent::Connection {
            connected: model.connected,
        });
        events.push(ViewEvent::Locked);
        return events;
    }
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
    for (peer_id, url) in &model.avatars {
        events.push(ViewEvent::Avatar {
            peer_id: peer_id.clone(),
            url: Some(url.clone()),
        });
    }
    if let Some(code) = &model.link_code {
        events.push(ViewEvent::LinkCode {
            image: qr_image(code),
            code: code.clone(),
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
                name: construct_client::names::generated(&info.user_id),
                user_id: info.user_id,
                device_id: info.device_id,
                server: server.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            };
            model.account = Some(account.clone());
            vec![account]
        }
        ClientEvent::Started(contacts) => {
            let mut avatars = Vec::new();
            for contact in contacts {
                let chat = model.chat_mut(&contact.user_id);
                chat.name = contact.display_name;
                chat.sharing = contact.sharing;
                chat.alias = contact.alias;
                if let Some(url) = contact.avatar.as_deref().and_then(avatar_uri) {
                    model.avatars.insert(contact.user_id.clone(), url.clone());
                    avatars.push(ViewEvent::Avatar {
                        peer_id: contact.user_id,
                        url: Some(url),
                    });
                }
            }
            let mut out = vec![
                screen(model, Screen::Main),
                ViewEvent::Chats {
                    chats: model.sorted_chats(),
                },
            ];
            out.extend(avatars);
            out
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
                    sharing: false,
                    alias: None,
                })
                .collect(),
        }],
        ClientEvent::ContactSearchError(reason) => vec![notice(reason, true)],
        ClientEvent::ContactAdded { user_id, username } => {
            model.chat_mut(&user_id).name =
                construct_client::names::resolved(None, "", &username, &user_id);
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
            model.link_code = Some(code.clone());
            vec![ViewEvent::LinkCode { image, code }]
        }
        ClientEvent::LinkFailed(reason) => {
            model.link_code = None;
            vec![ViewEvent::LinkFailed { reason }]
        }
        ClientEvent::LinkOffer { code, expires_at } => vec![ViewEvent::LinkOffer {
            image: qr_image(&code),
            expires_at: expires_at.saturating_mul(1000),
        }],
        ClientEvent::LinkOfferEnded(ended) => vec![match ended {
            Ok(device) => ViewEvent::LinkOfferEnded {
                device: Some(device),
                error: None,
            },
            Err(error) => ViewEvent::LinkOfferEnded {
                device: None,
                error: Some(error),
            },
        }],
        ClientEvent::LoggedOut => {
            model.lock = Lock::default();
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
        ClientEvent::Settings(s) => vec![ViewEvent::Settings {
            display_name: s.display_name,
            avatar: s.avatar.as_deref().and_then(avatar_uri),
            username: s.username,
            device_name: s.device_name,
            devices: s
                .devices
                .into_iter()
                .map(|e| DeviceRow {
                    current: e.device.is_current || e.device.device_id == own_device(model),
                    name: e.description.as_ref().map(|d| d.name.clone()),
                    platform: e.description.map_or("", |d| d.platform),
                    id: e.device.device_id,
                    created: e.device.created_at.saturating_mul(1000),
                    primary: e.device.is_primary,
                })
                .collect(),
            delivery_receipts: s.delivery_receipts,
            discoverable: s.discoverable,
            fingerprint: s.fingerprint,
            media_bytes: s.media_bytes,
            media_files: s.media_files,
            auto_download: auto_download_name(s.media.auto_download),
            cache_limit: s.media.quota,
            keep_days: s.media.keep_days,
            invites: s
                .invites
                .into_iter()
                .map(|i| InviteRow {
                    jti: i.jti,
                    kind: i.kind,
                    issued: i.issued_at_ms,
                    expires: i.expires_at_ms,
                })
                .collect(),
        }],
        ClientEvent::DeleteAccountFailed(reason) => vec![ViewEvent::DeleteFailed { reason }],
        ClientEvent::AppLock(lock) => {
            let was = model.lock.locked;
            model.lock.set(lock);
            let mut out = vec![ViewEvent::LockSetting {
                enabled: lock.enabled,
                delay_secs: lock.delay_secs,
            }];
            if model.lock.locked && !was {
                out.push(ViewEvent::Locked);
            }
            out
        }
        ClientEvent::AppUnlocked(Ok(())) => {
            model.lock.locked = false;
            model.lock.away_since = None;
            let mut out = vec![ViewEvent::Unlocked];
            out.extend(snapshot(model));
            out
        }
        ClientEvent::AppUnlocked(Err(reason)) => vec![ViewEvent::UnlockFailed { reason }],
        ClientEvent::AppLockRefused(reason) => vec![notice(reason, true)],
        ClientEvent::HistoryExported(result) => vec![match result {
            Ok((s, path)) => ViewEvent::HistoryTransfer {
                exported: true,
                error: None,
                messages: s.messages,
                media: s.media,
                file: path.file_name().map(|n| n.to_string_lossy().into_owned()),
            },
            Err(e) => history_failed(true, e),
        }],
        ClientEvent::HistoryImported(result) => {
            if let Some(path) = model.importing.take() {
                let _ = std::fs::remove_file(path);
            }
            let mut out = vec![match result {
                Ok(s) => ViewEvent::HistoryTransfer {
                    exported: false,
                    error: None,
                    messages: s.messages,
                    media: s.media,
                    file: None,
                },
                Err(e) => history_failed(false, e),
            }];
            // History received over the network lands as an import; that ends the transfer.
            if model.nearby == Some(false) {
                model.nearby = None;
                out.push(nearby_step(false, "done", None, None));
            }
            out
        }
        ClientEvent::HistoryNearby(step) => {
            use construct_client::history::NearbyStep;
            // A step after a cancel is the stopped task's last word: not shown.
            let Some(sending) = model.nearby else {
                return Vec::new();
            };
            vec![match step {
                NearbyStep::Waiting => nearby_step(sending, "waiting", None, None),
                NearbyStep::Moving => nearby_step(sending, "moving", None, None),
                NearbyStep::Sent(s) => {
                    model.nearby = None;
                    nearby_step(sending, "done", None, Some(s))
                }
                NearbyStep::Skipped => {
                    model.nearby = None;
                    nearby_step(sending, "skipped", None, None)
                }
                NearbyStep::Failed(e) => {
                    model.nearby = None;
                    nearby_step(sending, "failed", Some(e), None)
                }
            }]
        }
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
            if let Some((title, body)) = notification(model, &peer_id, &row) {
                show_notification(title, body);
            }
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
        BridgeEvent::ContactUpdated { contact } => {
            let chat = model.chat_mut(&contact.user_id);
            chat.name = contact.display_name;
            chat.sharing = contact.sharing;
            chat.alias = contact.alias;
            let url = contact.avatar.as_deref().and_then(avatar_uri);
            match &url {
                Some(url) => model.avatars.insert(contact.user_id.clone(), url.clone()),
                None => model.avatars.remove(&contact.user_id),
            };
            vec![
                ViewEvent::Chats {
                    chats: model.sorted_chats(),
                },
                ViewEvent::Avatar {
                    url,
                    peer_id: contact.user_id,
                },
            ]
        }
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

/// A profile picture as the page shows it: a data URI, only of a JPEG or PNG by its bytes, the
/// zeros it was padded with on the wire cut off.
fn avatar_uri(bytes: &[u8]) -> Option<String> {
    use base64::Engine;
    let mime = sniff(bytes, "")?;
    if !mime.starts_with("image/") {
        return None;
    }
    let end = bytes.iter().rposition(|b| *b != 0).map_or(0, |i| i + 1);
    let body = base64::engine::general_purpose::STANDARD.encode(&bytes[..end]);
    Some(format!("data:{mime};base64,{body}"))
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
fn history_failed(exported: bool, error: String) -> ViewEvent {
    ViewEvent::HistoryTransfer {
        exported,
        error: Some(error),
        messages: 0,
        media: 0,
        file: None,
    }
}

fn nearby_step(
    sending: bool,
    state: &'static str,
    error: Option<String>,
    sent: Option<construct_client::history::HistorySummary>,
) -> ViewEvent {
    let sent = sent.unwrap_or_default();
    ViewEvent::HistoryNearby {
        sending,
        state,
        error,
        messages: sent.messages,
        media: sent.media,
    }
}

/// `dir/stem.ext`, or `stem (n).ext` when that is taken.
fn free_path(dir: &std::path::Path, stem: &str, ext: &str) -> std::io::Result<std::path::PathBuf> {
    std::fs::create_dir_all(dir)?;
    (0..1000)
        .map(|n| {
            dir.join(if n == 0 {
                format!("{stem}.{ext}")
            } else {
                format!("{stem} ({n}).{ext}")
            })
        })
        .find(|p| !p.exists())
        .ok_or_else(|| std::io::Error::other("no free name"))
}

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

/// This device's id, from the account the page was told of.
fn own_device(model: &Model) -> String {
    match &model.account {
        Some(ViewEvent::Account { device_id, .. }) => device_id.clone(),
        _ => String::new(),
    }
}

fn notice(text: String, error: bool) -> ViewEvent {
    ViewEvent::Notice { text, error }
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
            notify: Notifications::default(),
            focused: true,
            avatars: Default::default(),
            lock: Lock::default(),
            importing: None,
            nearby: None,
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

    fn lock_on(delay_secs: u32) -> ClientEvent {
        ClientEvent::AppLock(construct_client::AppLock {
            enabled: true,
            delay_secs,
        })
    }

    #[test]
    fn a_window_with_a_lock_starts_locked_and_shows_nothing_until_the_code() {
        let mut model = model();
        let outs = on_client_event(lock_on(60), &mut model, "");
        assert!(outs.iter().any(|e| matches!(e, ViewEvent::Locked)));
        let outs = on_client_event(
            ClientEvent::Bridge(received("bob", "m1", "secret plans", 1)),
            &mut model,
            "",
        );
        let json = serde_json::to_string(&visible(&model, outs)).unwrap();
        assert!(
            !json.contains("secret plans") && !json.contains("bob"),
            "{json}"
        );
        let reload = serde_json::to_string(&snapshot(&model)).unwrap();
        assert!(
            !reload.contains("secret plans") && !reload.contains("\"chats\""),
            "{reload}"
        );
        let outs = on_client_event(ClientEvent::AppUnlocked(Ok(())), &mut model, "");
        let json = serde_json::to_string(&visible(&model, outs)).unwrap();
        assert!(
            json.contains("secret plans"),
            "unlocked, the page is drawn again"
        );
    }

    #[test]
    fn a_lock_turned_on_later_does_not_close_the_window_at_once() {
        let mut model = model();
        on_client_event(
            ClientEvent::AppLock(construct_client::AppLock {
                enabled: false,
                delay_secs: 0,
            }),
            &mut model,
            "",
        );
        let outs = on_client_event(lock_on(0), &mut model, "");
        assert!(!outs.iter().any(|e| matches!(e, ViewEvent::Locked)));
        assert!(!model.lock.locked);
    }

    #[test]
    fn the_lock_falls_when_the_window_has_been_away_its_delay() {
        let now = std::time::Instant::now();
        let mut lock = Lock {
            enabled: true,
            delay_secs: 60,
            told: true,
            ..Default::default()
        };
        assert!(!lock.due(now), "not away");
        lock.away_since = Some(now);
        assert!(!lock.due(now + std::time::Duration::from_secs(59)));
        assert!(lock.due(now + std::time::Duration::from_secs(60)));
        lock.enabled = false;
        assert!(!lock.due(now + std::time::Duration::from_secs(600)));
    }

    #[test]
    fn a_locked_window_announces_a_message_without_who_or_what() {
        let mut model = model();
        on_client_event(lock_on(0), &mut model, "");
        let (title, body) = notification(&model, "bob", &row("secret plans", false)).unwrap();
        assert_eq!(title, "Konstruct");
        assert!(!body.contains("secret"));
    }

    #[test]
    fn a_link_offer_reaches_the_page_as_an_image_only() {
        let mut model = model();
        let secret = "ab".repeat(32);
        let code = format!(
            "konstruct://link?token=tok&fp={}&s={secret}",
            "cd".repeat(32)
        );
        let events = on_client_event(
            ClientEvent::LinkOffer {
                code: Zeroizing::new(code),
                expires_at: 100,
            },
            &mut model,
            "",
        );
        let json = serde_json::to_string(&events).unwrap();
        assert!(json.contains("data:image/svg+xml"), "the code is drawn");
        assert!(
            !json.contains(&secret),
            "its secret is not in the page's text"
        );
        assert!(json.contains("100000"), "the expiry in ms");
        assert!(
            snapshot(&model)
                .iter()
                .all(|e| !matches!(e, ViewEvent::LinkOffer { .. })),
            "and not kept for a reload"
        );
    }

    #[test]
    fn an_imported_history_file_copy_is_deleted_once_read() {
        let mut model = model();
        let copy = std::env::temp_dir().join(format!("konstruct-import-test-{}.cthf", now_ms()));
        std::fs::write(&copy, b"sealed").unwrap();
        model.importing = Some(copy.clone());
        let summary = construct_client::history::HistorySummary {
            messages: 8,
            media: 3,
            ..Default::default()
        };
        let outs = on_client_event(ClientEvent::HistoryImported(Ok(summary)), &mut model, "");
        assert!(!copy.exists(), "the copy is gone");
        assert!(matches!(
            outs.as_slice(),
            [ViewEvent::HistoryTransfer {
                exported: false,
                error: None,
                messages: 8,
                media: 3,
                ..
            }]
        ));
    }

    #[test]
    fn history_received_over_the_network_ends_the_transfer_and_a_cancelled_one_says_nothing() {
        use construct_client::history::{HistorySummary, NearbyStep};
        let mut model = model();
        model.nearby = Some(false);
        let outs = on_client_event(
            ClientEvent::HistoryNearby(NearbyStep::Moving),
            &mut model,
            "",
        );
        assert!(matches!(
            outs.as_slice(),
            [ViewEvent::HistoryNearby {
                sending: false,
                state: "moving",
                ..
            }]
        ));
        let summary = HistorySummary {
            messages: 8,
            ..Default::default()
        };
        let outs = on_client_event(ClientEvent::HistoryImported(Ok(summary)), &mut model, "");
        assert!(
            matches!(
                outs.as_slice(),
                [
                    ViewEvent::HistoryTransfer { messages: 8, .. },
                    ViewEvent::HistoryNearby {
                        sending: false,
                        state: "done",
                        ..
                    }
                ]
            ),
            "the import is shown and the transfer ends"
        );
        assert_eq!(model.nearby, None);
        let outs = on_client_event(
            ClientEvent::HistoryNearby(NearbyStep::Failed("cancelled".into())),
            &mut model,
            "",
        );
        assert!(outs.is_empty(), "nothing is under way: nothing to show");
    }

    #[test]
    fn a_history_file_does_not_overwrite_one_already_there() {
        let dir = std::env::temp_dir().join(format!("konstruct-free-{}", now_ms()));
        let first = free_path(&dir, "konstruct-history-ab", "cthf").unwrap();
        std::fs::write(&first, b"1").unwrap();
        let second = free_path(&dir, "konstruct-history-ab", "cthf").unwrap();
        assert_ne!(first, second);
        assert!(second.ends_with("konstruct-history-ab (1).cthf"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_fetch_choice_goes_to_the_page_and_back_unchanged() {
        for p in [
            AutoDownload::Never,
            AutoDownload::Small,
            AutoDownload::Always,
        ] {
            assert_eq!(auto_download(auto_download_name(p)), p);
        }
        assert_eq!(
            auto_download("anything"),
            AutoDownload::Small,
            "the default"
        );
    }

    #[test]
    fn a_refused_deletion_reaches_the_page() {
        let mut model = model();
        let events = on_client_event(
            ClientEvent::DeleteAccountFailed("grpc 14: unavailable".into()),
            &mut model,
            "",
        );
        assert!(matches!(
            events.as_slice(),
            [ViewEvent::DeleteFailed { reason }] if reason.contains("unavailable")
        ));
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

    fn row(text: &str, sent: bool) -> MessageRow {
        MessageRow {
            id: "m".into(),
            text: text.into(),
            sent,
            time: 1,
            delivered: false,
            edited: false,
            quote: None,
            attachment: None,
            reactions: Vec::new(),
            media: Vec::new(),
        }
    }

    #[test]
    fn a_notification_says_what_the_person_chose() {
        let mut model = model();
        model.chat_mut("bob").name = "Bob".into();
        model.focused = false;
        let says = |model: &Model| notification(model, "bob", &row("hi there", false));
        assert_eq!(says(&model), Some(("Bob".into(), "hi there".into())));
        model.notify.preview = "nameOnly".into();
        assert_eq!(says(&model), Some(("Bob".into(), "New message".into())));
        model.notify.preview = "none".into();
        assert_eq!(
            says(&model),
            Some(("Konstruct".into(), "New message".into()))
        );
        model.notify.enabled = false;
        assert_eq!(says(&model), None);
    }

    #[test]
    fn a_message_being_looked_at_or_our_own_is_not_announced() {
        let mut model = model();
        model.open_chat = Some("bob".into());
        model.focused = true;
        assert_eq!(
            notification(&model, "bob", &row("hi", false)),
            None,
            "on screen"
        );
        assert!(
            notification(&model, "carol", &row("hi", false)).is_some(),
            "another chat"
        );
        model.focused = false;
        assert!(
            notification(&model, "bob", &row("hi", false)).is_some(),
            "window away"
        );
        assert_eq!(
            notification(&model, "bob", &row("hi", true)),
            None,
            "our own"
        );
    }

    #[test]
    fn a_picture_reaches_the_page_only_as_an_image_without_its_padding() {
        use base64::Engine;
        let mut padded = vec![0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 0xFF, 0xD9];
        padded.resize(1000, 0);
        let uri = avatar_uri(&padded).unwrap();
        let body = uri.strip_prefix("data:image/jpeg;base64,").unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(body)
            .unwrap();
        assert_eq!(bytes, &padded[..8]);
        let mut sound = b"\0\0\0\x20ftypM4A ".to_vec();
        sound.resize(64, 1);
        assert_eq!(avatar_uri(&sound), None, "a sound is no picture");
        assert_eq!(avatar_uri(b"<svg onload=x>"), None);
    }
}
