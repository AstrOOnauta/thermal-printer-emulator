# App lifecycle

Boot, window, tray, single instance, autostart, installers and logs.

## Boot

```
main.rs → lib::run()
  Builder
    .menu(app_menu) + on_menu_event   # macOS only (see "macOS Dock and menu")
    .plugin(single-instance)          # FIRST: a 2nd launch → show_main_window() on the 1st (not for --autostart), then exits
    .plugin(log)                      # LogDir (+ Stdout in debug), 2 MB × 5
    .plugin(autostart)                # LaunchAgent / HKCU Run / XDG .desktop, arg --autostart
    .plugin(opener)                   # Rust side only
    .setup:
       build_tray()
       show_main_window() unless launched with --autostart   # tauri.conf has visible: false
       log "started version=… locale=… autostart=…"
  webview: main.tsx → initLocale() (app_locale) → render App
```

## Window

- Label `main`, created **hidden** (`"visible": false`). `setup` shows it unless the
  launch came from autostart. Showing from `setup` (instead of hiding later) avoids a
  flash at login.
- **Close hides** (`CloseRequested` → `prevent_close` + `hide_main_window`). The emulator
  keeps listening; quitting is only from the tray.
- **Opening the app again shows the window.** On Windows and Linux a second process
  triggers single-instance → `show_main_window`. On macOS there is no second process:
  LaunchServices sends `RunEvent::Reopen` to the running one, handled in the `run` closure
  in `lib.rs`.

## macOS Dock and menu

The **Dock icon shows only while the window is open**, like a Windows taskbar button:
`show_main_window` → `set_dock_visibility(true)` and `hide_main_window` →
`set_dock_visibility(false)`. `LSUIElement` in `Info.plist` makes the process start
without a Dock icon, so a login launch with a hidden window never shows one.

- tao ignores a hide that comes less than 1 s after a show (otherwise macOS leaves stray
  icons), so a very fast open/close keeps the icon until the next close.
- **Quitting**: `terminate:` (⌘Q in Tauri's default menu, Dock › Quit, logout, shutdown)
  goes straight to `RunEvent::Exit`, which **cannot be cancelled**. So the app menu is
  replaced: ⌘Q = "Close window" (hides) and ⌘W = close. Dock › Quit still quits for real.
  It starts with "About Thermal Printer Emulator": the native About panel with the
  version and the credits (`MenuStrings::made_by` in `locale.rs`, the repository URL from `Cargo.toml`).
  That is acceptable because the icon is only there while someone is looking at the
  window, and logout/shutdown are never blocked.
- The **Edit** menu (undo/redo/cut/copy/paste/select all) must stay: without it ⌘C/⌘V
  stop working in the webview.
- Windows and Linux have no app menu: there, it would become a menu bar inside the window.
- `yarn tauri dev` only: after close → reopen, the Dock shows the generic "exec" icon.
  Dev runs a bare binary (no `.app`, no `icon.icns`). Bundles are fine. Not fixed on
  purpose.

## Tray

| Item               | Behaviour                                                                                      |
| ------------------ | ---------------------------------------------------------------------------------------------- |
| Status (disabled)  | Listener status line: "Listening on port 9100", "Port 9100 is in use"… (`shell::status_label`) |
| Open               | `show_main_window` (unminimize, show, focus)                                                   |
| Print test receipt | Same as the window's button (`flows/print-job.md` § Test receipt); errors only logged          |
| Launch at login    | Toggles autostart, then rebuilds the menu from the OS state (a failure shows). Off by default  |
| Show logs          | Opens `app_log_dir()` in the file manager                                                      |
| Quit               | `app.exit(0)`                                                                                  |

Labels come from `locale.rs` (en/es/pt-BR), in the language setting. The tray icon is the
app icon. The status item is held in `TrayStatus` and updated by `shell::forward_changes`
whenever the status changes. When the language changes, `shell::refresh_menus` rebuilds the tray menu
(`tray_menu`, with the current status line), the macOS app menu and the tray tooltip.
The autostart item reads the OS state on click, so it works with any rebuilt menu.

**Quit from the window**: Ctrl+Q on Windows and Linux (`quit_app`), as apps there do. On
GNOME without the AppIndicator extension no tray icon shows and closing the window only
hides it, so this is the way out there. On macOS ⌘Q only closes the window (above).

Linux: the tray needs an AppIndicator host (GNOME requires an extension). Without one,
launching the app again focuses the window, which is the way back in.

## Keyboard shortcuts

In the window (`App`), ⌘ on macOS and Ctrl elsewhere (`shortcut.ts`): ⌘, settings, ⌘T test
receipt, ⌘⌫ clear (the same confirmation dialog; ignored while typing in a field), ⌘+ ⌘−
⌘0 paper zoom, Ctrl+Q quit (Windows and Linux only). While a modal dialog is open they do nothing (and Esc closes only the
dialog, not the settings panel behind it). ⌘, ⌘T ⌘⌫ act once per press, not on key
repeat, and ⌘T waits for the test receipt in flight. Tooltips show them ("Ctrl+Backspace"
off macOS); `aria-keyshortcuts` gets the spec's names (`Meta+,`, `Control+Backspace`).

## Installers

- **Windows**: NSIS, **per machine** (`Program Files`, asks for admin once). The hooks in
  `windows/hooks.nsh` add an inbound firewall rule for the app on **private and domain
  networks only** (so Windows never prompts, and public Wi-Fi stays closed) and delete it
  on uninstall. An update runs the uninstaller in update mode (`$UpdateMode`), which keeps
  the rule; Tauri's own template likewise keeps the login entry through updates and
  removes it on a real uninstall.
- **macOS**: `.dmg` with a universal `.app`.
- **Linux**: `.deb`, `.rpm`, `.AppImage`.
- Not code-signed: the OS asks for confirmation on first launch (README § Installation).

## Logs

| OS      | Dir                                                                 |
| ------- | ------------------------------------------------------------------- |
| macOS   | `~/Library/Logs/io.github.astroonauta.thermalprinteremulator/`      |
| Windows | `%LOCALAPPDATA%\io.github.astroonauta.thermalprinteremulator\logs\` |
| Linux   | `~/.local/share/io.github.astroonauta.thermalprinteremulator/logs/` |
