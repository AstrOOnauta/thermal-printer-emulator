//! UI bridge: tray, app menu, window show/hide and the webview's commands. No emulator
//! logic here.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;
#[cfg(target_os = "macos")]
use tauri::menu::Submenu;

use tauri::async_runtime::JoinHandle;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_opener::OpenerExt as _;
use tokio::sync::mpsc;

use crate::listener::{self, lock, BindError, Event, Limits, ListenerStatus, Shared};
use crate::locale::{Locale, Strings};
use crate::receipts::{ReceiptSummary, ReceiptView};
use crate::settings::{self, Settings};

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

/// Where the listener sends its events (the forwarder in `lib.rs` emits them).
pub struct Events(pub mpsc::UnboundedSender<Event>);

/// The running listener task, replaced when the port or network changes.
#[derive(Default)]
pub struct ListenerTask(Mutex<Option<JoinHandle<()>>>);

/// The settings file.
pub struct SettingsPath(pub PathBuf);

/// Starts the listener on the address in the settings, after stopping the previous one:
/// awaiting the aborted task means its socket is closed before the new bind.
pub async fn restart_listener(app: &AppHandle) {
    let previous = lock(&app.state::<ListenerTask>().0).take();
    if let Some(previous) = previous {
        previous.abort();
        let _ = previous.await;
    }
    let shared = Arc::clone(&app.state::<Arc<Shared>>());
    let addr = lock(&shared.settings).addr();
    let events = app.state::<Events>().0.clone();
    let task = tauri::async_runtime::spawn(listener::run(addr, Limits::PRODUCTION, shared, events));
    *lock(&app.state::<ListenerTask>().0) = Some(task);
}

/// Builds the test receipt and sends it to our own listener, like a POS would.
#[tauri::command]
pub async fn print_test_receipt(app: AppHandle) -> Result<(), UiError> {
    send_test_receipt(&app).await
}

