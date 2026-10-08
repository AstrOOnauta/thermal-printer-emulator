# Rust core

One process, single instance. The Tauri main thread owns the tray and the window; work
that waits on the network runs on `tauri::async_runtime` (Tokio).

## Modules (one responsibility each)

| Module               | Owns                                                                                                                          |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `main.rs`            | Entry point, only calls `lib::run`. `windows_subsystem = "windows"` hides the console in release                              |
| `lib.rs`             | `Builder`: plugin order, macOS app menu, `setup` (tray, first show), window events, command handlers, `Reopen`                |
| `commands.rs`        | The webview's commands, `UiError`, listener restart (`ListenerTask`). Thin: **no emulator logic**                             |
| `shell.rs`           | Tray, macOS app menu, window show/hide + Dock visibility, `forward_changes` (changes → webview and tray)                      |
| `locale.rs`          | `Language` setting + OS language → `Locale::current()`, native menu labels                                                    |
| `listener.rs`        | TCP accept loop, bind retry, connection cap, one task per connection, `ListenerStatus`, `Shared` state (`flows/print-job.md`) |
| `receipts.rs`        | Receipts in memory: limits, eviction, `ReceiptSummary` / `ReceiptView`, raw bytes (`flows/print-job.md`)                      |
| `capture.rs`         | One connection's bytes → receipts: start on visible output, split on cut (`flows/print-job.md`)                               |
| `settings.rs`        | `Settings`: defaults, validation, load (field by field), atomic save (`flows/settings.md`)                                    |
| `updates.rs`         | Update checks (launch, then daily), the found update, install + restart (`flows/app-lifecycle.md`)                            |
| `network.rs`         | This computer's LAN IPv4 (UDP "connect" to TEST-NET-1, nothing sent)                                                          |
| `test_receipt.rs`    | The test receipt's bytes (UI language, configured code page and paper) and sending them to our own port                       |
| `escpos/mod.rs`      | `Decoder`: parser + printer for one connection, outputs tagged with stream offsets                                            |
| `escpos/parser.rs`   | ESC/POS bytes → `Command`, streaming (`conventions/escpos.md`)                                                                |
| `escpos/command.rs`  | `Command` and its parameter types: what the parser makes, what the printer and inspector take                                 |
| `escpos/codepage.rs` | `ESC t` tables → `char`; tables generated into `codepage_tables.rs` by `scripts/codepages.py`                                 |
| `escpos/barcode.rs`  | `GS k` symbologies → bar widths + HRI text                                                                                    |
| `escpos/bitmap.rs`   | 1-bit images: rows, scale, crop, base64 serialization                                                                         |
| `escpos/printer.rs`  | Printer state machine: commands → print model + side effects; barcodes and QR in `printer/codes.rs`                           |
| `escpos/model.rs`    | The print model (`Paper`, `Font`, `Segment`, `Placed`, `Block`) and `Output`, serialized for the webview                      |
| `escpos/inspect.rs`  | Raw bytes → command rows (offset, bytes, mnemonic, i18n kind, detail) for the Commands view                                   |

The crate is `thermal-printer-emulator` and the lib is `thermal_printer_emulator_lib`. The
`_lib` suffix keeps the lib name distinct from the bin name (cargo#8519 on Windows).

## Plugins (`lib.rs`)

| Plugin            | Config                                                                                           |
| ----------------- | ------------------------------------------------------------------------------------------------ |
| `single-instance` | **Registered first**. A second launch calls `show_main_window` on the running instance and exits |
| `log`             | `LogDir` target (+ `Stdout` in debug builds), Info level, 2 MB × 5 files, local timezone         |
| `autostart`       | `MacosLauncher::LaunchAgent`, arg `--autostart` (starts hidden)                                  |
| `opener`          | Rust side only (logs, repository, release page). The webview has no opener permission            |
| `updater`         | Rust side only (`updates.rs`). Endpoint and public key in `tauri.conf.json`                      |

## Security rules

- The webview gets only the commands listed in `build.rs` and allowed in
  `capabilities/main.json` (`conventions/bridge.md`).
- No shell. If an OS tool ever has to run, it gets an argv with a fixed binary path, never
  a shell string.
- **Listening socket**: the app accepts connections from the LAN by design (POS
  terminals on other machines). Rules for that code:
  - parse bytes as untrusted: bounded buffers, a size limit per receipt, an idle timeout
    per connection (short until its first bytes), and a cap on concurrent connections;
  - turn away connections that do not open like ESC/POS (`listener::classify`, see
    `flows/print-job.md` § Connection filter): port scanners probe 9100 with other
    protocols;
  - a failing connection never takes the app down: the release profile uses
    `panic = "unwind"`, so a panic in a connection task ends that task only;
  - never log job bytes or decoded text.

## Logging

`key=value` fields, one event per line: `log::info!("started version={v} locale={l}")`.
Log state changes, connections (peer address, byte count, duration) and errors with their
context. **Never** log receipt bytes or text: they can carry customer data.

## Tests

- Unit tests next to the code (`#[cfg(test)] mod tests`), e.g. `locale::tests`. A big
  module keeps them in a child file (`escpos/parser/tests.rs`, `escpos/printer/tests.rs`).
- Integration tests in `src-tauri/tests/` against real sockets on `127.0.0.1`, with an
  ephemeral port (`:0`) so they run in parallel. Modules they use are `pub` in `lib.rs`;
  the rest stay private.
- Timeouts and limits come from a struct (`listener::Limits`, `Receipts::new`) so tests run
  them in milliseconds. Production values live next to the code, not in tests.

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
