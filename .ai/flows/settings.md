# Settings

User settings live in `settings.json` in the app's config folder
(`app_config_dir()`), owned by Rust (`settings.rs`). The webview reads and changes them
only through `get_settings` / `set_settings`, and Rust validates every value.

| Field       | Default | Values                                                | Applies                       |
| ----------- | ------- | ----------------------------------------------------- | ----------------------------- |
| `port`      | 9100    | 1–65535                                               | restarts the listener         |
| `bind`      | `lan`   | `lan` (0.0.0.0, decision 2) / `local` (127.0.0.1)     | restarts the listener         |
| `paper`     | `mm80`  | `mm80` (576 dots) / `mm58` (384 dots)                 | next connections              |
| `code_page` | 0       | a supported `ESC t` table (`codepage.rs`)             | next connections (decision 4) |
| `sound`     | true    | play the printer's beep (`flows/print-job.md` § Beep) | at once (webview)             |

## Load and save

- **Load** at startup: a missing file gives the defaults; an unreadable, broken or invalid
  file is logged (`settings_invalid`) and gives the defaults too. Never fatal.
- Missing fields take their default (`#[serde(default)]`), so a file from an older version
  still loads.
- **Save** writes `settings.json.tmp`, then renames it over the old file: a crash mid-write
  never leaves half a file.

## `set_settings(settings)`

```
validate ─ invalid ─▶ UiError { key: settings.errors.port | settings.errors.codePage }
   │
save ─ fails ─▶ UiError { key: settings.errors.save } (logged)
   │
Shared.settings ← new value (log settings_changed)
   │
address changed? ─ yes ─▶ ui::restart_listener: abort the running listener task, await it
   │                        (its socket is closed), spawn listener::run on the new address
   ▼
returns the saved settings
```

- Each connection reads `paper` and `code_page` when it is accepted, so a change never
  alters a receipt mid-print.
- `Printer::with_code_page`: `ESC @` returns to the configured default, not to CP437.
- Connections already open keep running on the old socket's tasks until they end.

## Screen (`src/screens/settings/`)

Opened from the header's "Settings" button (it becomes "Receipts" to go back). Every
choice is saved at once; the port waits for "Apply" (restarting the listener on every
keystroke would be wrong) and is validated as you type. A refused change shows its
translated error (`uiErrorKey`) in an alert.
