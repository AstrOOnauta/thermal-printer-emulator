# ESC/POS decoder

Our own decoder, written from Epson's public ESC/POS command reference (rules in
`rules.md` § ESC/POS decoder). Code in `src-tauri/src/escpos/`.

`Decoder` (`mod.rs`) is a parser plus a printer for one connection: `feed(bytes, out)`
pushes each `Output` with the stream offset right after the command that produced it;
`capture.rs` uses those offsets to split receipts at cuts (`flows/print-job.md`).

## Parser (`parser.rs`, `command.rs`)

Bytes → `Command` (the types live in `command.rs`; tests in `parser/tests.rs`). `Parser::feed(bytes, emit)` is streaming: an unfinished command stays
in `pending` until the rest arrives, so any split of the stream parses the same (tested
at every byte boundary). `emit` gets each command with the **stream offset right after
it**, which lets the caller cut the raw bytes exactly at a command (receipt split on cut).

Guarantees:

- **No panic, no stall** on any input (2,000 random streams fed in random chunks).
- **No allocation beyond the input**: length fields only say how many bytes to wait for,
  and an image header is checked against the data that came with it (`GS ( L` with less
  data than `ceil(x / 8) × y` is ignored). The per-receipt 16 MB limit bounds `pending`.
- **No quadratic scans**: commands that end at a NUL look at a bounded window (`ESC D` 33
  bytes, `GS k` function A 256: past it, `GS k m` is skipped and the rest is data).
- **Alignment**: every command in the reference has a known length, including the ones
  the emulator ignores, each tested by name (`every_skipped_command_has_its_documented_length`).
  An unknown `ESC x` / `GS x` / `FS x` skips two bytes (`Command::Unknown`); a lone `DLE`
  skips one, as the printer does.
- **Out of range is ignored**, as the reference says: the command keeps the previous
  setting instead of being clamped (`ESC a 3`, `ESC - 3`, `GS !` with bit 3 or 7, `GS h 0`,
  `GS w 0`, a QR size outside 1–16 or a level outside 48–51).

