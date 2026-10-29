//! Konstruct for Linux and Windows: a Tauri window. The client runs in Rust (`bridge`); the
//! webview draws typed view events and sends back what the person did. No key, token or protocol
//! state reaches it (decisions/desktop-web-interface.md).

// No console window behind the app on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bridge;
mod logs;

use bridge::{Intent, ViewEvent};
use tauri::{Manager, State, ipc::Channel};
use tokio::sync::mpsc;
use zeroize::Zeroizing;

struct Intents(mpsc::UnboundedSender<Intent>);

impl Intents {
    fn send(&self, intent: Intent) {
        let _ = self.0.send(intent);
    }
}

/// The page is up (or was reloaded): send it everything to draw, then each change.
#[tauri::command]
fn attach(on_event: Channel<ViewEvent>, intents: State<Intents>) {
    intents.send(Intent::Attach(on_event));
}

#[tauri::command]
fn register(username: String, intents: State<Intents>) {
    intents.send(Intent::Register(username));
}

#[tauri::command]
fn start_link(intents: State<Intents>) {
    intents.send(Intent::StartLink);
}

#[tauri::command]
fn cancel_link(intents: State<Intents>) {
    intents.send(Intent::CancelLink);
}

#[tauri::command]
fn unlock(passphrase: String, intents: State<Intents>) {
    intents.send(Intent::Unlock(Zeroizing::new(passphrase.into_bytes())));
}

#[tauri::command]
fn set_passphrase(passphrase: String, intents: State<Intents>) {
    intents.send(Intent::SetPassphrase(Zeroizing::new(
        passphrase.into_bytes(),
    )));
}

#[tauri::command]
fn open_chat(peer_id: String, intents: State<Intents>) {
    intents.send(Intent::OpenChat(peer_id));
}

#[tauri::command]
fn send_message(peer_id: String, text: String, intents: State<Intents>) {
    intents.send(Intent::Send { peer_id, text });
}

#[tauri::command]
fn reply(peer_id: String, text: String, to: String, intents: State<Intents>) {
    intents.send(Intent::Reply { peer_id, text, to });
}

#[tauri::command]
fn edit_message(message_id: String, text: String, intents: State<Intents>) {
    intents.send(Intent::Edit { message_id, text });
}

#[tauri::command]
fn react(message_id: String, emoji: String, intents: State<Intents>) {
    intents.send(Intent::React { message_id, emoji });
}

#[tauri::command]
fn delete_message(message_id: String, intents: State<Intents>) {
    intents.send(Intent::DeleteMessage(message_id));
}

/// A file the person chose: its bytes as the body, the chat, its name and caption as
/// percent-encoded headers.
#[tauri::command]
fn send_file(request: tauri::ipc::Request<'_>, intents: State<Intents>) -> Result<(), String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("no file".into());
    };
    let header = |name: &str| {
        request
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(percent_decode)
            .unwrap_or_default()
    };
    let peer_id = header("x-peer");
    if peer_id.is_empty() {
        return Err("no chat".into());
    }
    intents.send(Intent::SendFile {
        peer_id,
        name: header("x-name"),
        bytes: bytes.clone(),
        caption: header("x-caption"),
    });
    Ok(())
}

/// Write this device's history for `device_id`, another device of the account, into the
/// downloads folder.
#[tauri::command]
fn export_history(device_id: String, intents: State<Intents>) {
    intents.send(Intent::ExportHistory(device_id));
}

/// A history file the person chose: its bytes as the body.
#[tauri::command]
fn import_history(request: tauri::ipc::Request<'_>, intents: State<Intents>) -> Result<(), String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("no file".into());
    };
    intents.send(Intent::ImportHistory(bytes.clone()));
    Ok(())
}

/// Send this device's history to `device_id` over the local network.
#[tauri::command]
fn offer_history_nearby(device_id: String, intents: State<Intents>) {
    intents.send(Intent::OfferHistoryNearby(device_id));
}

#[tauri::command]
fn cancel_history_nearby(intents: State<Intents>) {
    intents.send(Intent::CancelHistoryNearby);
}

