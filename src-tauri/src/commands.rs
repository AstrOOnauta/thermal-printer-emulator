//! The webview's commands (`conventions/bridge.md`), and the listener restart they share
//! with setup. Thin: the emulator's logic lives in `listener`, `receipts` and `settings`.

use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt as _;

use crate::listener::{self, lock, Limits, ListenerStatus, Shared};
use crate::locale::Locale;
use crate::receipts::{ReceiptSummary, ReceiptView};
use crate::settings::{self, Settings};
use crate::shell::{refresh_menus, PRODUCT_NAME};

/// A refused command: an i18n key the webview translates. Rust never sends a sentence.
#[derive(Debug, Serialize)]
pub struct UiError {
    key: &'static str,
}

impl UiError {
    fn new(key: &'static str) -> Self {
        Self { key }
    }
}

/// The running listener task, replaced when the port or network changes. An async mutex,
/// held for a whole restart: two restarts at once would otherwise both spawn a listener and
/// lose the handle of one, which then holds the port for good.
#[derive(Default)]
pub struct ListenerTask(tokio::sync::Mutex<Option<JoinHandle<()>>>);

/// The settings file. Held while a change is saved and applied, so two changes never
/// interleave (file and memory would disagree).
pub struct SettingsPath(pub tokio::sync::Mutex<PathBuf>);

/// Starts the listener on the address in the settings, after stopping the previous one:
/// awaiting the aborted task means its socket is closed before the new bind.
pub async fn restart_listener(app: &AppHandle) {
    let task = app.state::<ListenerTask>();
    let mut task = task.0.lock().await;
    if let Some(previous) = task.take() {
        previous.abort();
        let _ = previous.await;
    }
    let shared = Arc::clone(&app.state::<Arc<Shared>>());
    let addr = lock(&shared.settings).addr();
    *task = Some(tauri::async_runtime::spawn(listener::run(
        addr,
        Limits::PRODUCTION,
        shared,
    )));
}

/// Builds the test receipt and sends it to our own listener, like a POS would.
#[tauri::command]
pub async fn print_test_receipt(app: AppHandle) -> Result<(), UiError> {
    send_test_receipt(&app).await
}

pub(crate) async fn send_test_receipt(app: &AppHandle) -> Result<(), UiError> {
    let (port, bytes) = {
        let shared = app.state::<Arc<Shared>>();
        let ListenerStatus::Listening { port } = *lock(&shared.status) else {
            return Err(UiError::new("testReceipt.errors.notListening"));
        };
        let bytes =
            crate::test_receipt::build(&lock(&shared.settings), Locale::current().strings());
        (port, bytes)
    };
    crate::test_receipt::send(port, &bytes)
        .await
        .map_err(|error| {
            log::warn!("test_receipt_failed port={port} error={error}");
            UiError::new("testReceipt.errors.send")
        })
}

/// Drops every finished receipt from memory.
#[tauri::command]
pub fn clear_receipts(app: AppHandle) {
    let shared = app.state::<Arc<Shared>>();
    lock(&shared.receipts).clear();
    shared.changed();
    log::info!("receipts_cleared");
}

/// Saves a receipt's raw bytes in the Downloads folder and shows the file. Returns its name.
#[tauri::command]
pub fn export_receipt(app: AppHandle, id: u64) -> Result<String, UiError> {
    let (started_at, bytes) = {
        let shared = app.state::<Arc<Shared>>();
        let receipts = lock(&shared.receipts);
        let view = receipts
            .view(id)
            .ok_or_else(|| UiError::new("receipts.errors.gone"))?;
        let bytes = receipts.raw(id).unwrap_or_default().to_vec();
        (view.summary.started_at, bytes)
    };
    let name = format!("receipt-{started_at}-{id}.bin");
    let path = app
        .path()
        .download_dir()
        .map(|dir| dir.join(&name))
        .map_err(|error| {
            log::error!("export_failed id={id} error={error}");
            UiError::new("receipts.errors.export")
        })?;
    std::fs::write(&path, &bytes).map_err(|error| {
        log::error!(
            "export_failed id={id} path={} error={error}",
            path.display()
        );
        UiError::new("receipts.errors.export")
    })?;
    log::info!("receipt_exported id={id} bytes={}", bytes.len());
    if let Err(error) = app.opener().reveal_item_in_dir(&path) {
        log::warn!("reveal_failed path={} error={error}", path.display());
    }
    Ok(name)
}

