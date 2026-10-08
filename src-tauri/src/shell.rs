//! The app around the webview: tray, macOS app menu, window show/hide, and the forwarder
//! that sends changes to the webview and the tray.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(target_os = "macos")]
use tauri::menu::{AboutMetadataBuilder, Submenu};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_opener::OpenerExt as _;

use crate::commands::send_test_receipt;
use crate::listener::{lock, BindError, ListenerStatus, Shared};
use crate::locale::{Locale, Strings};
use crate::receipts::ReceiptSummary;
use crate::settings::Theme;

/// The tray tooltip's name.
pub(crate) const PRODUCT_NAME: &str = "Thermal Printer Emulator";
/// Credited in the macOS About panel (the Settings panel credits it too, in TS).
#[cfg(target_os = "macos")]
const AUTHOR: &str = "AstrOOnauta";
pub(crate) const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

/// The last count from `set_unseen`, to show it again in a new language.
pub(crate) static UNSEEN: AtomicU32 = AtomicU32::new(0);

/// The count next to the tray icon (macOS menu bar, Linux AppIndicator label; Windows has
/// none) and in its tooltip, in the current language.
pub(crate) fn show_unseen_in_tray(app: &AppHandle) {
    let count = UNSEEN.load(Ordering::Relaxed);
    if let Some(tray) = app.tray_by_id("main") {
        #[cfg(not(target_os = "windows"))]
        let _ = tray.set_title((count > 0).then(|| count.to_string()));
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
            log::warn!("tray_tooltip_failed error={error}");
        }
    }
}

/// The tray's first, disabled item: the listener status line. Replaced when the menu is
/// rebuilt for a new language.
#[derive(Default)]
pub struct TrayStatus(Mutex<Option<MenuItem<tauri::Wry>>>);

/// Least time between two updates to the webview: a flood of receipts is at most this many
/// renders a second.
const FORWARD_PAUSE: Duration = Duration::from_millis(50);

/// Sends the webview (and the tray, for the status) what changed, forever. It wakes on
/// `Shared::changed` and sends the current state, only the parts that differ from what it
/// last sent: changes made while it sends or pauses become one update, so nothing queues.
pub async fn forward_changes(app: AppHandle) {
    let shared = Arc::clone(&app.state::<Arc<Shared>>());
    let mut sent_status: Option<ListenerStatus> = None;
    let mut sent_receipts: Option<Vec<ReceiptSummary>> = None;
    loop {
        shared.changes.notified().await;
        let status = lock(&shared.status).clone();
        if sent_status.as_ref() != Some(&status) {
            // The item is cloned out of the lock first: off the main thread, `set_text` waits
            // for the main thread, which may be waiting for this lock (a menu rebuild).
            let item = app
                .try_state::<TrayStatus>()
                .and_then(|tray_status| lock(&tray_status.0).clone());
            if let Some(item) = item {
                let label = status_label(Locale::current().strings(), &status);
                if let Err(error) = item.set_text(label) {
                    log::error!("tray_status_failed error={error}");
                }
            }
            if let Err(error) = app.emit("listener_status", &status) {
                log::error!("emit_failed event=listener_status error={error}");
            }
            sent_status = Some(status);
        }
        let receipts = lock(&shared.receipts).summaries();
        if sent_receipts.as_ref() != Some(&receipts) {
            if let Err(error) = app.emit("receipts", &receipts) {
                log::error!("emit_failed event=receipts error={error}");
            }
            sent_receipts = Some(receipts);
        }
        tokio::time::sleep(FORWARD_PAUSE).await;
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
        log::error!("dock_show_failed error={error}");
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
        log::error!("dock_hide_failed error={error}");
    }
}

/// macOS app menu, shown while the window is open. It replaces Tauri's default so that
/// ⌘Q closes the window instead of `terminate:` (which would stop the emulator).
/// The Edit menu is what makes ⌘C/⌘V work in the webview, so it must stay.
#[cfg(target_os = "macos")]
pub fn app_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let text = &Locale::current().strings().menu;
    let close = MenuItem::with_id(app, "close_window", text.close_window, true, Some("Cmd+Q"))?;
    // The native About panel: name, version and credits.
    let about = PredefinedMenuItem::about(
        app,
        Some(text.about),
        Some(
            AboutMetadataBuilder::new()
                .name(Some(PRODUCT_NAME))
                .version(Some(app.package_info().version.to_string()))
                .credits(Some(format!(
                    "{}\n{REPOSITORY}",
                    text.made_by.replace("{author}", AUTHOR)
                )))
                .build(),
        ),
    )?;
    let app_submenu = Submenu::with_items(
        app,
        PRODUCT_NAME,
        true,
        &[
            &about,
            &PredefinedMenuItem::separator(app)?,
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

/// The window's appearance: its title bar, and the webview's `prefers-color-scheme` on macOS
/// and Windows (the webview also gets `data-theme`, for Linux). `System` follows the OS.
pub fn apply_theme(app: &AppHandle, theme: Theme) {
    let theme = match theme {
        Theme::System => None,
        Theme::Light => Some(tauri::Theme::Light),
        Theme::Dark => Some(tauri::Theme::Dark),
    };
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = window.set_theme(theme) {
            log::warn!("theme_failed error={error}");
        }
    }
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
        Err(error) => log::error!("autostart_failed enabled={enabled} error={error}"),
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
        log::error!("open_logs_failed error={error}");
    }
}

/// The tray menu in the current language, with the current listener status line.
fn tray_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let text = Locale::current().strings();
    let autostart_on = app.autolaunch().is_enabled().unwrap_or(false);
    let label = app
        .try_state::<Arc<Shared>>()
        .map(|shared| status_label(text, &lock(&shared.status)))
        .unwrap_or_else(|| text.starting.to_owned());

    let status = MenuItem::with_id(app, "status", label, false, None::<&str>)?;
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
    if let Some(tray_status) = app.try_state::<TrayStatus>() {
        *lock(&tray_status.0) = Some(status);
    }
    Ok(menu)
}

pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip(PRODUCT_NAME)
        .menu(&tray_menu(app)?)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "test" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    // Connection failures are logged inside; this is the "not listening" case.
                    if let Err(error) = send_test_receipt(&app).await {
                        log::info!("test_receipt_skipped reason={}", error.key);
                    }
                });
            }
            // The OS flips the check mark on click; the new value is the opposite of the
            // OS state. The menu is rebuilt after, so the mark shows what the OS really did.
            "autostart" => {
                let enabled = !app.autolaunch().is_enabled().unwrap_or(false);
                set_autostart(app, enabled);
                if let Err(error) = refresh_menus(app) {
                    log::error!("menus_refresh_failed error={error}");
                }
            }
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

/// Rebuilds the native menus (tray, macOS app menu) and the tray tooltip in the current
/// language.
pub fn refresh_menus(app: &AppHandle) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(tray_menu(app)?))?;
    }
    show_unseen_in_tray(app);
    #[cfg(target_os = "macos")]
    app.set_menu(app_menu(app)?)?;
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
