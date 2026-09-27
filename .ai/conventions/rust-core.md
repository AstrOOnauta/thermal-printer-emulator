# Rust core

One process, single instance. The Tauri main thread owns the tray and the window; work
that waits on the network runs on `tauri::async_runtime` (Tokio).

## Modules (one responsibility each)

| Module      | Owns                                                                                                           |
| ----------- | -------------------------------------------------------------------------------------------------------------- |
| `main.rs`   | Entry point, only calls `lib::run`. `windows_subsystem = "windows"` hides the console in release               |
| `lib.rs`    | `Builder`: plugin order, macOS app menu, `setup` (tray, first show), window events, command handlers, `Reopen` |
| `ui.rs`     | Commands, tray, macOS app menu, window show/hide + Dock visibility. **No emulator logic**                      |
| `locale.rs` | OS language → `Locale` (cached in a `OnceLock`), native menu labels                                            |

The crate is `thermal-printer-emulator` and the lib is `thermal_printer_emulator_lib`. The
`_lib` suffix keeps the lib name distinct from the bin name (cargo#8519 on Windows).

## Plugins (`lib.rs`)

| Plugin            | Config                                                                                           |
| ----------------- | ------------------------------------------------------------------------------------------------ |
| `single-instance` | **Registered first**. A second launch calls `show_main_window` on the running instance and exits |
| `log`             | `LogDir` target (+ `Stdout` in debug builds), Info level, 2 MB × 5 files, local timezone         |
| `autostart`       | `MacosLauncher::LaunchAgent`, arg `--autostart` (starts hidden)                                  |
| `opener`          | Rust side only (`open_logs`). The webview has no opener permission                               |

## Security rules

- The webview gets only the commands listed in `build.rs` and allowed in
  `capabilities/main.json` (`conventions/bridge.md`).
- No shell. If an OS tool ever has to run, it gets an argv with a fixed binary path, never
  a shell string.
- **Listening socket (P1)**: the app accepts connections from the LAN by design (POS
  terminals on other machines). Rules for that code:
  - parse bytes as untrusted: bounded buffers, a size limit per job, an idle timeout per
    connection, and a cap on concurrent connections;
  - reject connections that do not start with `ESC @`. Port scanners probe 9100 with
    other protocols (see virtual-thermal-printer's `escpos.ts`);
  - a failing connection never takes the app down: the release profile uses
    `panic = "unwind"`, so a panic in a connection task ends that task only;
  - never log job bytes or decoded text.

## Logging

`key=value` fields, one event per line: `log::info!("started version={v} locale={l}")`.
Log state changes, connections (peer address, byte count, duration) and errors with their
context. **Never** log receipt bytes or text: they can carry customer data.

## Tests

- Unit tests next to the code (`#[cfg(test)] mod tests`), e.g. `locale::tests`.
- Integration tests in `src-tauri/tests/` against real sockets on `127.0.0.1`, with an
  ephemeral port (`:0`) so they run in parallel.

## Platform

- **macOS**: the Dock icon shows only while the window is open (`set_dock_visibility`;
  `LSUIElement` in `Info.plist` so the app starts without one), plus a custom app menu
  where ⌘Q hides the window (`flows/app-lifecycle.md`). Ad-hoc signed (`stack.md`).
  `NSLocalNetworkUsageDescription` is declared for the macOS 15+ Local Network prompt.
- **Windows**: WebView2 `embedBootstrapper`, per-user NSIS (`installMode: currentUser`),
  so no admin is needed. The installer is in English, Spanish or Portuguese (BR), picked
  from the OS.
- **Linux**: `.deb` depends on `libwebkit2gtk-4.1-0` and `libayatana-appindicator3-1`;
  `.rpm` on `libayatana-appindicator-gtk3`. The AppImage needs `libfuse2`.

## Cargo

- Release profile: LTO, `codegen-units = 1`, `opt-level = "s"`, stripped,
  **`panic = "unwind"`**: with `abort`, a panic in any connection task would kill the app.
- Dependencies use caret versions (`"2"`); `Cargo.lock` is committed and pins them.
