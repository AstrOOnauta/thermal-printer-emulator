# Design system

## Tokens

Defined in `src/shared/styles/globals.css` and exposed to Tailwind v4 through
`@theme inline` (`bg-surface`, `text-muted`, `border-border`, …). **Never** use raw
Tailwind colors like `bg-blue-500`: add a token instead.

The app chrome follows the OS theme (`prefers-color-scheme`). **The receipt paper is
always white**, as real paper is.

| Token                           | Light                             | Dark                              | Use                                          |
| ------------------------------- | --------------------------------- | --------------------------------- | -------------------------------------------- |
| `surface`                       | `#f4f4f5`                         | `#18181b`                         | Window background                            |
| `raised`                        | `#ffffff`                         | `#222226`                         | Panels over the surface (settings)           |
| `paper`                         | `#ffffff`                         | `#ffffff`                         | Receipt paper (same in both themes)          |
| `ink`                           | `#18181b`                         | `#f4f4f5`                         | Primary text on the surface                  |
| `muted`                         | `#6b6b74`                         | `#a1a1aa`                         | Secondary text                               |
| `border`                        | `#d4d4d8`                         | `#3f3f46`                         | Dividers                                     |
| `field-border`                  | `#8a8a93`                         | `#71717a`                         | Input and select outlines (3:1, WCAG 1.4.11) |
| `accent`                        | `#2563eb`                         | `#60a5fa`                         | Primary actions, focus                       |
| `success` / `warning` / `error` | `#15803d` / `#b45309` / `#b91c1c` | `#4ade80` / `#fbbf24` / `#f87171` | Status (listening / attention / failed)      |

Text printed on the paper is black on `paper` in both themes. It does not use `ink`, which
flips in dark mode.

Font: the **system stack** (`--font-sans`). The CSP allows only bundled files and the app
works offline, so no web fonts. The receipt uses the system monospace font, stretched to
the printer's cells (see Receipt rendering below).

## Patterns

- `cn()` in `src/shared/styles/cn.ts` (`clsx` + `tailwind-merge`) composes conditional
  classes.
- `src/shared/styles/patterns.ts`: `INTERACTIVE` (hover/active/disabled feedback),
  `BUTTON` (bordered secondary button), `FIELD` (inputs and selects). Use them directly.
- UI primitives go to `src/components/ui/<name>/index.tsx` **on their second use**:
  `icon-button`, `icons`.

## Window

- 680×820 by default, resizable, minimum 640×480, centered. The receipt list has 16 px
  side padding, so 80 mm paper (576 dots + 2 × 16 margin) fits exactly at the minimum width.
- **One top bar** (`App`): on the left the status and the address in one line
  (`StatusBar`: dot + "Point your POS at `ip:port`" + Copy, or "Port N is in use"); on the
  right Print test receipt and Clear (when there are receipts) and the settings gear. The
  window title already names the app: no in-app title or tagline. While the port fails,
  the hint banner sits under the bar.
- **Settings is a panel**, not a screen: 22 rem wide, sliding in from the right over the
  receipts (`translate`, 200 ms, `ease-out-quart`, none with reduced motion), closed by Esc
  or ✕, focus moves to ✕ on open and back to the gear on close. Closed, it is `inert`
  (skipped by Tab and screen readers) and parked off-screen inside an `overflow-hidden`
  area (otherwise it adds a horizontal scrollbar).
- **Confirmations** of destructive actions use a native `<dialog>` with `showModal()`
  (focus trap, Esc, backdrop `bg-black/40`), `bg-raised`, the safe button focused first and
  the destructive one in `bg-error`. Nothing else is modal.
- **Layers** (`--z-*` in globals.css, lowest first): `--z-pill` 10, `--z-panel` 20. Use
  `z-(--z-name)`, never a raw number.
- **Motion curves**: `ease-out-quart` (`cubic-bezier(0.25, 1, 0.5, 1)`) for state changes
  (the panel); `--ease-out-cubic` (`cubic-bezier(0.33, 1, 0.68, 1)`) for the paper feed,
  mirrored by `easeOutCubic` in `paper-feed.ts`. No bounce. Motion means something
  happened: the panel opening, a receipt printing. Every animation has a
  `prefers-reduced-motion` path (none, or a short fade).
- **The paper feed** is the one signature motion: a new receipt is revealed top to bottom
  while the list scrolls with it at printer speed (`flows/print-job.md` § Webview).
- **Receipt header**: as wide as the paper. Facts on the left in muted text, separated
  by `·` (only the time in ink); actions on the right: muted icon buttons (copy text, save
  .bin) and one accent text toggle (Show commands, with a chevron). An action confirms in
  place (`ReceiptAction`: the icon becomes a green check for 1.5 s, announced to screen
  readers), so nothing moves; only an error shows as text, below the header. Toolbar
  actions are bordered buttons or icon buttons; shortcuts appear in tooltips
  (`IconButton`'s `shortcut`).
- **Icons**: `src/components/ui/icons/` (drawn for this app: 24×24, `currentColor`, 1.75
  stroke; gear, trash, close) inside `IconButton` (`src/components/ui/icon-button/`: square,
  `label` is both the tooltip and the accessible name, highlighted while `aria-expanded`).
- A desktop tool, not a page: `-webkit-user-select` + `user-select: none` on the chrome
  (WebKit ignores the unprefixed one, and ⌘A would select the whole UI); inputs and
  `select-text` elements (the address to copy) turn it back on. No overscroll bounce.
- Status uses a colored dot **plus** text, never color alone.

## Receipt rendering (`src/shared/utils/draw-receipt.ts`)

- **One dot = one CSS pixel**: an 80 mm receipt is 576 px wide, inside 16 px of paper
  margin. Canvases are sized at `devicePixelRatio`, so text and bitmaps stay sharp.
- **Text**: each character is drawn in its own printer cell (font A 12×24, font B 9×17
  dots, times the size multipliers): the system monospace glyph (`ui-monospace`, SF Mono,
  Menlo, Consolas, Liberation/DejaVu Mono) is stretched to the cell, so columns line up
  whatever font the OS has. Glyph bottoms sit on the line's `ascent`. Bold uses weight 700;
  reverse paints the cell black and the glyph white; underline is a 1 or 2 dot bar.
- **Bitmaps** go through an offscreen canvas (`ImageData`, black or transparent) and are
  drawn with smoothing off.
- **Slices**: a receipt is drawn on canvases of at most 4096 device pixels each
  (`sliceHeight` in `receipt-layout.ts`), under the canvas size limits of WebKit and
  Chromium; only slices near the view hold pixels (`flows/print-job.md` § Receipts in the
  window).
- Ink is always `#000` on the white `paper` token; it never follows the theme.
- Cut receipts end with `.paper-cut` (globals.css), a row of paper-colored teeth.
