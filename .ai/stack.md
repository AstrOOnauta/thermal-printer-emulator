# Stack & architecture

Thermal printer emulator, also searched for as **virtual thermal printer**: a desktop
app that emulates an ESC/POS **network** receipt printer. Point-of-sale software sends raw
bytes to TCP port 9100 and the app draws the receipt. It is installed like any app and
never needs a terminal. The idea comes from
[virtual-thermal-printer](https://github.com/FilipChalupa/virtual-thermal-printer), a
web-based emulator; **no code is taken from it** (see `rules.md` § ESC/POS decoder).

## How it works

```
POS ─TCP 9100─▶ listener ─▶ capture ─▶ decoder ─▶ receipts ─event─▶ webview (canvas)
                (Rust)      (one       (parser +   (memory,          draws what Rust
                            connection) printer)   limits)           publishes
```

- **Rust owns everything that matters**: the socket, decoding, receipts in memory,
  settings, the tray. The **webview only draws** what Rust publishes and sends commands.
- **Listener** (`listener.rs`): binds the configured port, retries while it is taken,
  filters non-ESC/POS connections, one task per connection, limits. → `flows/print-job.md`
- **Capture** (`capture.rs`): turns one connection's bytes into receipts, split at cuts.
- **Decoder** (`escpos/`): our own ESC/POS parser and printer state machine, producing a
  print model laid out in printer dots. → `conventions/escpos.md`
- **Receipts** (`receipts.rs`): the history in memory, bounded.
- **Webview**: receipts on paper, the status, the address to print to, settings.
  → `conventions/bridge.md`, `design-system.md`
- **Settings** (`settings.rs`): a JSON file, applied without a restart.
  → `flows/settings.md`
- **App shell**: tray, window, autostart, logs, language. → `flows/app-lifecycle.md`,
  `conventions/i18n.md`

## Features

**Printing**

- Listens on TCP 9100 (configurable), on every network or only on this computer.
- Accepts connections that open like ESC/POS (`ESC @`, `DLE`, `GS`); turns away port
  scanners and other protocols.
- Answers status requests (`DLE EOT`, `GS r`, `GS I`) as an online printer with paper.
- Decodes text styles (fonts A/B, sizes 1–8, bold, underline, reverse, alignment, margins,
  tabs), 9 code pages, raster and column images, graphics, QR codes and 7 barcode
  symbologies, cuts, the cash drawer and the beep.

**Receipts**

- Drawn on paper at the printer's scale (576 dots for 80 mm, 384 for 58 mm), sharp on
  Retina screens; only the ones near the visible area are drawn.
- One receipt per cut; status-only connections leave nothing in the list.
- History in memory: clear it, or save a receipt's raw bytes as `.bin`.
- Badges for the cash drawer and the beep; the beep can play a sound.

**App**

- The status (listening / port in use / blocked) in the window and the tray; recovers by
  itself when the port frees up.
- "Point your POS at `ip:port`" with a copy button.
- A test receipt, from the tray or the window, sent through the real socket.
- Settings: port, who can print, paper width, default code page, sound, language.
- English, Spanish and Brazilian Portuguese, following the OS or chosen in settings.
- Runs from the tray, starts at login if asked, one instance only, rotating logs.

## Next

- **Windows installer with admin** (NSIS `perMachine`, UAC at install) that adds a Windows
  Firewall rule for private and domain networks only, removed on uninstall.
- **Automatic updates** (`tauri-plugin-updater`, signed with Tauri's own key, no paid
  certificate): Windows, macOS and AppImage; `.deb`/`.rpm` only get a "new version" link.
- Screenshots for the README, a release checklist, the final icon.

## Product decisions

Changing one is a product decision: update this list.

1. **Receipt boundary**: a receipt ends at a cut command or when the connection closes,
   whichever comes first. A connection is only transport: some POS send many receipts over
   one.
2. **Bind address**: every interface by default (POS terminals on other machines); "only
   this computer" (`127.0.0.1`) is a setting.
3. **History**: in memory only, newest 100 receipts within 32 MB (raw bytes and print model),
   oldest dropped first. Rust keeps each receipt's raw bytes for export.
4. **Default code page** (no `ESC t` received): **CP437**, Epson's factory default, and a
   setting: Brazilian printers (Elgin, Bematech) usually ship with CP850 and some POS never
   send `ESC t`.
5. **Status replies**: answer `DLE EOT n`, `GS r n` and `GS I n` as an online printer with
   paper, with an honest identity (not a real printer model). The connection filter accepts
   a first byte of `ESC @`, `DLE` or `GS`.
6. **Decoder**: written from scratch in **Rust** from Epson's public ESC/POS command
   reference. Rust interprets, the webview only draws.
7. **Windows installer**: asks for admin, so it can add the firewall rule. Trade-off
   accepted: every update shows UAC, and users without admin rights cannot install.

## Tech stack

| Category          | Technology                                                                                                                                                             |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Shell             | **Tauri v2** (2.12): tray + one window, single process                                                                                                                 |
| Core              | **Rust** (edition 2021), toolchain pinned in `rust-toolchain.toml`                                                                                                     |
| Tauri plugins     | `single-instance`, `log` (rotating files), `autostart` (`--autostart` arg), `opener` (Rust side only)                                                                  |
| Rust crates       | `tokio` (sockets, timers; Tauri's runtime), `qrcode` (QR encoding, no default features), `base64` (bitmaps to the webview), `sys-locale` (OS language), `serde`, `log` |
| Webview           | **React 19** + **TypeScript 6** (strict, `noUncheckedIndexedAccess`)                                                                                                   |
| Bundler           | **Vite 8** (dev server on fixed port 1420)                                                                                                                             |
| UI                | **Tailwind CSS v4** via `@tailwindcss/vite`, neutral tokens that follow the OS theme                                                                                   |
| Class composition | `clsx` + `tailwind-merge` via `cn()`                                                                                                                                   |
| i18n              | Typed dictionaries (en, es, pt-BR) + `t()`, no library; the locale comes from Rust                                                                                     |
| Tests             | **Vitest** (webview, colocated `*.test.ts`), `cargo test` (Rust)                                                                                                       |
| Quality           | ESLint 9 flat config + Prettier + Husky + lint-staged; `cargo fmt` + `clippy -D warnings`                                                                              |
| Package manager   | **Yarn 4** via Corepack, `nodeLinker: node-modules`                                                                                                                    |

Husky is installed from `postinstall`, not `prepare`: Yarn 2+ does not run `prepare` on
install. That is safe because the package is `private` and never published.

## CI and releases

- `ci.yml`, on every push to `main` and every PR:
  - `checks`: `yarn check` on Ubuntu 22.04;
  - `platforms`: `cargo clippy -D warnings` + `cargo test` on macOS and Windows, because
    `checks` never compiles `#[cfg(target_os = "macos")]` / `#[cfg(windows)]` code;
  - `build` (unsigned installers for the three OSes, as workflow artifacts) runs **only**
    from "Run workflow" (`workflow_dispatch`): a macOS bundle takes ~15 min and would hold
    up every PR.
- `release.yml`, on a `v*` tag: fails unless the tag equals `v` + the `package.json`
  version, then `tauri-action` builds the installers and attaches them to a **draft**
  GitHub Release, which is published by hand.
- Actions are on v7 (`checkout`, `setup-node`, `upload-artifact`); `rust-cache` keeps the
  cache on failure, so a red run does not make the next one start cold.
- **Dependabot** (`.github/dependabot.yml`): weekly; minor + patch grouped into one PR per
  ecosystem (npm, cargo, actions), majors alone; titles `build(deps): …` / `ci(deps): …`.
  The Tauri npm packages and crates must stay on matching versions: merge their PRs
  together. `rust-toolchain.toml` is bumped by hand.
- **Version**: `package.json` is the single source (`tauri.conf.json` has
  `"version": "../package.json"`). The crate version in `Cargo.toml` is not shown anywhere.
- **No code signing** (open source, no paid certificates). macOS gets an **ad-hoc**
  signature (`signingIdentity: "-"`): an unsigned app downloaded from the internet is
  reported as "damaged" and can only be fixed from a terminal, while an ad-hoc one gets
  "Open Anyway" in System Settings. Windows shows SmartScreen's "Run anyway". The README
  explains both. CI proves the build, not this first-launch experience: check it by hand
  on real machines before the first public release.

## Layout

```
.ai/                         # AI + contributor context (this folder); AGENTS.md / CLAUDE.md point here
src/                         # Webview (React)
  main.tsx                   # initLocale() → createRoot
  app/index.tsx              # header (status, settings button), failure hint, address bar, screen
  app/listener-status/       # status badge, failure hint
  app/connection-bar/        # "Point your POS at ip:port" + copy
  screens/receipts/          # receipts on paper: receipt-card/, receipt-paper/, toolbar buttons
  screens/settings/          # settings form (saved and applied at once)
  shared/
    api/                     # the ONLY @tauri-apps/api imports: app.ts, emulator.ts, settings.ts
    interfaces/emulator.ts   # bridge types (mirror of the Rust structs)
    hooks/                   # use-translation, use-synced (event + read), use-near-viewport
    utils/                   # format, receipt-layout, ui-error, beep (+ tests), draw-receipt (canvas)
    styles/                  # globals.css (tokens), cn.ts, patterns.ts (BUTTON, FIELD)
    translations/            # en.ts (source of truth), es.ts, pt-BR.ts
src-tauri/                   # Rust core
  src/main.rs                # entry (calls lib::run)
  src/lib.rs                 # Builder: plugins, setup, window events, handlers
  src/ui.rs                  # commands, tray, app menu, window show/hide, listener restart
  src/locale.rs              # language setting + OS language → Locale, native menu labels
  src/listener.rs            # TCP: bind, accept, connection filter, one task per connection
  src/capture.rs             # one connection → receipts (split on cut)
  src/receipts.rs            # receipts in memory, limits
  src/settings.rs            # settings.json: defaults, validation, load/save
  src/network.rs             # LAN IPv4 for "point your POS at…"
  src/test_receipt.rs        # sample receipt sent to our own port
  src/escpos/                # our ESC/POS decoder: parser, codepage, printer, bitmap, barcode
  scripts/codepages.py       # generates escpos/codepage_tables.rs
  tests/listener.rs          # the listener against real sockets
  build.rs                   # app command manifest (permissions)
  capabilities/main.json     # what the `main` window may call
  tauri.conf.json            # window, CSP, bundle targets, version source
  Info.plist                 # macOS: LSUIElement, localizations, Local Network string
  icons/                     # generated; source in icons/source/icon.svg
.github/                     # workflows (ci.yml, release.yml), dependabot.yml
rust-toolchain.toml          # pinned Rust; bump it on purpose, in its own commit
```

## App icons

`src-tauri/icons/source/icon.svg` is the only source (provisional: a receipt coming out of
a printer, on the Apple grid with an 824 px body at 100 px on a 1024 canvas, so it also
fills the macOS squircle). Regenerate every icon with
`yarn tauri icon src-tauri/icons/source/icon.svg`, then delete the `android/` and `ios/`
folders it creates (desktop only).

## Where things live

| New thing                 | Where                                                                                                                |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Networking / OS logic     | Rust, one module per responsibility (`conventions/rust-core.md`)                                                     |
| A command for the UI      | `ui.rs` + `build.rs` + `capabilities/main.json` + `lib.rs` handler + `src/shared/api/` (see `conventions/bridge.md`) |
| A setting                 | `settings.rs` + `ISettings` + the settings screen (see `flows/settings.md`)                                          |
| A screen                  | `src/screens/<name>/index.tsx`                                                                                       |
| Widget used by one screen | `src/screens/<name>/<widget>/index.tsx`                                                                              |
| Shared UI primitive       | `src/components/ui/<name>/index.tsx`, created on its **second** use                                                  |
| User-facing text          | key in `src/shared/translations/en.ts`, then `es.ts` and `pt-BR.ts`                                                  |
| Native menu label         | `Strings` tables in `src-tauri/src/locale.rs`                                                                        |
| Pure helper               | `src/shared/utils/<name>.ts` + `<name>.test.ts`                                                                      |