/// `encodeURIComponent` undone. Bytes that do not decode are kept as they came.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        match (bytes[i], bytes.get(i + 1), bytes.get(i + 2)) {
            (b'%', Some(&h), Some(&l)) if hex(h).is_some() && hex(l).is_some() => {
                out.push((hex(h).unwrap() * 16 + hex(l).unwrap()) as u8);
                i += 3;
            }
            (b, ..) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[tauri::command]
fn set_focus(focused: bool, intents: State<Intents>) {
    intents.send(Intent::Focus(focused));
}

#[tauri::command]
fn set_notifications(settings: bridge::Notifications, intents: State<Intents>) {
    intents.send(Intent::Notifications(settings));
}

#[tauri::command]
fn load_settings(intents: State<Intents>) {
    intents.send(Intent::LoadSettings);
}

#[tauri::command]
fn change_username(username: String, intents: State<Intents>) {
    intents.send(Intent::ChangeUsername(username));
}

#[tauri::command]
fn set_discoverable(on: bool, intents: State<Intents>) {
    intents.send(Intent::SetDiscoverable(on));
}

#[tauri::command]
fn set_delivery_receipts(on: bool, intents: State<Intents>) {
    intents.send(Intent::SetDeliveryReceipts(on));
}

/// Sign out device `device_id` of this account, or every other one when it is absent.
#[tauri::command]
fn revoke_devices(device_id: Option<String>, intents: State<Intents>) {
    intents.send(Intent::RevokeDevices(device_id));
}

/// Our shared name; the picture stays.
#[tauri::command]
fn set_display_name(name: String, intents: State<Intents>) {
    intents.send(Intent::SetProfile {
        display_name: Some(name),
        avatar: construct_client::AvatarChange::Keep,
    });
}

/// Our picture: the JPEG the page made from the file chosen, as the body. The client checks it.
#[tauri::command]
fn set_avatar(request: tauri::ipc::Request<'_>, intents: State<Intents>) -> Result<(), String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("no picture".into());
    };
    intents.send(Intent::SetProfile {
        display_name: None,
        avatar: construct_client::AvatarChange::Set(bytes.clone()),
    });
    Ok(())
}

#[tauri::command]
fn remove_avatar(intents: State<Intents>) {
    intents.send(Intent::SetProfile {
        display_name: None,
        avatar: construct_client::AvatarChange::Remove,
    });
}

/// Our own name for a contact; empty removes it.
#[tauri::command]
fn set_alias(peer_id: String, alias: String, intents: State<Intents>) {
    let alias = (!alias.trim().is_empty()).then_some(alias);
    intents.send(Intent::SetAlias { peer_id, alias });
}

#[tauri::command]
fn rename_device(name: String, intents: State<Intents>) {
    intents.send(Intent::RenameDevice(name));
}

#[tauri::command]
fn share_profile(peer_id: String, on: bool, intents: State<Intents>) {
    intents.send(Intent::ShareProfile { peer_id, on });
}

#[tauri::command]
fn set_veil(on: bool, intents: State<Intents>) {
    intents.send(Intent::SetVeil(on));
}

#[tauri::command]
fn import_veil_link(link: String, intents: State<Intents>) {
    intents.send(Intent::ImportVeilLink(link));
}

#[tauri::command]
fn load_veil(intents: State<Intents>) {
    intents.send(Intent::LoadVeil);
}

#[tauri::command]
fn issue_veil_voucher(intents: State<Intents>) {
    intents.send(Intent::IssueVeilVoucher);
}

#[tauri::command]
fn load_stickers(intents: State<Intents>) {
    intents.send(Intent::LoadStickers);
}

#[tauri::command]
fn load_sticker(pack_id: String, index: u32, emoji: String, intents: State<Intents>) {
    intents.send(Intent::LoadSticker(
        construct_client::stickers::StickerRef {
            pack_id,
            index,
            emoji,
        },
    ));
}

#[tauri::command]
fn send_sticker(peer_id: String, pack_id: String, index: u32, intents: State<Intents>) {
    intents.send(Intent::SendSticker {
        peer_id,
        pack_id,
        index,
    });
}

#[tauri::command]
fn load_sticker_catalog(intents: State<Intents>) {
    intents.send(Intent::LoadStickerCatalog);
}

