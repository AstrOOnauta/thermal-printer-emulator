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
