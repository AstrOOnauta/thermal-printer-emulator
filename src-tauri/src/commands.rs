//! The webview's commands (`conventions/bridge.md`), and the listener restart they share
//! with setup. Thin: the emulator's logic lives in `listener`, `receipts` and `settings`.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use serde::Serialize;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt as _;

use crate::listener::{self, lock, Limits, ListenerStatus, Shared};
use crate::locale::Locale;
use crate::receipts::{CodePages, ReceiptSummary, ReceiptView};
use crate::settings::{self, Settings};
use crate::shell::{refresh_menus, show_unseen_in_tray, UNSEEN};

/// A refused command: an i18n key the webview translates. Rust never sends a sentence.
#[derive(Debug, Serialize)]
pub struct UiError {
    pub(crate) key: &'static str,
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
/// Async with the write on a blocking thread: a sync command would run on the main thread,
/// and a receipt can be 16 MB.
#[tauri::command]
pub async fn export_receipt(app: AppHandle, id: u64) -> Result<String, UiError> {
    let gone = || UiError::new("receipts.errors.gone");
    let (started_at, bytes) = {
        let shared = app.state::<Arc<Shared>>();
        let receipts = lock(&shared.receipts);
        let started_at = receipts.summary(id).ok_or_else(gone)?.started_at;
        let raw = receipts.raw(id).ok_or_else(gone)?;
        let code_pages = receipts.code_pages(id).ok_or_else(gone)?;
        (started_at, replayable(raw, code_pages))
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
    let written = {
        let (path, length) = (path.clone(), bytes.len());
        tauri::async_runtime::spawn_blocking(move || std::fs::write(&path, &bytes))
            .await
            .map_err(std::io::Error::other)
            .and_then(|result| result)
            .map(|()| length)
    };
    let length = written.map_err(|error| {
        log::error!(
            "export_failed id={id} path={} error={error}",
            path.display()
        );
        UiError::new("receipts.errors.export")
    })?;
    log::info!("receipt_exported id={id} bytes={length}");
    if let Err(error) = app.opener().reveal_item_in_dir(&path) {
        log::warn!("reveal_failed path={} error={error}", path.display());
    }
    Ok(name)
}

/// A receipt's bytes that print it again on their own: a receipt after the first of its
/// connection usually starts mid-stream (`ESC a`, text), which the connection filter turns
/// away, and may rely on an earlier `ESC t`. Those get `ESC @` and `ESC t n` in front.
///
/// ponytail: only the code page is restored; other state an earlier receipt set (alignment,
/// sizes, barcode settings) is not, as on any fresh connection. Replaying the earlier
/// receipts' bytes first would restore it, if that ever matters.
fn replayable(raw: &[u8], code_pages: CodePages) -> Vec<u8> {
    if raw.starts_with(b"\x1b@") {
        return raw.to_vec();
    }
    let mut bytes = vec![0x1b, b'@', 0x1b, b't', code_pages.start.table()];
    bytes.extend_from_slice(raw);
    bytes
}

/// A receipt's commands, re-parsed from its raw bytes (`escpos::inspect`). `None` once
/// dropped from memory. The bytes are copied out so parsing never holds the lock, and
/// parsed on a blocking thread (up to 16 MB).
#[tauri::command]
pub async fn get_receipt_commands(
    app: AppHandle,
    id: u64,
) -> Option<crate::escpos::inspect::Inspection> {
    let (raw, code_pages) = {
        let shared = app.state::<Arc<Shared>>();
        let receipts = lock(&shared.receipts);
        (receipts.raw(id)?.to_vec(), receipts.code_pages(id)?)
    };
    tauri::async_runtime::spawn_blocking(move || {
        crate::escpos::inspect::inspect(&raw, code_pages.start, code_pages.default)
    })
    .await
    .ok()
}

/// Receipts that finished while the window was not in front: a badge on the Dock icon
/// (macOS, some Linux docks), the count next to the menu bar icon (macOS: the Dock icon is
/// hidden while the window is closed), and the tray tooltip everywhere. 0 clears them.
#[tauri::command]
pub fn set_unseen(app: AppHandle, count: u32) {
    UNSEEN.store(count, Ordering::Relaxed);
    let badge = (count > 0).then_some(i64::from(count));
    if let Some(window) = app.get_webview_window("main") {
        // Unsupported on Windows (it has no badge count): nothing to do there.
        let _ = window.set_badge_count(badge);
    }
    show_unseen_in_tray(&app);
}

/// Quits from the window: where no tray icon shows (GNOME without the AppIndicator
/// extension), closing the window only hides it, and this is the way out.
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    log::info!("quit from=window");
    app.exit(0);
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
pub async fn get_receipt(app: AppHandle, id: u64) -> Option<ReceiptView> {
    // Async: the print model (bitmaps included) is cloned and serialized off the main
    // thread.
    let shared = app.state::<Arc<Shared>>();
    let view = listener::lock(&shared.receipts).view(id);
    view
}

/// Whether the emulator is listening, and on which port.
#[tauri::command]
pub fn get_listener_status(shared: State<'_, Arc<Shared>>) -> ListenerStatus {
    listener::lock(&shared.status).clone()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::replayable;
    use crate::escpos::codepage::CodePage;
    use crate::receipts::CodePages;

    #[test]
    fn a_saved_receipt_replays_on_its_own() {
        let pages = CodePages {
            start: CodePage::Cp850,
            default: CodePage::DEFAULT,
        };
        assert_eq!(
            replayable(b"\x1ba\x01Two\n", pages),
            b"\x1b@\x1bt\x02\x1ba\x01Two\n",
            "mid-stream: reset and the code page it printed with"
        );
        assert_eq!(
            replayable(b"\x1b@One\n", pages),
            b"\x1b@One\n",
            "already opens with ESC @: as received"
        );
    }

    /// The text between `start` and the next `end`.
    fn between<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
        let from = text.find(start).expect(start) + start.len();
        &text[from..from + text[from..].find(end).expect(end)]
    }

    fn names<'a>(list: impl Iterator<Item = &'a str>) -> BTreeSet<String> {
        list.map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect()
    }

    /// A command lives in four places (`conventions/bridge.md`); one left out is found only
    /// at runtime, as a denied or unknown command.
    #[test]
    fn every_command_is_listed_allowed_handled_and_wrapped() {
        let listed = names(
            between(include_str!("../build.rs"), ".commands(&[", "]")
                .split(',')
                .map(|name| name.trim().trim_matches('"')),
        );
        let allowed = include_str!("../capabilities/main.json")
            .split('"')
            .filter_map(|word| word.strip_prefix("allow-"))
            .map(|word| word.replace('-', "_"))
            .collect::<BTreeSet<_>>();
        let handled = names(
            between(include_str!("lib.rs"), "generate_handler![", "]")
                .split(',')
                .map(|name| name.trim().trim_start_matches("commands::")),
        );
        let wrappers = [
            include_str!("../../src/shared/api/app.ts"),
            include_str!("../../src/shared/api/emulator.ts"),
            include_str!("../../src/shared/api/settings.ts"),
        ]
        .concat();
        // `invoke<T>('name', …)`
        let wrapped = names(wrappers.split("invoke<").skip(1).filter_map(|rest| {
            let (_, name) = rest.split_once("('")?;
            name.split('\'').next()
        }));
        assert!(listed.len() > 10, "parsed {listed:?}");
        assert_eq!(allowed, listed, "capabilities/main.json vs build.rs");
        assert_eq!(handled, listed, "lib.rs generate_handler! vs build.rs");
        assert_eq!(wrapped, listed, "src/shared/api/*.ts vs build.rs");
    }
}