#[tauri::command]
fn add_sticker_pack(pack_id: String, intents: State<Intents>) {
    intents.send(Intent::AddStickerPack(pack_id));
}

#[tauri::command]
fn remove_sticker_pack(pack_id: String, intents: State<Intents>) {
    intents.send(Intent::RemoveStickerPack(pack_id));
}

#[tauri::command]
fn load_media(message_id: String, index: usize, intents: State<Intents>) {
    intents.send(Intent::LoadMedia { message_id, index });
}

#[tauri::command]
fn save_media(message_id: String, index: usize, intents: State<Intents>) {
    intents.send(Intent::SaveMedia { message_id, index });
}

#[tauri::command]
fn find_contact(query: String, intents: State<Intents>) {
    intents.send(Intent::Find(query));
}

#[tauri::command]
fn add_contact(user_id: String, name: String, intents: State<Intents>) {
    intents.send(Intent::AddContact { user_id, name });
}

#[tauri::command]
fn delete_contact(peer_id: String, intents: State<Intents>) {
    intents.send(Intent::DeleteContact(peer_id));
}

#[tauri::command]
fn safety_number(peer_id: String, intents: State<Intents>) {
    intents.send(Intent::SafetyNumber(peer_id));
}

#[tauri::command]
fn mint_invite(intents: State<Intents>) {
    intents.send(Intent::MintInvite);
}

#[tauri::command]
fn logout(intents: State<Intents>) {
    intents.send(Intent::Logout);
}

/// Sign out every device of the account, this one wiped too.
#[tauri::command]
fn logout_everywhere(intents: State<Intents>) {
    intents.send(Intent::LogoutEverywhere);
}

/// Close the window now, if the app lock is on.
#[tauri::command]
fn lock_now(intents: State<Intents>) {
    intents.send(Intent::LockNow);
}

/// A code typed at the lock screen; the client checks it.
#[tauri::command]
fn unlock_app(code: String, intents: State<Intents>) {
    intents.send(Intent::UnlockApp(Zeroizing::new(code)));
}

/// Set, change or remove the app lock's code, or change its delay.
#[tauri::command]
fn set_app_lock(
    current: Option<String>,
    code: Option<String>,
    remove: bool,
    delay_secs: u32,
    intents: State<Intents>,
) {
    intents.send(Intent::SetAppLock {
        current: current.map(Zeroizing::new),
        code: code.map(Zeroizing::new),
        remove,
        delay_secs,
    });
}

/// Burn invites this device handed out.
#[tauri::command]
fn revoke_invites(jtis: Vec<String>, intents: State<Intents>) {
    intents.send(Intent::RevokeInvites(jtis));
}

/// Show a code a new device scans to join this account.
#[tauri::command]
fn offer_link(intents: State<Intents>) {
    intents.send(Intent::OfferLink);
}

#[tauri::command]
fn cancel_link_offer(intents: State<Intents>) {
    intents.send(Intent::CancelLinkOffer);
}

/// Let the device whose `konstruct://link-to-me` code was pasted join this account.
#[tauri::command]
fn approve_device(code: String, intents: State<Intents>) {
    intents.send(Intent::ApproveDevice(code));
}

/// Delete the account and wipe this device; `local_only`: wipe only.
#[tauri::command]
fn delete_account(local_only: bool, intents: State<Intents>) {
    intents.send(Intent::DeleteAccount { local_only });
}

#[tauri::command]
fn clear_media_cache(intents: State<Intents>) {
    intents.send(Intent::ClearMediaCache);
}

/// How attachments are fetched and kept: `never`, `small` or `always`; a limit in bytes and an
/// age in days, absent for none.
#[tauri::command]
fn set_media_prefs(
    auto_download: String,
    cache_limit: Option<u64>,
    keep_days: Option<u32>,
    intents: State<Intents>,
) {
    intents.send(Intent::SetMediaPrefs(
        construct_client::local_store::MediaPrefs {
            auto_download: bridge::auto_download(&auto_download),
            quota: cache_limit.filter(|b| *b > 0),
            keep_days: keep_days.filter(|d| *d > 0),
        },
    ));
}

