//! The app around the webview: tray, macOS app menu, window show/hide, and the forwarder
//! that sends changes to the webview and the tray.

use std::sync::{Arc, Mutex};

#[cfg(target_os = "macos")]
use tauri::menu::Submenu;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_opener::OpenerExt as _;

use crate::commands::send_test_receipt;
use crate::listener::{lock, BindError, Event, ListenerStatus, Shared};
use crate::locale::{Locale, Strings};

/// The tray tooltip's name.
pub(crate) const PRODUCT_NAME: &str = "Thermal Printer Emulator";

/// The tray's first, disabled item: the listener status line. Replaced when the menu is
/// rebuilt for a new language.
#[derive(Default)]
pub struct TrayStatus(Mutex<Option<MenuItem<tauri::Wry>>>);

/// Hands a listener event to the webview (and the tray, for the status).
pub fn forward(app: &AppHandle, event: Event) {
    let emitted = match event {
        Event::Receipts(receipts) => app.emit("receipts", receipts),
        Event::Status(status) => {
            if let Some(tray_status) = app.try_state::<TrayStatus>() {
                if let Some(item) = lock(&tray_status.0).as_ref() {
                    let label = status_label(Locale::current().strings(), &status);
                    if let Err(error) = item.set_text(label) {
                        log::error!("tray status failed: {error}");
                    }
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
                    let _ = send_test_receipt(&app).await;
                });
            }
            // The OS flips the check mark on click; the new value is the opposite of the
            // OS state. Read from the OS, so it works with any rebuilt menu.
            "autostart" => {
                let enabled = !app.autolaunch().is_enabled().unwrap_or(false);
                set_autostart(app, enabled);
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

/// Rebuilds the native menus (tray, macOS app menu) in the current language.
pub fn refresh_menus(app: &AppHandle) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(tray_menu(app)?))?;
    }
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
