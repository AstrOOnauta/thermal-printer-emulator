# Webview ↔ Rust bridge

The webview gets **a handful of commands** and **events** pushed by Rust.
Nothing else: no plugin APIs, no direct OS access.

## Commands

| Command                | Args           | Returns                   | Notes                                                                                                           |
| ---------------------- | -------------- | ------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `app_locale`           | none           | `'en' \| 'es' \| 'pt-BR'` | Resolved once per process from the OS (`conventions/i18n.md`). Called by `initLocale()` before the first render |
| `get_receipts`         | none           | `IReceiptSummary[]`       | Printed receipts, oldest first. Same list as the `receipts` event                                               |
| `get_receipt`          | `{ id }`       | `IReceiptView \| null`    | One receipt with its print model, to draw. `null` once dropped from memory                                      |
| `get_listener_status`  | none           | `IListenerStatus`         | Same value as the `listener_status` event                                                                       |
| `get_settings`         | none           | `ISettings`               |                                                                                                                 |
| `get_lan_address`      | none           | `string \| null`          | This computer's LAN IPv4, `null` offline. Asked again whenever the listener (re)starts                          |
| `print_test_receipt`   | none           | `()` or `UiError`         | Sends the test receipt to 127.0.0.1:port. `testReceipt.errors.notListening` / `.send`                           |
| `clear_receipts`       | none           | `()`                      | Drops finished receipts; the new list goes out as a `receipts` event                                            |
| `export_receipt`       | `{ id }`       | file name or `UiError`    | Raw bytes to `Downloads/receipt-<started_at>-<id>.bin`, revealed. `receipts.errors.gone` / `.export`            |
| `get_receipt_commands` | `{ id }`       | `IInspection \| null`     | The receipt's commands re-parsed from its raw bytes (`conventions/escpos.md` § Inspect)                         |
| `set_unseen`           | `{ count }`    | `()`                      | Dock badge, tray count and tooltip; 0 clears them                                                               |
| `set_settings`         | `{ settings }` | `ISettings` or `UiError`  | Validates, saves, applies (`flows/settings.md`)                                                                 |

Wrappers: `src/shared/api/app.ts` (`getAppLocale`), `src/shared/api/settings.ts`
(`getSettings`, `setSettings`), `src/shared/api/emulator.ts`
(`getReceipts`, `onReceipts`, `getReceipt`, `getListenerStatus`, `onListenerStatus`,
`getLanAddress`). Components never call
`invoke` or `listen`.

Commands that may take long (`get_receipt`, `get_receipt_commands`, `export_receipt`) are
`async`: Tauri runs a sync command on the main thread, and a receipt can be 16 MB. Their
heavy part (parse, file write) runs on `spawn_blocking`.

## Events

| Event             | Payload             | Fires when                                                                                                                                     |
| ----------------- | ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `receipts`        | `IReceiptSummary[]` | A receipt starts or ends. The **whole list**, oldest first (≤ 100 small items): the webview replaces its copy, so it can never drift from Rust |
| `listener_status` | `IListenerStatus`   | The listener starts, fails to bind or recovers (only on change)                                                                                |

`IReceiptSummary` (`src/shared/interfaces/emulator.ts`) mirrors `receipts::ReceiptSummary`:
`{ id, peer: "ip:port", started_at, ended_at: number | null, state, cut, drawer, beeps,
size, paper, width, height }`, times in unix ms, `state` one of `printing`, `done`,
`idle_timeout`, `too_large`, `connection_error`, `cut` `full` / `partial` / `null`,
`paper` `mm80` / `mm58`, `width` and `height` in dots (so the webview reserves the paper's
exact size before drawing it). Never the raw bytes.

`IReceiptView` (from `get_receipt`) adds `blocks: IBlock[]`, the print model of
`conventions/escpos.md` (`IBlock`, `ISegment`, `IPlaced` mirror `model.rs`); bitmaps
are base64 1-bit rows.

`IListenerStatus` mirrors `listener::ListenerStatus`, tagged by `state`:
`{ state: 'starting' } | { state: 'listening', port } | { state: 'failed', port, error }`,
`error` one of `port_in_use`, `permission_denied`, `other`.

One forwarder task (`shell::forward_changes`, spawned in `lib.rs`) emits both events: it
wakes on `Shared::changed`, sends the current state when it differs from what it last
sent, and pauses 50 ms. A burst of changes is one event with the latest state, never a
queue (`flows/print-job.md` § Concurrency).

**Sync rule** (`use-synced.ts`, used for both): subscribe first, then read. Once an event
has arrived, the read's answer is ignored: it may be older. Its three arguments must be
stable (API functions, a module-level fallback constant).

Rules for the wire:

- **snake_case** field names on both sides. TS interfaces keep them as they are.
- Rust structs and their TS interfaces (`src/shared/interfaces/`) change **together**, in
  the same commit.
- A failing command rejects with `UiError = { key }`, a dot path into the translation
  files. The webview renders `t(uiErrorKey(error))`; any other rejection shape becomes
  `errors.unexpected`. Rust never sends a sentence.

## Adding a command (checklist)

1. `#[tauri::command] pub fn x(...)` in `commands.rs`. Keep it thin and delegate to the module
   that owns the logic.
2. Add `"x"` to `AppManifest::commands` in `src-tauri/build.rs`.
3. Add `"allow-x"` to `src-tauri/capabilities/main.json`.
4. Add it to `generate_handler![…]` in `lib.rs`.
5. Add a typed wrapper in `src/shared/api/`.
6. Update the table above.

Without steps 2 and 3 the command is **denied** at runtime. That is on purpose: listing
the commands in `build.rs` stops Tauri from exposing every command to every window by
default.

## Capabilities & CSP

- `capabilities/main.json`: `core:event:default` plus one `allow-<command>` per command.
  **No** fs, shell, http, opener or dialog permission for the webview. Rust opens folders.
- CSP (`tauri.conf.json`):
  `default-src 'self'; script-src 'self'; connect-src ipc: http://ipc.localhost`. Bundled
  files only: no remote fonts, images or scripts.
- Receipt images are 1-bit bitmaps drawn straight onto the canvas, so even they need no
  `img-src` exception. Don't loosen the CSP to make a library work; pick another library.
