mod capture;
pub mod escpos;
pub mod listener;
mod locale;
mod network;
pub mod receipts;
pub mod settings;
mod test_receipt;
mod ui;

use std::sync::Arc;

use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};
use tokio::sync::mpsc;

/// Passed by the OS login item; such a launch starts hidden in the tray.
const AUTOSTART_ARG: &str = "--autostart";

/// Files rotated at 2 MB, 5 kept; stdout too in dev builds.
fn log_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let mut targets = vec![Target::new(TargetKind::LogDir { file_name: None })];
    if cfg!(debug_assertions) {
        targets.push(Target::new(TargetKind::Stdout));
    }
    tauri_plugin_log::Builder::new()
        .targets(targets)
        .level(log::LevelFilter::Info)
        .max_file_size(2_000_000)
        .rotation_strategy(RotationStrategy::KeepSome(5))
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .build()
}

pub fn run() {
    let builder = tauri::Builder::default();
    // Elsewhere an app menu would become a menu bar inside the window.
    #[cfg(target_os = "macos")]
    let builder = builder.menu(ui::app_menu).on_menu_event(|app, event| {
        if event.id() == "close_window" {
            ui::hide_main_window(app);
        }
    });

    builder
        // Must be first: a second launch focuses this instance and exits.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            ui::show_main_window(app);
        }))
        .plugin(log_plugin())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_ARG]),
        ))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle();
            let settings_path = ui::settings_path(handle)?;
            let settings = settings::Settings::load(&settings_path);
            // Before any native menu is built, so they all start in the chosen language.
            locale::Locale::prefer(settings.language);
            app.manage(ui::SettingsPath(settings_path));
            app.manage(Arc::new(listener::Shared::new(
                receipts::Receipts::default(),
                settings,
            )));
            app.manage(ui::TrayStatus::default());
            ui::build_tray(handle)?;
            // The macOS app menu was built by `Builder::menu`, before the settings loaded.
            #[cfg(target_os = "macos")]
            app.set_menu(ui::app_menu(handle)?)?;
            let (events, mut received) = mpsc::unbounded_channel();
            let forwarder = handle.clone();
            // One consumer, so the webview sees events in the order the listener made them.
            tauri::async_runtime::spawn(async move {
                while let Some(event) = received.recv().await {
                    ui::forward(&forwarder, event);
                }
            });
            app.manage(ui::Events(events));
            app.manage(ui::ListenerTask::default());
            let starter = handle.clone();
            tauri::async_runtime::spawn(async move { ui::restart_listener(&starter).await });

            // The window is created hidden (tauri.conf.json) so a login launch never flashes it.
            let autostart = std::env::args().any(|arg| arg == AUTOSTART_ARG);
            if !autostart {
                ui::show_main_window(handle);
            }
            log::info!(
                "started version={} locale={} autostart={autostart}",
                app.package_info().version,
                locale::Locale::current().tag(),
            );
            Ok(())
        })
        // Closing hides; quitting is only from the tray.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                ui::hide_main_window(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            ui::app_locale,
            ui::get_receipts,
            ui::get_receipt,
            ui::get_listener_status,
            ui::get_settings,
            ui::get_lan_address,
            ui::print_test_receipt,
            ui::clear_receipts,
            ui::export_receipt,
            ui::set_unseen,
            ui::set_settings
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, _event| {
            // Opening the app again from Finder/Launchpad does not start a second process on
            // macOS (so single-instance never fires): LaunchServices sends "reopen" instead.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                log::info!("reopen: show window");
                ui::show_main_window(_app);
            }
        });
}