/// Show the recovery phrase that waits for its copy: the one secret the window is ever given,
/// because the person has to read it.
#[tauri::command]
fn reveal_recovery_phrase(intents: State<Intents>) {
    intents.send(Intent::RevealRecoveryPhrase);
}

/// Words of the phrase by position, as the person typed them back; the client checks them.
#[tauri::command]
fn confirm_recovery_copy(answers: Vec<(usize, String)>, intents: State<Intents>) {
    intents.send(Intent::ConfirmRecoveryCopy(answers));
}

#[tauri::command]
fn enter_recovery_phrase(phrase: String, intents: State<Intents>) {
    intents.send(Intent::EnterRecoveryPhrase(Zeroizing::new(phrase)));
}

/// Where the logs are written.
struct LogDir(std::path::PathBuf);

/// Where saved attachments go.
struct Downloads(std::path::PathBuf);

/// Open the log folder in the system's file manager.
#[tauri::command]
fn open_logs(dir: State<LogDir>) -> Result<(), String> {
    open_folder(&dir.0)
}

/// Open the folder saved attachments go to. The folder, never a file in it: a file from the
/// network is not handed to whatever the system would run it with.
#[tauri::command]
fn open_downloads(dir: State<Downloads>) -> Result<(), String> {
    open_folder(&dir.0)
}

fn open_folder(dir: &std::path::Path) -> Result<(), String> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(dir)
        .spawn()
        .map(drop)
        .map_err(|e| format!("{opener}: {e}"))
}

/// Settings → Diagnostics: where the logs are and how much they take.
#[derive(serde::Serialize)]
struct LogsInfo {
    path: String,
    bytes: u64,
}

#[tauri::command]
async fn logs_info(dir: State<'_, LogDir>) -> Result<LogsInfo, String> {
    let dir = dir.0.clone();
    blocking(move || {
        Ok(LogsInfo {
            bytes: logs::size(&dir),
            path: dir.display().to_string(),
        })
    })
    .await
}

/// The last lines written, to read without opening the folder.
#[tauri::command]
async fn logs_recent(dir: State<'_, LogDir>) -> Result<String, String> {
    let dir = dir.0.clone();
    blocking(move || Ok(logs::recent(&dir, 200))).await
}

/// All the logs in one file in the downloads folder, to send; its name.
#[tauri::command]
async fn save_logs(
    dir: State<'_, LogDir>,
    downloads: State<'_, Downloads>,
) -> Result<String, String> {
    let (dir, out) = (dir.0.clone(), downloads.0.clone());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let path = blocking(move || logs::bundle(&dir, &out, now)).await?;
    tracing::info!("logs saved to the downloads folder");
    Ok(path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default())
}

#[tauri::command]
async fn clear_logs(dir: State<'_, LogDir>) -> Result<(), String> {
    let dir = dir.0.clone();
    blocking(move || logs::clear(&dir)).await?;
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "logs cleared");
    Ok(())
}

/// File work off the window's thread.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| e.to_string())?
}

/// A log writer that keeps VEIL fronts and IP addresses out of what it writes: in a release
/// build they become labels keyed to this device (construct-docs
/// decisions/veil-front-in-logs-is-a-keyed-label). Logs are what a person sends when something
/// is wrong; a front's address in them is one more address to block.
struct Redacted<M>(M);

impl<'a, M: tracing_subscriber::fmt::MakeWriter<'a>> tracing_subscriber::fmt::MakeWriter<'a>
    for Redacted<M>
{
    type Writer = RedactedWriter<M::Writer>;
    fn make_writer(&'a self) -> Self::Writer {
        RedactedWriter(self.0.make_writer())
    }
}

struct RedactedWriter<W>(W);

