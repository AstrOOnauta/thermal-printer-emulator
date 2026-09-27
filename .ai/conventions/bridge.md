# Webview ↔ Rust bridge

The webview gets **a handful of commands** and, from P1 on, **events** pushed by Rust.
Nothing else: no plugin APIs, no direct OS access.

## Commands

| Command      | Args | Returns                   | Notes                                                                                                           |
| ------------ | ---- | ------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `app_locale` | none | `'en' \| 'es' \| 'pt-BR'` | Resolved once per process from the OS (`conventions/i18n.md`). Called by `initLocale()` before the first render |
| `get_jobs`   | none | `IJobSummary[]`           | Received jobs, oldest first. Same list as the `jobs` event                                                      |

Wrappers: `src/shared/api/app.ts` (`getAppLocale`), `src/shared/api/emulator.ts`
(`getJobs`, `onJobs`). Components never call `invoke` or `listen`.

## Events

| Event  | Payload         | Fires when                                                                                                                                 |
| ------ | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `jobs` | `IJobSummary[]` | A job starts or ends. The **whole list**, oldest first (≤ 100 small items): the webview replaces its copy, so it can never drift from Rust |

`IJobSummary` (`src/shared/interfaces/emulator.ts`) mirrors `jobs::JobSummary`:
`{ id, peer: "ip:port", started_at, ended_at: number | null, state, size }`, times in unix
ms, `state` one of `receiving`, `done`, `idle_timeout`, `too_large`, `connection_error`.
Never the job's bytes.

The listener sends its changes on an `mpsc` channel; one forwarder task in `lib.rs`
(`ui::forward`) emits them, so events reach the webview in the order they happened.

**Sync rule** (`use-jobs.ts`): subscribe first, then read (`get_jobs`). Once an event has
arrived, the read's answer is ignored: it may be older.

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