async fn send_test_receipt(app: &AppHandle) -> Result<(), UiError> {
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
    let mut receipts = lock(&shared.receipts);
    receipts.clear();
    // Through the listener's channel, so it stays in order with the listener's events.
    let _ = app
        .state::<Events>()
        .0
        .send(Event::Receipts(receipts.summaries()));
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
    let path = app.state::<SettingsPath>().0.clone();
    if let Err(error) = settings.save(&path) {
        log::error!("settings_save_failed path={} error={error}", path.display());
        return Err(UiError::new("settings.errors.save"));
    }
    let restart = {
        let shared = app.state::<Arc<Shared>>();
        let mut current = lock(&shared.settings);
        let restart = current.addr() != settings.addr();
        *current = settings.clone();
        restart
    };
    log::info!(
        "settings_changed port={} bind={:?} paper={:?} code_page={} sound={}",
        settings.port,
        settings.bind,
        settings.paper,
        settings.code_page,
        settings.sound
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

const PRODUCT_NAME: &str = "Thermal Printer Emulator";

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

/// The tray's first, disabled item: the listener status line.
struct TrayStatus(MenuItem<tauri::Wry>);

/// Hands a listener event to the webview (and the tray, for the status).
pub fn forward(app: &AppHandle, event: Event) {
    let emitted = match event {
        Event::Receipts(receipts) => app.emit("receipts", receipts),
        Event::Status(status) => {
            if let Some(item) = app.try_state::<TrayStatus>() {
                let label = status_label(Locale::current().strings(), &status);
                if let Err(error) = item.0.set_text(label) {
                    log::error!("tray status failed: {error}");
                }
            }
            app.emit("listener_status", status)
        }
    };
    if let Err(error) = emitted {
        log::error!("emit failed: {error}");
    }
}

fn status_label(text: &Strings, status: &ListenerStatus) -> String {
    let (template, port) = match status {
        ListenerStatus::Starting => return text.starting.to_owned(),
        ListenerStatus::Listening { port } => (text.listening, port),
        ListenerStatus::Failed { port, error } => match error {
            BindError::PortInUse => (text.port_in_use, port),
            BindError::PermissionDenied => (text.port_denied, port),
            BindError::Other => (text.port_failed, port),
        },
    };
    template.replace("{port}", &port.to_string())
}

/// macOS: the Dock icon exists only while the window is open, like the Windows taskbar button.
pub fn show_main_window(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    if let Err(error) = app.set_dock_visibility(true) {
        log::error!("dock show failed: {error}");
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Closing never quits: the emulator keeps receiving jobs from the tray.
pub fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    // ponytail: tao ignores a hide within 1 s of a show (macOS leaves stray Dock icons
    // otherwise), so a very fast open/close keeps the icon until the next close.
    #[cfg(target_os = "macos")]
    if let Err(error) = app.set_dock_visibility(false) {
        log::error!("dock hide failed: {error}");
    }
}

/// macOS app menu, shown while the window is open. It replaces Tauri's default so that
/// ⌘Q closes the window instead of `terminate:` (which would stop the emulator).
/// The Edit menu is what makes ⌘C/⌘V work in the webview, so it must stay.
#[cfg(target_os = "macos")]
pub fn app_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let text = Locale::current().strings();
    let close = MenuItem::with_id(app, "close_window", text.close_window, true, Some("Cmd+Q"))?;
    let app_submenu = Submenu::with_items(
        app,
        PRODUCT_NAME,
        true,
        &[
            &close,
            &PredefinedMenuItem::close_window(app, Some(text.close))?,
        ],
    )?;
    let edit_submenu = Submenu::with_items(
        app,
        text.edit,
        true,
        &[
            &PredefinedMenuItem::undo(app, Some(text.undo))?,
            &PredefinedMenuItem::redo(app, Some(text.redo))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, Some(text.cut))?,
            &PredefinedMenuItem::copy(app, Some(text.copy))?,
            &PredefinedMenuItem::paste(app, Some(text.paste))?,
            &PredefinedMenuItem::select_all(app, Some(text.select_all))?,
        ],
    )?;
    Menu::with_items(app, &[&app_submenu, &edit_submenu])
}

fn set_autostart(app: &AppHandle, enabled: bool) {
    let autolaunch = app.autolaunch();
    let result = if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    match result {
        Ok(()) => log::info!("autostart enabled={enabled}"),
        Err(error) => log::error!("autostart enabled={enabled} failed: {error}"),
    }
}

fn open_logs(app: &AppHandle) {
    let opened = app
        .path()
        .app_log_dir()
        .map_err(|error| error.to_string())
        .and_then(|dir| {
            app.opener()
                .open_path(dir.to_string_lossy(), None::<&str>)
                .map_err(|error| error.to_string())
        });
    if let Err(error) = opened {
        log::error!("open log dir failed: {error}");
    }
}

pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let text = Locale::current().strings();
    let autostart_on = app.autolaunch().is_enabled().unwrap_or(false);

    let status = MenuItem::with_id(app, "status", text.starting, false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", text.open, true, None::<&str>)?;
    let test = MenuItem::with_id(app, "test", text.print_test, true, None::<&str>)?;
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        text.autostart,
        true,
        autostart_on,
        None::<&str>,
    )?;
    let logs = MenuItem::with_id(app, "logs", text.logs, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", text.quit, true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &status,
            &PredefinedMenuItem::separator(app)?,
            &open,
            &test,
            &autostart,
            &logs,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
    app.manage(TrayStatus(status));

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip(PRODUCT_NAME)
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "test" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = send_test_receipt(&app).await;
                });
            }
            // The OS flips the check mark on click; read it back rather than assume.
            "autostart" => set_autostart(app, autostart.is_checked().unwrap_or(false)),
            "logs" => open_logs(app),
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::status_label;
    use crate::listener::{BindError, ListenerStatus};
    use crate::locale::Locale;

    #[test]
    fn labels_every_status_with_its_port() {
        let text = Locale::En.strings();
        for (status, expected) in [
            (ListenerStatus::Starting, "Starting…"),
            (
                ListenerStatus::Listening { port: 9100 },
                "Listening on port 9100",
            ),
            (
                ListenerStatus::Failed {
                    port: 9100,
                    error: BindError::PortInUse,
                },
                "Port 9100 is in use",
            ),
            (
                ListenerStatus::Failed {
                    port: 9101,
                    error: BindError::PermissionDenied,
                },
                "Port 9101 is blocked",
            ),
            (
                ListenerStatus::Failed {
                    port: 9102,
                    error: BindError::Other,
                },
                "Can't open port 9102",
            ),
        ] {
            assert_eq!(status_label(text, &status), expected);
        }
    }
}