impl<W: std::io::Write> std::io::Write for RedactedWriter<W> {
    // The formatter hands over one whole event per write.
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let text = String::from_utf8_lossy(buf);
        self.0
            .write_all(construct_client::veil::redact::redact(&text).as_bytes())?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

/// Log to the terminal and to a file a day in `dir`, the last week kept: an app started from
/// the menu has no terminal, and the logs are what a person can send when something is wrong.
/// They stay on this computer; message text is never logged, fronts and addresses are labels.
fn init_logging(dir: &std::path::Path) {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};
    construct_client::veil::redact::init(dir);
    let filter = || {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info,h2=warn,rustls=warn"))
    };
    let file = std::fs::create_dir_all(dir)
        .map_err(|e| e.to_string())
        .and_then(|()| {
            tracing_appender::rolling::Builder::new()
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .filename_prefix("konstruct")
                .filename_suffix("log")
                .max_log_files(7)
                .build(dir)
                .map_err(|e| e.to_string())
        });
    let registry = tracing_subscriber::registry().with(
        fmt::layer()
            .with_writer(Redacted(std::io::stdout))
            .with_filter(filter()),
    );
    match file {
        Ok(appender) => registry
            .with(
                fmt::layer()
                    .with_ansi(false)
                    .with_writer(Redacted(appender))
                    .with_filter(filter()),
            )
            .init(),
        Err(e) => {
            registry.init();
            tracing::warn!(error = %e, dir = %dir.display(), "logs go to the terminal only");
        }
    }
}

fn main() {
    let (intents, inbox) = mpsc::unbounded_channel();
    let config = construct_client::config::load_config().unwrap_or_default();

    let builder = tauri::Builder::default();
    // WKWebView takes copy and paste only from an Edit menu; Linux and Windows need none, and a
    // menu there would be a menu bar in the window.
    #[cfg(target_os = "macos")]
    let builder = builder.menu(tauri::menu::Menu::default);
    builder
        .manage(Intents(intents))
        .setup(move |app| {
            let dir = app
                .path()
                .app_log_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("konstruct-logs"));
            init_logging(&dir);
            tracing::info!(version = env!("CARGO_PKG_VERSION"), "Konstruct starting");
            app.manage(LogDir(dir));
            let downloads = app
                .path()
                .download_dir()
                .or_else(|_| app.path().home_dir())
                .unwrap_or_else(|_| std::env::temp_dir());
            app.manage(Downloads(downloads.clone()));
            tauri::async_runtime::spawn(bridge::run(
                construct_client::ClientConfig {
                    server_url: config.server,
                    transport: config.transport,
                    no_encrypt: false,
                    pq_active: cfg!(feature = "post-quantum"),
                },
                downloads,
                inbox,
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            attach,
            register,
            start_link,
            cancel_link,
            unlock,
            set_passphrase,
            open_chat,
            send_message,
            reply,
            edit_message,
            react,
            delete_message,
            send_file,
            set_focus,
            set_notifications,
            load_settings,
            change_username,
            set_discoverable,
            set_delivery_receipts,
            revoke_devices,
            set_display_name,
            set_avatar,
            remove_avatar,
            share_profile,
            set_alias,
            rename_device,
            load_media,
            set_veil,
            import_veil_link,
            load_veil,
            issue_veil_voucher,
            load_stickers,
            load_sticker,
            send_sticker,
            load_sticker_catalog,
            add_sticker_pack,
            remove_sticker_pack,
            save_media,
            open_downloads,
            find_contact,
            add_contact,
            delete_contact,
            safety_number,
            mint_invite,
            logout,
            logout_everywhere,
            offer_link,
            lock_now,
            export_history,
            import_history,
            offer_history_nearby,
            cancel_history_nearby,
            unlock_app,
            set_app_lock,
            revoke_invites,
            cancel_link_offer,
            approve_device,
            delete_account,
            clear_media_cache,
            set_media_prefs,
            reveal_recovery_phrase,
            confirm_recovery_copy,
            enter_recovery_phrase,
            open_logs,
            logs_info,
            logs_recent,
            save_logs,
            clear_logs,
        ])
        .run(tauri::generate_context!())
        .expect("the Tauri runtime failed to start");
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_file_name_comes_back_as_the_page_encoded_it() {
        assert_eq!(
            super::percent_decode("%D0%BE%D1%82%D1%87%D1%91%D1%82%201.pdf"),
            "отчёт 1.pdf"
        );
        assert_eq!(super::percent_decode("50%"), "50%", "a stray % is kept");
    }
}
