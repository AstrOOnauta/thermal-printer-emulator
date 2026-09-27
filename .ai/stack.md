# Stack & architecture

Thermal printer emulator, also searched for as **virtual thermal printer**: a desktop
app that emulates an ESC/POS **network** receipt printer: point-of-sale software
sends raw bytes to TCP port 9100 and the app shows the receipt. It is installed like any
app and never needs a terminal. The idea comes from
[virtual-thermal-printer](https://github.com/FilipChalupa/virtual-thermal-printer), a
web-based emulator; **no code is taken from it** (see `rules.md` § ESC/POS decoder).

## Tech stack

| Category          | Technology                                                                                            |
| ----------------- | ----------------------------------------------------------------------------------------------------- |
| Shell             | **Tauri v2** (2.12): tray + one window, single process                                                |
| Core              | **Rust** (edition 2021), toolchain pinned in `rust-toolchain.toml`: networking and OS integration     |
| Tauri plugins     | `single-instance`, `log` (rotating files), `autostart` (`--autostart` arg), `opener` (Rust side only) |
| Rust crates       | `tokio` (sockets, timers; Tauri's runtime), `sys-locale` (OS language), `serde`, `log`                |
| Webview           | **React 19** + **TypeScript 6** (strict, `noUncheckedIndexedAccess`)                                  |
| Bundler           | **Vite 8** (dev server on fixed port 1420)                                                            |
| UI                | **Tailwind CSS v4** via `@tailwindcss/vite`, neutral tokens that follow the OS theme                  |
| Class composition | `clsx` + `tailwind-merge` via `cn()`                                                                  |
| i18n              | Typed dictionaries (en, es, pt-BR) + `t()`, no library; the locale comes from Rust                    |
| Tests             | **Vitest** (webview, colocated `*.test.ts`), `cargo test` (Rust)                                      |
| Quality           | ESLint 9 flat config + Prettier + Husky + lint-staged; `cargo fmt` + `clippy -D warnings`             |
| Package manager   | **Yarn 4** via Corepack, `nodeLinker: node-modules`                                                   |

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

## Phases

- **P0 Skeleton: done.** Tauri app, tray (Open / Launch at login / Show logs / Quit),
  hide-on-close window, Dock only while the window is open, single instance, rotating logs,
  autostart, capability lockdown, CSP, en/es/pt-BR, CI, release workflow, Dependabot,
  provisional icon.
- **P1 Listener: next.** TCP 9100 in Rust (`0.0.0.0`), connection filter, limits, jobs kept
  in memory and pushed to the webview, tray status, port-in-use error. The window shows a
  temporary list of received jobs (time, peer, size).
- **P2 Decoder + receipt: planned.** Our own ESC/POS parser and printer state machine in
  Rust, emitting a print model (lines, images, cuts) that the webview draws on a canvas at
  1:1 dots; code pages, raster images, QR and barcodes, status replies, receipt split on
  cut, 58/80 mm paper.
- **P3 App: planned.** "Point your POS at `IP:9100`" panel, test receipt (tray + window),
  settings (port, LAN or local only, paper width, default code page, sound) in a JSON file
  written by Rust, history (clear, export raw `.bin`).
- **P4 Distribution: planned.** NSIS `perMachine` (UAC at install) with a Windows Firewall
  rule for private and domain networks only, removed on uninstall; `tauri-plugin-updater`
  (Tauri's own signing key, no certificate; Windows, macOS and AppImage; `.deb`/`.rpm`
  only get a "new version" link); screenshots; release checklist.

## Decisions

Made on 2026-10-06. Changing one is a product decision: update this list.

1. **Receipt boundary**: a receipt ends at a cut command (`GS V`) or when the connection
   closes, whichever comes first. A connection is only transport: some POS send many
   receipts over one. Until the parser exists (P1), the connection close is the only
   boundary.
2. **Bind address**: `0.0.0.0:9100` by default (POS terminals on other machines). "Local
   only" (`127.0.0.1`) becomes a setting in P3.
3. **History**: in memory only, newest 100 receipts or 32 MB, oldest dropped first. Rust
   keeps the raw bytes of each receipt (for export).
4. **Default code page** (no `ESC t` received): **CP437**, Epson's factory default.
   Configurable in P3: Brazilian printers (Elgin, Bematech) usually ship with CP850 and
   some POS never send `ESC t`.
5. **Status replies**: answer `DLE EOT n`, `GS r n` and `GS I n` as an online printer with
   paper (P2, once the parser knows command boundaries). The connection filter accepts a
   first byte of `ESC @`, `DLE` or `GS`.
6. **Decoder**: written from scratch in **Rust** from Epson's public ESC/POS command
   reference. Rust interprets, the webview only draws.
7. **Windows installer** (P4): asks for admin, so it can add the firewall rule. Trade-off
   accepted: every update shows UAC, and users without admin rights cannot install.

## Top-level layout

```
.ai/                         # AI + contributor context (this folder); AGENTS.md / CLAUDE.md point here
src/                         # Webview (React)
  main.tsx                   # initLocale() → createRoot
  app/index.tsx              # root component
  shared/
    api/app.ts               # the ONLY @tauri-apps/api imports (typed invoke wrappers)
    hooks/use-translation.ts # t(), setLocale, initLocale (+ test)
    styles/                  # globals.css (tokens), cn.ts
    translations/            # en.ts (source of truth), es.ts, pt-BR.ts
src-tauri/                   # Rust core
  src/main.rs                # entry (calls lib::run)
  src/lib.rs                 # Builder: plugins, setup, window events, handlers
  src/ui.rs                  # commands, tray, app menu, window show/hide
  src/locale.rs              # OS language → Locale, native menu labels
  build.rs                   # app command manifest (permissions)
  capabilities/main.json     # what the `main` window may call
  tauri.conf.json            # window, CSP, bundle targets, version source
  Info.plist                 # macOS: LSUIElement, localizations, Local Network string
  icons/                     # generated; source in icons/source/icon.svg
.github/workflows/           # ci.yml, release.yml
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
| A screen                  | `src/screens/<name>/index.tsx`                                                                                       |
| Widget used by one screen | `src/screens/<name>/<widget>/index.tsx`                                                                              |
| Shared UI primitive       | `src/components/ui/<name>/index.tsx`, created on its **second** use                                                  |
| User-facing text          | key in `src/shared/translations/en.ts`, then `es.ts` and `pt-BR.ts`                                                  |
| Native menu label         | `Strings` tables in `src-tauri/src/locale.rs`                                                                        |
| Pure helper               | `src/shared/utils/<name>.ts` + `<name>.test.ts`                                                                      |
