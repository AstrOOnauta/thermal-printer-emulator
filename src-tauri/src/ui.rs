//! UI bridge: tray, app menu, window show/hide and the webview's commands. No emulator
//! logic here.

use std::sync::Arc;
#[cfg(target_os = "macos")]
use tauri::menu::Submenu;

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_opener::OpenerExt as _;

use crate::jobs::JobSummary;
use crate::listener::{self, Event, Shared};
use crate::locale::Locale;

const PRODUCT_NAME: &str = "Thermal Printer Emulator";

/// The webview's UI language (`en`, `es` or `pt-BR`).
#[tauri::command]
pub fn app_locale() -> &'static str {
    Locale::current().tag()
}

/// Received jobs, oldest first. Same list as the `jobs` event.
#[tauri::command]
pub fn get_jobs(shared: State<'_, Arc<Shared>>) -> Vec<JobSummary> {
    listener::lock(&shared.jobs).summaries()
}

/// Hands a listener event to the webview.
pub fn forward(app: &AppHandle, event: Event) {
    let emitted = match event {
        Event::Jobs(jobs) => app.emit("jobs", jobs),
        Event::Status(_) => Ok(()),
    };
    if let Err(error) = emitted {
        log::error!("emit failed: {error}");
    }
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

    let open = MenuItem::with_id(app, "open", text.open, true, None::<&str>)?;
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
            &open,
            &autostart,
            &logs,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip(PRODUCT_NAME)
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
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
