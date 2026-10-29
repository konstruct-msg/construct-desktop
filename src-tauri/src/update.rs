//! Updates (construct-docs decisions/desktop-updates-tauri-github-releases.md), as the macOS
//! Desktop has them through Sparkle: once at start and once a day the app reads a feed on GitHub
//! Releases and offers a version newer than its own; it installs only a file whose minisign
//! signature checks against the key built into it (`plugins.updater.pubkey`).
//!
//! An AppImage and the Windows installers update themselves. A `.deb` or `.rpm` copy belongs to
//! the system's package manager: the app says that a version is out and how to install it, and
//! never asks for the root password itself. A copy not installed from a package (`cargo run`)
//! only says it.
//!
//! The plugin is used from Rust alone; the page is granted none of its commands.

use std::sync::Mutex;
use std::time::Duration;

use tauri::utils::config::BundleType;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::Intents;
use crate::bridge::{Intent, ViewEvent};

/// After the start, so the first check does not compete with signing in.
const FIRST_CHECK: Duration = Duration::from_secs(30);
const EVERY: Duration = Duration::from_secs(24 * 60 * 60);

/// Where a version's packages are, for a copy the package manager updates.
const RELEASES: &str = "https://github.com/konstruct-msg/construct-desktop/releases/tag/v";

/// How this copy gets a new version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Install {
    /// It replaces itself: an AppImage, the NSIS or MSI installer.
    Itself,
    Deb,
    Rpm,
    /// Not installed from a package.
    Manual,
}

impl Install {
    pub fn of(bundle: Option<BundleType>) -> Self {
        match bundle {
            Some(BundleType::AppImage | BundleType::Nsis | BundleType::Msi) => Self::Itself,
            Some(BundleType::Deb) => Self::Deb,
            Some(BundleType::Rpm) => Self::Rpm,
            _ => Self::Manual,
        }
    }

    fn here() -> Self {
        Self::of(tauri::utils::platform::bundle_type())
    }

    fn name(self) -> &'static str {
        match self {
            Self::Itself => "itself",
            Self::Deb => "deb",
            Self::Rpm => "rpm",
            Self::Manual => "manual",
        }
    }
}

/// The update found and not installed yet.
#[derive(Default)]
pub struct Updates(Mutex<Option<Update>>);

fn view(state: &'static str, version: Option<String>, error: Option<String>) -> ViewEvent {
    ViewEvent::Update {
        state,
        version,
        install: Install::here().name(),
        error,
    }
}

fn show(app: &AppHandle, event: ViewEvent) {
    app.state::<Intents>().send(Intent::Update(event));
}

/// Check now, and once a day while the app runs.
pub async fn run(app: AppHandle) {
    tokio::time::sleep(FIRST_CHECK).await;
    loop {
        check(&app).await;
        tokio::time::sleep(EVERY).await;
    }
}

/// Read the feed; show what it says.
pub async fn check(app: &AppHandle) {
    let found = async { app.updater()?.check().await }.await;
    let event = match found {
        Ok(Some(update)) => {
            tracing::info!(version = %update.version, "update available");
            let version = update.version.clone();
            *app.state::<Updates>()
                .0
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(update);
            view("available", Some(version), None)
        }
        Ok(None) => {
            // A feed that went back (a release withdrawn) withdraws what it offered.
            *app.state::<Updates>()
                .0
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            view("current", None, None)
        }
        Err(e) => {
            tracing::warn!(error = %e, "update check failed");
            view("failed", None, Some(e.to_string()))
        }
    };
    show(app, event);
}

/// Download the update found, check its signature, install it and start again. Only for a copy
/// that replaces itself; on Windows the installer ends this process.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    if Install::here() != Install::Itself {
        return Err("this copy is updated by its package manager".into());
    }
    let update = app
        .state::<Updates>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
        .ok_or("no update to install")?;
    let version = update.version.clone();
    show(app, view("installing", Some(version.clone()), None));
    match update.download_and_install(|_, _| {}, || {}).await {
        Ok(()) => {
            tracing::info!(%version, "update installed; restarting");
            app.restart()
        }
        Err(e) => {
            tracing::warn!(error = %e, "update not installed");
            show(app, view("failed", Some(version), Some(e.to_string())));
            Err(e.to_string())
        }
    }
}

/// The release page of the update found, for a copy the package manager updates.
pub fn release_page(app: &AppHandle) -> Option<String> {
    let held = app.state::<Updates>();
    let held = held.0.lock().unwrap_or_else(|e| e.into_inner());
    // The plugin parsed the version as semver: digits, dots, letters and hyphens.
    held.as_ref().map(|u| format!("{RELEASES}{}", u.version))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only an AppImage and the Windows installers replace themselves: a `.deb` or `.rpm` would
    /// take root, and a copy run from a build has nothing to replace.
    #[test]
    fn only_self_contained_copies_install_themselves() {
        for bundle in [BundleType::AppImage, BundleType::Nsis, BundleType::Msi] {
            let name = format!("{bundle:?}");
            assert_eq!(Install::of(Some(bundle)), Install::Itself, "{name}");
        }
        assert_eq!(Install::of(Some(BundleType::Deb)), Install::Deb);
        assert_eq!(Install::of(Some(BundleType::Rpm)), Install::Rpm);
        assert_eq!(Install::of(None), Install::Manual);
    }
}