/// A receipt's commands, re-parsed from its raw bytes (`escpos::inspect`). `None` once
/// dropped from memory. The bytes are copied out so parsing never holds the lock.
#[tauri::command]
pub fn get_receipt_commands(
    shared: State<'_, Arc<Shared>>,
    id: u64,
) -> Option<crate::escpos::inspect::Inspection> {
    let raw = lock(&shared.receipts).raw(id)?.to_vec();
    let code_page = lock(&shared.settings).code_page();
    Some(crate::escpos::inspect::inspect(&raw, code_page))
}

/// Receipts that finished while the window was not in front: a badge on the Dock icon
/// (macOS, some Linux docks), the count next to the menu bar icon (macOS: the Dock icon is
/// hidden while the window is closed), and the tray tooltip everywhere. 0 clears them.
#[tauri::command]
pub fn set_unseen(app: AppHandle, count: u32) {
    let badge = (count > 0).then_some(i64::from(count));
    if let Some(window) = app.get_webview_window("main") {
        // Unsupported on Windows (it has no badge count): nothing to do there.
        let _ = window.set_badge_count(badge);
    }
    if let Some(tray) = app.tray_by_id("main") {
        #[cfg(target_os = "macos")]
        let _ = tray.set_title(badge.map(|count| count.to_string()));
        let tooltip = match count {
            0 => PRODUCT_NAME.to_owned(),
            _ => {
                let unseen = Locale::current()
                    .strings()
                    .unseen
                    .replace("{count}", &count.to_string());
                format!("{PRODUCT_NAME} · {unseen}")
            }
        };
        if let Err(error) = tray.set_tooltip(Some(tooltip)) {
            log::warn!("tray tooltip failed: {error}");
        }
    }
}

/// This computer's LAN IPv4 address (`network.rs`), `None` offline.
#[tauri::command]
pub fn get_lan_address() -> Option<String> {
    crate::network::lan_ipv4().map(|ip| ip.to_string())
}

#[tauri::command]
pub fn get_settings(shared: State<'_, Arc<Shared>>) -> Settings {
    lock(&shared.settings).clone()
}

/// Validates, saves and applies new settings. A new port or network restarts the listener;
/// paper and code page apply to the next connections.
#[tauri::command]
pub async fn set_settings(app: AppHandle, settings: Settings) -> Result<Settings, UiError> {
    settings
        .validate()
        .map_err(|invalid| UiError::new(invalid.key()))?;
    let path = app.state::<SettingsPath>();
    let path = path.0.lock().await;
    // A file write and a disk sync: on a blocking thread, not the async worker.
    let saved = {
        let (settings, path) = (settings.clone(), path.clone());
        tauri::async_runtime::spawn_blocking(move || settings.save(&path))
            .await
            .map_err(std::io::Error::other)
            .and_then(|result| result)
    };
    if let Err(error) = saved {
        log::error!("settings_save_failed path={} error={error}", path.display());
        return Err(UiError::new("settings.errors.save"));
    }
    let (restart, language_changed) = {
        let shared = app.state::<Arc<Shared>>();
        let mut current = lock(&shared.settings);
        let changes = (
            current.addr() != settings.addr(),
            current.language != settings.language,
        );
        *current = settings.clone();
        changes
    };
    if language_changed {
        Locale::prefer(settings.language);
        if let Err(error) = refresh_menus(&app) {
            log::error!("menus_refresh_failed error={error}");
        }
    }
    log::info!(
        "settings_changed port={} bind={:?} paper={:?} code_page={} sound={} language={:?}",
        settings.port,
        settings.bind,
        settings.paper,
        settings.code_page,
        settings.sound,
        settings.language
    );
    if restart {
        restart_listener(&app).await;
    }
    Ok(settings)
}

/// The settings file path in the app's config folder.
pub fn settings_path(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join(settings::FILE_NAME))
}

/// The webview's UI language (`en`, `es` or `pt-BR`).
#[tauri::command]
pub fn app_locale() -> &'static str {
    Locale::current().tag()
}

/// Printed receipts, oldest first. Same list as the `receipts` event.
#[tauri::command]
pub fn get_receipts(shared: State<'_, Arc<Shared>>) -> Vec<ReceiptSummary> {
    listener::lock(&shared.receipts).summaries()
}

/// One receipt with its print model, to draw. `None` once it was dropped from memory.
#[tauri::command]
pub fn get_receipt(id: u64, shared: State<'_, Arc<Shared>>) -> Option<ReceiptView> {
    listener::lock(&shared.receipts).view(id)
}

/// Whether the emulator is listening, and on which port.
#[tauri::command]
pub fn get_listener_status(shared: State<'_, Arc<Shared>>) -> ListenerStatus {
    listener::lock(&shared.status).clone()
}
