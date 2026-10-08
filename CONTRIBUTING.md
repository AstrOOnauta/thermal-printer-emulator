# Contributing

Thanks for helping. This guide covers building, running and changing the project. To use
the app instead, see the [README](README.md).

Before you start, read [`.ai/rules.md`](.ai/rules.md): it holds the project's rules for
humans and AI agents alike, and links to the stack, conventions and flows in
[`.ai/`](.ai). In short:

- Commits follow [Conventional Commits](https://www.conventionalcommits.org).
- User-facing text ships in English, Spanish and Brazilian Portuguese.
- `yarn check` must pass before you open a pull request.

## Prerequisites

- Node.js 24+ with Corepack (`corepack enable`), which provides Yarn 4. Node 25 and later
  no longer bundle Corepack: install it with `npm install -g corepack` first.
- Rust via `rustup`. The version is pinned in `rust-toolchain.toml` (with clippy and
  rustfmt) and `rustup toolchain install` fetches it.
- The OS prerequisites for Tauri: https://v2.tauri.app/start/prerequisites/
  - macOS: Xcode Command Line Tools
  - Windows: MSVC Build Tools + WebView2
  - Linux: `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev patchelf`

## Run it

```bash
git clone https://github.com/AstrOOnauta/thermal-printer-emulator.git
cd thermal-printer-emulator
yarn install
yarn tauri dev
```

Then print to `127.0.0.1:9100` (see the README's quick start).

## Scripts

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

## How it works

```
POS ──TCP 9100──▶ listener ──▶ capture ──▶ ESC/POS decoder ──▶ receipts ──▶ webview
                 (bind, limits,  (receipt per  (parser + printer,  (memory,      (draws on
                  connection      cut)          written in Rust)    limits)       canvas)
                  filter)
```

- **Rust** owns sockets, files and every OS integration. The ESC/POS decoder is written
  from scratch from Epson's public command reference, never panics, and is fuzzed.
- **The webview** (React) only draws what Rust publishes and sends commands. It has no
  file, shell or network access.
- The flows in detail: [`.ai/flows/print-job.md`](.ai/flows/print-job.md),
  [`.ai/flows/app-lifecycle.md`](.ai/flows/app-lifecycle.md),
  [`.ai/flows/settings.md`](.ai/flows/settings.md). The decoder:
  [`.ai/conventions/escpos.md`](.ai/conventions/escpos.md).

## Project structure

```
.ai/                      # Project rules, stack, conventions and flows
.github/                  # CI, release workflow, issue templates, README images
src/                      # Webview (React)
├── app/                  # Root component, top bar
├── screens/              # receipts (the paper roll), settings (the panel)
├── components/ui/        # Shared primitives and icons
└── shared/               # api (Tauri wrappers), hooks, utils, styles, translations
src-tauri/                # Rust core
├── src/                  # lib.rs (builder), commands.rs, shell.rs (tray, window),
│                         # listener.rs, capture.rs, receipts.rs (TCP → receipts),
│                         # settings.rs, locale.rs, updates.rs, escpos/ (the decoder)
├── tests/                # The listener against real sockets
├── capabilities/         # What the webview may call
├── icons/source/         # Icon source: `yarn tauri icon src-tauri/icons/source/icon.svg`
└── tauri.conf.json
```

## Tech stack

- **Tauri v2**: shell, tray, IPC; plugins `single-instance`, `log`, `autostart`, `opener`, `updater`
- **Rust**: networking, the ESC/POS decoder and every OS integration
- **React 19 + TypeScript (strict)**: the webview
- **Vite**, **Tailwind CSS v4**, **Vitest**
- **ESLint + Prettier + Husky + lint-staged**: checks on every commit

## Tests

- Rust: unit tests next to the code, integration tests against real sockets in
  `src-tauri/tests/`, random-bytes tests for the decoder.
- Webview: Vitest for pure logic (`*.test.ts`).
- A bug fix starts with a test that fails without the fix.

## Translations

The UI ships in English (default), Spanish and Brazilian Portuguese. It follows the OS
language unless one is picked in Settings. Rust resolves it (`src-tauri/src/locale.rs`) and
the webview asks for it, so the window and the native tray menu always match.

- Webview text: `src/shared/translations/`. `en.ts` is the source of truth; the other
  files are typed against it, so a missing key fails `yarn typecheck`.
- Tray and macOS menu labels: the `Strings` tables in `src-tauri/src/locale.rs`.

Adding a language touches a few more places (the language setting, the macOS and Windows
installer lists): [`.ai/conventions/i18n.md`](.ai/conventions/i18n.md) has the checklist.

## Reporting bugs

Use the bug report template. The log (tray → **Show logs**) and the receipt's `.bin`
(**Save .bin** on the receipt) usually tell the whole story. Security issues go through
[SECURITY.md](SECURITY.md), not public issues.

## Releasing (maintainers)

1. Bump `version` in `package.json` (the app reads its version from there).
2. Commit, then tag and push the tag: `git tag -a v0.2.0 -m v0.2.0 && git push origin v0.2.0`.
3. The `Release` workflow builds the installers and attaches them to a **draft** release.
   Review it on GitHub and publish.

To try the installers before a release: **Actions → Build → Run workflow**. They come out
as workflow artifacts, unsigned, for the three OSes.
