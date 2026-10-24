//! Konstruct for Linux and Windows: a Tauri window. The client runs in Rust (`bridge`); the
//! webview draws typed view events and sends back what the person did. No key, token or protocol
//! state reaches it (decisions/desktop-web-interface.md).

// No console window behind the app on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bridge;

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

#[tauri::command]
fn share_profile(peer_id: String, on: bool, intents: State<Intents>) {
    intents.send(Intent::ShareProfile { peer_id, on });
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

#[tauri::command]
fn logs_path(dir: State<LogDir>) -> String {
    dir.0.display().to_string()
}

/// Log to the terminal and to a file a day in `dir`, the last week kept: an app started from
/// the menu has no terminal, and the logs are what a person can send when something is wrong.
/// They stay on this computer; message text is never logged.
fn init_logging(dir: &std::path::Path) {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};
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
    let registry = tracing_subscriber::registry().with(fmt::layer().with_filter(filter()));
    match file {
        Ok(appender) => registry
            .with(
                fmt::layer()
                    .with_ansi(false)
                    .with_writer(appender)
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
            load_media,
            save_media,
            open_downloads,
            find_contact,
            add_contact,
            delete_contact,
            safety_number,
            mint_invite,
            logout,
            reveal_recovery_phrase,
            confirm_recovery_copy,
            enter_recovery_phrase,
            open_logs,
            logs_path,
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
