# Webview ↔ Rust bridge

The webview gets **a handful of commands** and, from P1 on, **events** pushed by Rust.
Nothing else: no plugin APIs, no direct OS access.

## Commands

| Command      | Args | Returns                   | Notes                                                                                                           |
| ------------ | ---- | ------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `app_locale` | none | `'en' \| 'es' \| 'pt-BR'` | Resolved once per process from the OS (`conventions/i18n.md`). Called by `initLocale()` before the first render |

Wrappers: `src/shared/api/app.ts` (`getAppLocale`). Components never call `invoke`.

## Events

None yet. The listener (P1) introduces them. Document each one here: name, payload type,
when it fires, and the TS interface that mirrors it.

Rules for the wire:

- **snake_case** field names on both sides. TS interfaces keep them as they are.
- Rust structs and their TS interfaces (`src/shared/interfaces/`) change **together**, in
  the same commit.
- A failing command rejects with `{ key, params? }`, where `key` is a dot path into the
  translation files. The webview renders `t(error.key, error.params)`. Rust never sends a
  sentence.

## Adding a command (checklist)

1. `#[tauri::command] pub fn x(...)` in `ui.rs`. Keep it thin and delegate to the module
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
- Planned loosening: when decoded receipt images land (P2), add `img-src 'self' data:`.
  Nothing else. Don't loosen the CSP to make a library work; pick another library.
