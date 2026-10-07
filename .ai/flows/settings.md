# Settings

User settings live in `settings.json` in the app's config folder
(`app_config_dir()`), owned by Rust (`settings.rs`). The webview reads and changes them
only through `get_settings` / `set_settings`, and Rust validates every value.

| Field       | Default  | Values                                                                               | Applies                                                      |
| ----------- | -------- | ------------------------------------------------------------------------------------ | ------------------------------------------------------------ |
| `port`      | 9100     | 1–65535                                                                              | restarts the listener                                        |
| `bind`      | `lan`    | `lan` (0.0.0.0, decision 2) / `local` (127.0.0.1)                                    | restarts the listener                                        |
| `paper`     | `mm80`   | `mm80` (576 dots) / `mm58` (384 dots)                                                | next connections                                             |
| `code_page` | 0        | a supported `ESC t` table (`codepage.rs`)                                            | next connections (decision 4)                                |
| `sound`     | true     | printing sound for every receipt + the printer's beep (`flows/print-job.md` § Sound) | at once (webview)                                            |
| `language`  | `system` | `system` (follow the OS) / `en` / `es` / `pt-BR`                                     | at once: window, tray and macOS menu (`conventions/i18n.md`) |
| `zoom`      | 100      | 75 / 100 / 125 / 150 / 200 (`ZOOM_STEPS`, percent)                                   | at once: paper redrawn at the new scale; ⌘+ ⌘− ⌘0            |

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
language changed? ─ yes ─▶ Locale::prefer + shell::refresh_menus (tray, macOS app menu)
   │
address changed? ─ yes ─▶ commands::restart_listener: abort the running listener task, await it
   │                        (its socket is closed), spawn listener::run on the new address
   ▼
returns the saved settings
```

- Each connection reads `paper` and `code_page` when it is accepted, so a change never
  alters a receipt mid-print.
- `Printer::with_code_page`: `ESC @` returns to the configured default, not to CP437.
- Connections already open keep running on the old socket's tasks until they end.

## Panel (`src/screens/settings/`)

Opened by the gear in the top bar; a panel slides in from the right over the receipts
(`design-system.md` § Window), closed by Esc or ✕. Every choice is saved at once; the
port waits for "Apply" (restarting the listener on every keystroke would be wrong) and is
validated as you type. A refused change shows its translated error (`uiErrorKey`) in an
alert.
