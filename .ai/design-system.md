# Design system

## Tokens

Defined in `src/shared/styles/globals.css` and exposed to Tailwind v4 through
`@theme inline` (`bg-surface`, `text-muted`, `border-border`, …). **Never** use raw
Tailwind colors like `bg-blue-500`: add a token instead.

The app chrome follows the OS theme (`prefers-color-scheme`). **The receipt paper is
always white**, as real paper is.

| Token                           | Light                             | Dark                              | Use                                     |
| ------------------------------- | --------------------------------- | --------------------------------- | --------------------------------------- |
| `surface`                       | `#f4f4f5`                         | `#18181b`                         | Window background                       |
| `paper`                         | `#ffffff`                         | `#ffffff`                         | Receipt paper (same in both themes)     |
| `ink`                           | `#18181b`                         | `#f4f4f5`                         | Primary text on the surface             |
| `muted`                         | `#71717a`                         | `#a1a1aa`                         | Secondary text                          |
| `border`                        | `#d4d4d8`                         | `#3f3f46`                         | Dividers, input borders                 |
| `accent`                        | `#2563eb`                         | `#60a5fa`                         | Primary actions, focus                  |
| `success` / `warning` / `error` | `#15803d` / `#b45309` / `#b91c1c` | `#4ade80` / `#fbbf24` / `#f87171` | Status (listening / attention / failed) |

Text printed on the paper is black on `paper` in both themes. It does not use `ink`, which
flips in dark mode.

Font: the **system stack** (`--font-sans`). The CSP allows only bundled files and the app
works offline, so no web fonts. The receipt itself will use a bundled or system monospace
font, sized so that 48 columns (font A) fill 80 mm paper (P2).

## Patterns

- `cn()` in `src/shared/styles/cn.ts` (`clsx` + `tailwind-merge`) composes conditional
  classes.
- `src/shared/styles/patterns.ts`: `INTERACTIVE` (hover/active/disabled feedback),
  `BUTTON` (bordered secondary button), `FIELD` (inputs and selects). Use them directly.
- UI primitives go to `src/components/ui/<name>/index.tsx` **on their second use**. There
  are none yet.

## Window

- 680×820 by default, resizable, minimum 640×480, centered. The width fits 80 mm paper
  (576 dots) plus margins.
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
- **Slices**: receipts taller than 4096 dots are drawn on several canvases
  (`receipt-layout.ts`), under the canvas size limits of WebKit and Chromium.
- Ink is always `#000` on the white `paper` token; it never follows the theme.
- Cut receipts end with `.paper-cut` (globals.css), a row of paper-colored teeth.
