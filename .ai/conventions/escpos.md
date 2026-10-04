# ESC/POS decoder

Our own decoder, written from Epson's public ESC/POS command reference (rules in
`rules.md` § ESC/POS decoder). Code in `src-tauri/src/escpos/`.

## Parser (`parser.rs`)

Bytes → `Command`. `Parser::feed(bytes, emit)` is streaming: an unfinished command stays
in `pending` until the rest arrives, so any split of the stream parses the same (tested
at every byte boundary). `emit` gets each command with the **stream offset right after
it**, which lets the caller cut the raw bytes exactly at a command (receipt split on cut).

Guarantees:

- **No panic, no stall** on any input (2,000 random streams fed in random chunks).
- **No allocation beyond the input**: length fields only say how many bytes to wait for.
  The connection's 16 MB limit bounds `pending`.
- **Alignment**: every command in the reference has a known length, including the ones
  the emulator ignores. An unknown `ESC x` / `GS x` / `FS x` skips two bytes
  (`Command::Unknown`); a lone `DLE` skips one, as the printer does.

| Bytes                                                                              | Command                                              |
| ---------------------------------------------------------------------------------- | ---------------------------------------------------- |
| `0x20–0x7E`, `0x80–0xFF` runs                                                      | `Text` (still in the selected code page)             |
| `LF`, `CR`, `HT`                                                                   | `LineFeed`, `CarriageReturn`, `Tab`                  |
| `ESC @`                                                                            | `Initialize`                                         |
| `ESC !`, `ESC E`/`ESC G`, `ESC -`, `ESC M`, `GS !`, `GS B`                         | print mode, emphasis, underline, font, size, reverse |
| `ESC a`, `ESC SP`, `ESC 2`/`ESC 3`, `GS L`, `GS W`, `ESC $`, `ESC \`, `ESC D`      | alignment, spacing, margins, positions, tab stops    |
| `ESC J`, `ESC d`                                                                   | feed dots / lines                                    |
| `ESC t`                                                                            | code table                                           |
| `GS V`, `ESC i`, `ESC m`                                                           | `Cut { partial, feed }`                              |
| `ESC p`, `DLE DC4 1`                                                               | `DrawerPulse`                                        |
| `ESC B n t`                                                                        | `Beep` (buzzer on many ESC/POS printers)             |
| `DLE EOT n`, `GS r n`, `GS I n`                                                    | `Status(..)`                                         |
| Raster/bit images, QR (`GS ( k`), barcodes (`GS k`), graphics (`GS ( L`, `GS 8 L`) | skipped by length for now (P2 commits add them)      |
| Page mode, NV/user images, macros, Kanji, sensors, counters                        | `Ignored` (skipped by length)                        |

Other control bytes (`0x00–0x1F`, `0x7F`) are `Ignored`, one byte each.

## Code pages (`codepage.rs`)

| `ESC t n` | Table | `ESC t n` | Table   |
| --------- | ----- | --------- | ------- |
| 0         | CP437 | 16        | WPC1252 |
| 2         | CP850 | 17        | CP866   |
| 3         | CP860 | 18        | CP852   |
| 4         | CP863 | 19        | CP858   |
| 5         | CP865 |           |         |

- Default **CP437** until `ESC t` (decision 4); `ESC @` goes back to it. P3 makes the
  default configurable (Brazilian printers often ship with CP850).
- An unsupported `n` (Katakana, Thai, …) keeps the current table, as the printer does.
- Bytes `0x20–0x7E` are ASCII in every table. The upper half is generated:
  `python3 scripts/codepages.py > src/escpos/codepage_tables.rs` (from `src-tauri/`), from
  Python's codecs, which are built from the Unicode Consortium's mapping files. Never edit
  the generated file by hand; `#[rustfmt::skip]` keeps it identical to the script output.
- Bytes a table leaves undefined (5 in WPC1252) decode to U+FFFD.

## Printer (`printer.rs`)

`Printer::apply(command, &mut out)` updates the state and pushes `Output`s:
`Block(Block)`, `Cut { partial }`, `DrawerPulse`, `Beep`. `finish` prints what is left.

**Geometry** (a 203 dpi printer, all in dots): 80 mm paper = 576, 58 mm = 384
(`Paper`); font A 12×24, font B 9×17 (font C prints as B); default line spacing 30;
default tab stops every 8 columns. 48 columns of font A (64 of font B) fill 80 mm.

**The print model** (what the webview draws, serialized as snake_case JSON):

```
Block::Text { height, ascent, segments: [Segment] }   one printed line
Block::Feed { height }                                blank paper (consecutive feeds merge)
Segment { x, text, font: a|b, width, height, advance, bold, underline: 0|1|2, reverse }
```

Character `i` of a segment sits at `x + i × advance` (dots from the printable area's left
edge); its glyph fills `cell × (width, height)` and is bottom-aligned at the line's
`ascent`. A style change mid-line opens a new segment on the **same** line.

**Rules taken from the Epson reference:**

- `ESC a`, `GS L` and `GS W` take effect at the start of a line: a change mid-line applies
  from the next one.
- A line is `max(line spacing, tallest character)` tall; `ESC J n` uses `n` instead of
  the line spacing.
- `ESC d n` after text: the printed line counts as the first of the n lines.
- `ESC SP` spacing is multiplied by the width multiplier, like the character.
- A character that does not fit wraps to the next line.
- `ESC @` resets every setting (code page included) and drops the unprinted line.
- `CR` does nothing (automatic line feed off, the default).
- `HT` jumps to the next tab stop in the current character width; past the last stop it
  does nothing. `ESC D` stops must ascend.
- `GS V n` feeds `n` dots, then cuts.

**On purpose, not like the printer:** text without a final `LF` is printed when the job
ends. The printer would keep it in its buffer, and an emulator that hides it would hide a
bug from the developer.

Not emulated: page mode, upside-down and 90° rotation, user-defined characters,
international character sets (`ESC R`), Kanji.
