//! Automatic updates (decision 8 in `.ai/stack.md`): a check soon after launch, then once a
//! day, against `latest.json` of the newest GitHub release (`plugins.updater` in
//! tauri.conf.json; the plugin checks the signature with its public key). An update is
//! announced to the window and the tray, and installed only when the user asks.

use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::listener::lock;

/// First check after launch: out of the way of startup.
const FIRST_CHECK: Duration = Duration::from_secs(30);
/// How often the loop wakes up. A check runs once the last one is a day old by the wall
/// clock, so a laptop that slept through the day checks when it wakes.
const TICK: Duration = Duration::from_secs(60 * 60);
const CHECK_EVERY: Duration = Duration::from_secs(24 * 60 * 60);

/// What the window and the tray show.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UpdateInfo {
    pub version: String,
    /// False for a `.deb` or `.rpm`: the system package manager owns those files, so the
    /// window links to the release page instead of installing.
    pub installable: bool,
}

/// The update the last check found, kept to install it.
#[derive(Default)]
pub struct Available(pub Mutex<Option<Update>>);

/// On Linux only the AppImage replaces itself (its runtime sets `APPIMAGE`).
pub fn installable() -> bool {
    !cfg!(target_os = "linux") || std::env::var_os("APPIMAGE").is_some()
}

pub fn info(update: &Update) -> UpdateInfo {
    UpdateInfo {
        version: update.version.clone(),
        installable: installable(),
    }
}

/// The update found by the last check, if any.
pub fn available(app: &AppHandle) -> Option<UpdateInfo> {
    let available = app.try_state::<Available>()?;
    let update = lock(&available.0);
    update.as_ref().map(info)
}

/// Checks after launch and then daily, forever.
pub async fn watch(app: AppHandle) {
    tokio::time::sleep(FIRST_CHECK).await;
    let mut last: Option<SystemTime> = None;
    loop {
        if due(last, SystemTime::now()) {
            check(&app).await;
            last = Some(SystemTime::now());
        }
        tokio::time::sleep(TICK).await;
    }
}

/// Whether a check is due: never checked, or the last one is a day old. A clock moved
/// backwards (the last check "in the future") counts as due.
fn due(last: Option<SystemTime>, now: SystemTime) -> bool {
    last.is_none_or(|at| {
        now.duration_since(at)
            .map_or(true, |age| age >= CHECK_EVERY)
    })
}

async fn check(app: &AppHandle) {
    let found = match app.updater() {
        Ok(updater) => updater.check().await,
        Err(error) => Err(error),
    };
    match found {
        Ok(Some(update)) => {
            let info = info(&update);
            log::info!(
                "update_available version={} installable={}",
                info.version,
                info.installable
            );
            *lock(&app.state::<Available>().0) = Some(update);
            if let Err(error) = app.emit("update_available", &info) {
                log::error!("emit_failed event=update_available error={error}");
            }
            if let Err(error) = crate::shell::refresh_menus(app) {
                log::error!("menus_refresh_failed error={error}");
            }
        }
        Ok(None) => log::info!("update_none"),
        // Offline, GitHub down, no release yet: the next check tries again.
        Err(error) => log::warn!("update_check_failed error={error}"),
    }
}

/// Downloads the update, verifies its signature and installs it, then restarts into the
/// new version. On Windows the installer takes over and the app exits by itself.
pub async fn install(app: &AppHandle) -> Result<(), InstallError> {
    if !installable() {
        return Err(InstallError::NotInstallable);
    }
    let update = lock(&app.state::<Available>().0)
        .clone()
        .ok_or(InstallError::NoUpdate)?;
    log::info!("update_install version={}", update.version);
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(InstallError::Updater)?;
    app.restart();
}

#[derive(Debug)]
pub enum InstallError {
    NotInstallable,
    NoUpdate,
    Updater(tauri_plugin_updater::Error),
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInstallable => formatter.write_str("not installable (system package)"),
            Self::NoUpdate => formatter.write_str("no update available"),
            Self::Updater(error) => write!(formatter, "{error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_at_first_then_once_a_day() {
        let now = SystemTime::now();
        let hours = |count: u64| Duration::from_secs(count * 60 * 60);
        assert!(due(None, now), "never checked");
        assert!(!due(Some(now - hours(1)), now));
        assert!(!due(Some(now - hours(23)), now));
        assert!(due(Some(now - hours(24)), now));
        assert!(due(Some(now - hours(72)), now), "after a long sleep");
        assert!(due(Some(now + hours(2)), now), "clock moved backwards");
    }
}