| Bytes                                                                         | Command                                                                              |
| ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `0x20–0x7E`, `0x80–0xFF` runs                                                 | `Text` (still in the selected code page)                                             |
| `LF`, `CR`, `HT`                                                              | `LineFeed`, `CarriageReturn`, `Tab`                                                  |
| `ESC @`                                                                       | `Initialize`                                                                         |
| `ESC !`, `ESC E`/`ESC G`, `ESC -`, `ESC M`, `GS !`, `GS B`                    | print mode, emphasis, underline, font, size, reverse                                 |
| `ESC a`, `ESC SP`, `ESC 2`/`ESC 3`, `GS L`, `GS W`, `ESC $`, `ESC \`, `ESC D` | alignment, spacing, margins, positions, tab stops                                    |
| `ESC J`, `ESC d`                                                              | feed dots / lines                                                                    |
| `ESC t`                                                                       | code table                                                                           |
| `GS V`, `ESC i`, `ESC m`                                                      | `Cut { partial, feed }`                                                              |
| `ESC p`, `DLE DC4 1`                                                          | `DrawerPulse`                                                                        |
| `ESC B n t`                                                                   | `Beep` (buzzer on many ESC/POS printers)                                             |
| `DLE EOT n`, `GS r n`, `GS I n`                                               | `Status(..)`                                                                         |
| `GS v 0`, `ESC *`, `GS ( L` / `GS 8 L` (functions 112, 50)                    | `Raster`, `BitImage`, `StoreGraphics`, `PrintGraphics`                               |
| `GS h`, `GS w`, `GS H`, `GS f`, `GS k`                                        | barcode height, module width, HRI position/font, `Barcode`                           |
| `GS ( k` cn 49, fn 67/69/80/81                                                | `QrModuleSize`, `QrErrorCorrection`, `QrStore`, `QrPrint` (other 2D codes `Ignored`) |
| Page mode, NV/user images, macros, Kanji, sensors, counters                   | `Ignored` (skipped by length)                                                        |

Other control bytes (`0x00–0x1F`, `0x7F`) are `Ignored`, one byte each.

## Code pages (`codepage.rs`)

| `ESC t n` | Table | `ESC t n` | Table   |
| --------- | ----- | --------- | ------- |
| 0         | CP437 | 16        | WPC1252 |
| 2         | CP850 | 17        | CP866   |
| 3         | CP860 | 18        | CP852   |
| 4         | CP863 | 19        | CP858   |
| 5         | CP865 |           |         |

- Default **CP437** until `ESC t` (decision 4), configurable in settings (Brazilian
  printers often ship with CP850). `ESC @` goes back to the configured default.
- An unsupported `n` (Katakana, Thai, …) keeps the current table, as the printer does.
- Bytes `0x20–0x7E` are ASCII in every table. The upper half is generated:
  `python3 scripts/codepages.py > src/escpos/codepage_tables.rs` (from `src-tauri/`), from
  Python's codecs, which are built from the Unicode Consortium's mapping files. Never edit
  the generated file by hand; `#[rustfmt::skip]` keeps it identical to the script output.
- Bytes a table leaves undefined (5 in WPC1252) decode to U+FFFD.
- `CodePage::encode(char)` goes the other way (test receipt); a character the table lacks
  becomes `?`. `CodePage::name` gives `CP437`, `WPC1252`…

## Printer (`printer.rs`, `model.rs`)

Barcode and QR printing is in `printer/codes.rs`, the print model types in `model.rs`, the
tests in `printer/tests.rs`. Streaming end to end: 1,000 random streams decode to the same
outputs, cut at the same byte, whether fed whole, byte by byte or in random chunks
(`escpos::tests`).

`Printer::apply(command, &mut out)` updates the state and pushes `Output`s:
`Block(Block)`, `Cut { partial }`, `DrawerPulse`, `Beep`. `finish` prints what is left.

**Geometry** (a 203 dpi printer, all in dots): 80 mm paper = 576, 58 mm = 384
(`Paper`); font A 12×24, font B 9×17 (font C prints as B); default line spacing 30;
default tab stops every 8 columns. 48 columns of font A (64 of font B) fill 80 mm.

**The print model** (what the webview draws, serialized as snake_case JSON):

```
Block::Line  { height, ascent, segments: [Segment], images?: [Placed] }   one printed line
Block::Image (Placed)                                                     an image band
Block::Feed  { height }                                    blank paper (consecutive feeds merge)
Segment { x, text, font: a|b, width, height, advance, bold, underline: 0|1|2, reverse }
Placed  { x, width, height, data }   1-bit rows, MSB leftmost, 1 = black, base64
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
- A character that does not fit wraps to the next line, also when only the position moved
  (`ESC $`, `HT`, an `ESC *` stripe). At the start of the area it is placed anyway:
  wrapping could not make it fit.
- `ESC @` resets every setting (code page included) and drops the unprinted line.
- `CR` does nothing (automatic line feed off, the default).
- `HT` jumps to the next tab stop in the current character width; a stop past the print
  area moves to its end (the next character wraps), as the reference says; past the last
  stop it does nothing. `ESC D` ends at a NUL, at the first value that does not ascend (that byte
  is normal data), or after 32 values, as the reference says.
- `GS V n` feeds `n` dots, then cuts.

**On purpose, not like the printer:** text without a final `LF` is printed when the job
ends. The printer would keep it in its buffer, and an emulator that hides it would hide a
bug from the developer.

Not emulated: page mode, upside-down and 90° rotation, user-defined characters,
international character sets (`ESC R`), Kanji.

## Images (`bitmap.rs`, `printer.rs`)

All images are 1-bit `Bitmap`s, sent as base64 and drawn straight onto the canvas: no
PNG, no `data:` URL, so the CSP stays closed.

| Command                                | Becomes                                 | Notes                                                                                  |
| -------------------------------------- | --------------------------------------- | -------------------------------------------------------------------------------------- |
| `GS v 0 m`                             | `Block::Image`                          | `m` 1 doubles the width, 2 the height, 3 both                                          |
| `GS ( L` / `GS 8 L` fn 112, then fn 50 | `Block::Image`                          | stored, printed on fn 50 (or 2); `bx`/`by` scale 1–2; color ignored                    |
| `ESC * m`                              | stripe inside the current `Block::Line` | 0/1: 8 dots, each 3 dots tall; 32/33: 24 dots; 0/32 single density (2 dots per column) |

- Image bands are aligned by `ESC a` inside the print area (margin and `GS W`), and a
  started text line prints first.
- `ESC *` stripes belong to the line, so the usual `ESC 3 24` + stripe + `LF` sequence
  stacks without gaps: the line is `max(line spacing, tallest stripe)` tall.
- Everything is cropped to the print area **before** scaling, so an image costs at most
  4× its input bytes in memory.

## Barcodes (`barcode.rs`) and QR codes

| `GS k m`                                      | Symbology | Data                                                                                                                            |
| --------------------------------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------- |
| 0 / 65                                        | UPC-A     | 11 digits (check digit added) or 12                                                                                             |
| 2 / 67                                        | EAN-13    | 12 digits (check digit added) or 13                                                                                             |
| 3 / 68                                        | EAN-8     | 7 digits (check digit added) or 8                                                                                               |
| 4 / 69                                        | CODE39    | `0–9 A–Z space $ % + - . /`; `*` start/stop added when missing                                                                  |
| 5 / 70                                        | ITF       | an even number of digits                                                                                                        |
| 6 / 71                                        | CODABAR   | starts and ends with `A–D`                                                                                                      |
| 73                                            | CODE128   | starts with `{A`/`{B`/`{C`; `{A`/`{B`/`{C` switch, `{S` shift, `{1`–`{4` FNC, `{{` = `{`; in set C each byte 0–99 is two digits |
| 1/66 (UPC-E), 72 (CODE93), 74+ (GS1 DataBar…) | not drawn |                                                                                                                                 |

- Defaults (Epson power-on): height 162 dots, module 3 dots, no HRI, HRI font A. `ESC @`
  restores them.
- Module codes (EAN/UPC/CODE128): element = modules × `GS w`. Narrow/wide codes (CODE39,
  ITF, CODABAR): narrow = `GS w`, wide = `ceil(5 × w / 2)` (2→5, 3→8, 4→10, 5→13, 6→15).
- The bars are a `Block::Image`, aligned by `ESC a`. HRI is a `Block::Line` above and/or
  below, centered on the bars (CODE39 HRI shows the `*`s).
- **Invalid data or a barcode wider than the print area prints nothing**, like the printer.
- Tables come from the symbology specs. The tests compare every symbology bit for bit
  with strings produced by python-barcode, used only as an oracle outside the repo.

QR (`GS ( k`, `cn` 49): module size 1–16 dots (default 3), error correction L/M/Q/H
(default L), store, print. Encoded with the `qrcode` crate (no default features), drawn
without a quiet zone as a `Block::Image` aligned by `ESC a`. Nothing stored, data too long
for a QR, or a symbol wider than the print area (cropped it could not be read; barcodes
do the same): nothing printed. Model selection (fn 65) is ignored.

## Status replies (decision 5)

The printer answers as an idle printer with paper, cover closed and no error. Replies are
`Output::Reply` bytes; `capture.rs` collects them (they never start a receipt) and the
listener writes them after each read, with the idle timeout so a client that never reads
cannot hold the task.

| Request                                 | Reply                                                                                      |
| --------------------------------------- | ------------------------------------------------------------------------------------------ |
| `DLE EOT 1`–`4`, `7`, `8`               | `0x12` (fixed bits 1 and 4; online, no error, paper present)                               |
| `GS r 1`/`2`/`4` (or `'1'`/`'2'`/`'4'`) | `0x00` (paper present, drawer pin low, ink fine)                                           |
| `GS I 1` / `2` / `3`                    | `0x20` / `0x02` (autocutter installed) / `0x10`                                            |
| `GS I 65`–`69`                          | `_` + text + `NUL`: `1.0`, `Thermal Printer Emulator`, `Virtual 80mm`, `0000000001`, `ANK` |

The identity is honest, not a real printer model. Other requests get no reply. Not
emulated: Automatic Status Back (`GS a`), `DLE ENQ`.

## Inspect (`inspect.rs`)

The Commands view of a receipt: `inspect(raw, default_code_page)` re-parses the receipt's
raw bytes with the same parser and returns one `CommandRow` per command:

- `offset` and `length` in the receipt, `bytes` (hex of the first 16, `…` when cut);
- `mnemonic` as the reference writes it (`ESC a`, `GS ( k`, `DLE EOT`, `LF`; empty for
  text);
- `kind`, an i18n key under `inspect.kinds` (`align_center`, `cut_partial`, `raster`…),
  translated by the webview;
- `detail` in the reference's notation, language-neutral (`n=16`, `96×32 m=0`, `CP850`,
  `m=73 {BAb12`), or the text decoded in the code page in force (it follows `ESC t` and
  `ESC @` like the printer).

At most 5,000 rows (`truncated` says when there were more). `get_receipt_commands` copies
the raw bytes out of the store before parsing, so a large receipt never holds the lock.
