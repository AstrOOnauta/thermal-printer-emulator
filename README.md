# Thermal Printer Emulator

A **virtual thermal printer** for your desktop. This thermal printer emulator pretends to
be an ESC/POS network receipt printer: point your point-of-sale software at it, print,
and see the receipt on screen. No paper, no hardware, no terminal.

Use it to develop and test receipt printing without a physical printer, on Windows,
macOS and Linux.

> **Status: early development.** The app receives ESC/POS jobs on TCP port 9100 and
> draws the receipts: text styles, code pages, images, barcodes and QR codes. Next:
> installers with a firewall rule and automatic updates.

## How it works

Network receipt printers accept raw ESC/POS bytes on TCP port **9100**. The emulator
listens on that port, decodes the bytes and draws the receipt. It keeps running from the
system tray when the window is closed, and can start at login.

From the POS software, add a network (TCP/IP, "RAW" or "Socket") printer at:

- `127.0.0.1:9100` when the POS runs on the same computer, or
- `<this computer's LAN IP>:9100` from another device on the network.

The window shows the exact address to use, and **Print test receipt** checks that
everything works. In **Settings** you can change the port, allow only this computer,
pick 80 or 58 mm paper, the default code page, the printing sound and beep, the paper
zoom and the language.

A job must open like ESC/POS, with `ESC @` (initialize), as every ESC/POS library does.
Other traffic on port 9100, such as port scanners or plain text, is ignored. To try it
from a terminal:

```bash
printf '\x1b@Hello, printer!\n\x1dV\x00' | nc 127.0.0.1 9100
```

`\x1dV\x00` (`GS V`) cuts the paper, which ends the receipt. On Linux, add `-N` to `nc` so
it closes the connection when the input ends.

## Installation

Download the installer for your system from the
[latest release](https://github.com/AstrOOnauta/thermal-printer-emulator/releases/latest):

| System                | File                                                             |
| --------------------- | ---------------------------------------------------------------- |
| Windows 10/11         | `*_x64-setup.exe`                                                |
| macOS 11+ (Intel/ARM) | `*_universal.dmg`                                                |
| Linux                 | `*.deb` (Debian/Ubuntu), `*.rpm` (Fedora/openSUSE), `*.AppImage` |

The app is free and open source, and it is **not code-signed** (signing certificates are
paid). Your system asks for confirmation the first time you open it:

- **macOS**: open the app once and close the warning. Then go to **System Settings →
  Privacy & Security**, scroll down and click **Open Anyway**. On macOS 14 and older,
  right-click the app and choose **Open** instead.
- **Windows**: on the "Windows protected your PC" screen, click **More info → Run
  anyway**. When the firewall asks, allow access on private networks so other devices
  can print to it.
- **Linux**: open the `.deb`/`.rpm` with your software center. For the AppImage, mark the
  file as executable (Properties → Permissions) before opening it; it needs `libfuse2`.

## Development

### Prerequisites

- Node.js 24+ with Corepack (`corepack enable`), which provides Yarn 4
- Rust via `rustup`. The version is pinned in `rust-toolchain.toml` (with clippy and
  rustfmt) and `rustup toolchain install` fetches it
- The OS prerequisites for Tauri: https://v2.tauri.app/start/prerequisites/
  - macOS: Xcode Command Line Tools
  - Windows: MSVC Build Tools + WebView2
  - Linux: `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev patchelf`

```bash
yarn install
yarn tauri dev
```

### Scripts

| Command            | Description                                             |
| ------------------ | ------------------------------------------------------- |
| `yarn tauri dev`   | Run the desktop app (Vite + Rust, hot reload)           |
| `yarn tauri build` | Build installers for the current OS                     |
| `yarn dev`         | Webview only, in a browser (Tauri APIs are unavailable) |
| `yarn typecheck`   | `tsc --noEmit`                                          |
| `yarn lint`        | ESLint, 0 warnings allowed                              |
| `yarn lint:rust`   | `cargo clippy -D warnings`                              |
| `yarn test`        | Vitest (webview)                                        |
| `yarn test:rust`   | `cargo test`                                            |
| `yarn format`      | Prettier + `cargo fmt`                                  |
| `yarn check`       | Everything CI runs: run it before opening a PR          |

### Releasing

1. Bump `version` in `package.json` (the app reads its version from there).
2. Commit, tag and push: `git tag v0.2.0 && git push --follow-tags`.
3. The `Release` workflow builds the installers and attaches them to a **draft** release.
   Review it on GitHub and publish.

### Translations

The UI ships in English (default), Spanish and Brazilian Portuguese. It follows the OS
language unless one is picked in Settings. Rust resolves it (`src-tauri/src/locale.rs`) and
the webview asks for it, so the window and the native tray menu always match.

- Webview text: `src/shared/translations/`. `en.ts` is the source of truth; the other
  files are typed against it, so a missing key fails `yarn typecheck`.
- Tray and macOS menu labels: the `Strings` tables in `src-tauri/src/locale.rs`.

Adding a language touches a few more places (the language setting, the macOS and Windows
installer lists): [`.ai/conventions/i18n.md`](.ai/conventions/i18n.md) has the checklist.

## Tech stack

- **Tauri v2**: shell, tray, IPC; plugins `single-instance`, `log`, `autostart`, `opener`
- **Rust**: networking and every OS integration
- **React 19 + TypeScript (strict)**: the webview
- **Vite**, **Tailwind CSS v4**, **Vitest**
- **ESLint + Prettier + Husky + lint-staged**: checks on every commit

## Project structure

```
.ai/                      # Project rules, stack, conventions and flows
src/                      # Webview (React)
├── app/                  # Root component, top bar
├── screens/              # receipts (the paper roll), settings (the panel)
├── components/ui/        # Shared primitives and icons
└── shared/               # api (Tauri wrappers), hooks, utils, styles, translations
src-tauri/                # Rust core
├── src/                  # lib.rs (builder), commands.rs, shell.rs (tray, window),
│                         # listener.rs, capture.rs, receipts.rs (TCP → receipts),
│                         # settings.rs, locale.rs, escpos/ (the decoder)
├── tests/                # The listener against real sockets
├── capabilities/         # What the webview may call
├── icons/source/         # Icon source: `yarn tauri icon src-tauri/icons/source/icon.svg`
└── tauri.conf.json
```

## Contributing

Contributions are welcome. Before you start, read [`.ai/rules.md`](.ai/rules.md): it holds
the project's rules for humans and AI agents alike, and links to the stack, conventions
and flows in [`.ai/`](.ai). In short:

- Commits follow [Conventional Commits](https://www.conventionalcommits.org).
- User-facing text ships in English, Spanish and Brazilian Portuguese.
- `yarn check` must pass before you open a pull request.

## License

[MIT](LICENSE)
