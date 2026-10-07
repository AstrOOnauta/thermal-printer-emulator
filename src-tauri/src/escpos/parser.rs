//! ESC/POS byte stream → commands. Written from Epson's public ESC/POS command reference.
//!
//! Streaming: `Parser::feed` takes bytes as they arrive and keeps an incomplete command
//! until the rest comes, so a command split across TCP reads parses exactly like one sent
//! whole. Never panics and never allocates more than the bytes it was given: a length
//! field only says how many bytes to wait for.
//!
//! Every command the reference defines has a known length, even the ones this emulator
//! ignores, so skipping them keeps the rest of the stream aligned. An unknown `ESC x`,
//! `GS x` or `FS x` skips those two bytes.

use super::command::{Alignment, Command, StatusRequest};

const LF: u8 = 0x0a;
const HT: u8 = 0x09;
const FF: u8 = 0x0c;
const CR: u8 = 0x0d;
const DLE: u8 = 0x10;
const ESC: u8 = 0x1b;
const FS: u8 = 0x1c;
const GS: u8 = 0x1d;
const EOT: u8 = 0x04;
const ENQ: u8 = 0x05;
const DC4: u8 = 0x14;

enum Step {
    /// The command and how many bytes it took.
    Done(Command, usize),
    /// The input ends inside a command: wait for more bytes.
    Incomplete,
}

#[derive(Default)]
pub struct Parser {
    pending: Vec<u8>,
    /// Stream offset of `pending[0]`.
    offset: u64,
}

impl Parser {
    /// Parses as much of `pending + input` as forms whole commands. `emit` gets each
    /// command with the stream offset right after its last byte.
    pub fn feed(&mut self, input: &[u8], mut emit: impl FnMut(Command, u64)) {
        self.pending.extend_from_slice(input);
        let mut position = 0;
        while position < self.pending.len() {
            match parse(&self.pending[position..]) {
                Step::Done(command, used) => {
                    position += used;
                    emit(command, self.offset + position as u64);
                }
                Step::Incomplete => break,
            }
        }
        self.pending.drain(..position);
        self.offset += position as u64;
    }

    /// Bytes of an unfinished command still waiting. Dropped when the stream ends.
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
}

/// Little-endian u16 from two bytes.
fn le16(low: u8, high: u8) -> u16 {
    u16::from(low) | (u16::from(high) << 8)
}

/// `fixed(input, n, f)`: a command of `n` bytes, built from them by `f`.
fn fixed(input: &[u8], len: usize, build: impl FnOnce(&[u8]) -> Command) -> Step {
    match input.get(..len) {
        Some(bytes) => Step::Done(build(bytes), len),
        None => Step::Incomplete,
    }
}

/// A command of `len` bytes that only needs skipping.
fn skip(input: &[u8], len: usize) -> Step {
    fixed(input, len, |_| Command::Ignored)
}

fn parse(input: &[u8]) -> Step {
    let Some(&first) = input.first() else {
        return Step::Incomplete;
    };
    match first {
        LF => Step::Done(Command::LineFeed, 1),
        CR => Step::Done(Command::CarriageReturn, 1),
        HT => Step::Done(Command::Tab, 1),
        ESC => parse_esc(input),
        GS => parse_gs(input),
        FS => parse_fs(input),
        DLE => parse_dle(input),
        // FF prints the page in page mode (unsupported); other controls do nothing.
        0x00..=0x1f | 0x7f => Step::Done(Command::Ignored, 1),
        _ => {
            let len = input
                .iter()
                .position(|&byte| byte < 0x20 || byte == 0x7f)
                .unwrap_or(input.len());
            Step::Done(Command::Text(input[..len].to_vec()), len)
        }
    }
}

fn parse_esc(input: &[u8]) -> Step {
    let Some(&code) = input.get(1) else {
        return Step::Incomplete;
    };
    match code {
        b'@' => Step::Done(Command::Initialize, 2),
        b'!' => fixed(input, 3, |b| Command::PrintMode(b[2])),
        b'E' | b'G' => fixed(input, 3, |b| Command::Emphasis(b[2] & 1 == 1)),
        b'-' => fixed(input, 3, |b| Command::Underline(b[2] & 0x0f)),
        b'a' => fixed(input, 3, |b| {
            Command::Align(match b[2] {
                1 | b'1' => Alignment::Center,
                2 | b'2' => Alignment::Right,
                _ => Alignment::Left,
            })
        }),
        b'M' => fixed(input, 3, |b| Command::Font(b[2] & 0x0f)),
        b't' => fixed(input, 3, |b| Command::CodeTable(b[2])),
        b' ' => fixed(input, 3, |b| Command::CharSpacing(b[2])),
        b'2' => Step::Done(Command::LineSpacing(None), 2),
        b'3' => fixed(input, 3, |b| Command::LineSpacing(Some(b[2]))),
        b'J' => fixed(input, 3, |b| Command::FeedDots(b[2])),
        b'd' => fixed(input, 3, |b| Command::FeedLines(b[2])),
        b'$' => fixed(input, 4, |b| Command::AbsolutePosition(le16(b[2], b[3]))),
        b'\\' => fixed(input, 4, |b| {
            Command::RelativePosition(le16(b[2], b[3]) as i16)
        }),
        b'D' => tab_stops(input),
        b'i' => Step::Done(
            Command::Cut {
                partial: false,
                feed: 0,
            },
            2,
        ),
        b'm' => Step::Done(
            Command::Cut {
                partial: true,
                feed: 0,
            },
            2,
        ),
        b'p' => fixed(input, 5, |_| Command::DrawerPulse),
        b'B' => fixed(input, 4, |_| Command::Beep),
        b'*' => esc_bit_image(input),
        b'&' => esc_user_chars(input),
        // ESC ( fn pL pH data
        b'(' => match input.get(3..5) {
            Some(&[low, high]) => skip(input, 5 + usize::from(le16(low, high))),
            _ => Step::Incomplete,
        },
        // Page mode on/off, page print, select standard mode.
        b'L' | b'S' | FF => Step::Done(Command::Ignored, 2),
        // One parameter, no visible effect here: user chars on/off, peripheral, cancel user
        // char, reverse feed, international set, page direction, unidirectional, 90°
        // rotation, reverse feed lines, color, status transmit, upside down.
        b'%' | b'=' | b'?' | b'K' | b'R' | b'T' | b'U' | b'V' | b'e' | b'r' | b'u' | b'{' => {
            skip(input, 3)
        }
        b'v' => Step::Done(Command::Ignored, 2),
        // ESC c 3 n, ESC c 4 n, ESC c 5 n, ESC c 0/1 n: paper sensors, panel buttons.
        b'c' => skip(input, 4),
        // ESC W: page-mode print area, 8 parameters.
        b'W' => skip(input, 10),
        _ => Step::Done(Command::Unknown([ESC, code]), 2),
    }
}

/// `ESC * m nL nH d1…dk`: k = n columns × 1 byte (8-dot modes) or 3 bytes (24-dot modes).
fn esc_bit_image(input: &[u8]) -> Step {
    let Some(&[mode, low, high]) = input.get(2..5) else {
        return Step::Incomplete;
    };
    let columns = le16(low, high);
    let bytes_per_column = if mode >= 32 { 3 } else { 1 };
    let len = 5 + usize::from(columns) * bytes_per_column;
    fixed(input, len, |b| Command::BitImage {
        mode,
        columns,
        data: b[5..].to_vec(),
    })
}

/// `ESC & y c1 c2 [x d1…d(y×x)]…` for each character from c1 to c2.
fn esc_user_chars(input: &[u8]) -> Step {
    let Some(&[height, first, last]) = input.get(2..5) else {
        return Step::Incomplete;
    };
    let mut position = 5;
    for _ in first..=last.max(first) {
        let Some(&width) = input.get(position) else {
            return Step::Incomplete;
        };
        position += 1 + usize::from(height) * usize::from(width);
    }
    skip(input, position)
}

fn parse_gs(input: &[u8]) -> Step {
    let Some(&code) = input.get(1) else {
        return Step::Incomplete;
    };
    match code {
        b'!' => fixed(input, 3, |b| Command::CharSize {
            width: (b[2] >> 4 & 0x07) + 1,
            height: (b[2] & 0x07) + 1,
        }),
        b'B' => fixed(input, 3, |b| Command::Reverse(b[2] & 1 == 1)),
        b'L' => fixed(input, 4, |b| Command::LeftMargin(le16(b[2], b[3]))),
        b'W' => fixed(input, 4, |b| Command::PrintAreaWidth(le16(b[2], b[3]))),
        b'V' => gs_cut(input),
        b'r' => fixed(input, 3, |b| Command::Status(StatusRequest::Status(b[2]))),
        b'I' => fixed(input, 3, |b| {
            Command::Status(StatusRequest::PrinterId(b[2]))
        }),
        // GS ( fn pL pH data: QR/2D codes (k), graphics (L), setup, and more.
        b'(' => match input.get(2..5) {
            Some(&[function, low, high]) => {
                let len = 5 + usize::from(le16(low, high));
                fixed(input, len, |b| match function {
                    b'L' => graphics(&b[5..]),
                    b'k' => two_dimensional(&b[5..]),
                    _ => Command::Ignored,
                })
            }
            _ => Step::Incomplete,
        },
        // GS 8 L p1 p2 p3 p4 data: graphics with a 32-bit length.
        b'8' => match input.get(2..7) {
            Some(&[b'L', p1, p2, p3, p4]) => {
                let len = u32::from_le_bytes([p1, p2, p3, p4]) as usize;
                fixed(input, 7usize.saturating_add(len), |b| graphics(&b[7..]))
            }
            Some(_) => Step::Done(Command::Unknown([GS, code]), 2),
            None => Step::Incomplete,
        },
        b'v' => gs_raster(input),
        b'k' => gs_barcode(input),
        // GS * x y d1…d(x×y×8): define a downloaded bit image.
        b'*' => match input.get(2..4) {
            Some(&[x, y]) => skip(input, 4 + usize::from(x) * usize::from(y) * 8),
            _ => Step::Incomplete,
        },
        // One parameter: print downloaded image, head control, HRI position, ASB, smoothing,
        // HRI font, barcode height, ASB ink, barcode width, print position mode.
        b'h' => fixed(input, 3, |b| Command::BarcodeHeight(b[2])),
        b'w' => fixed(input, 3, |b| Command::BarcodeWidth(b[2])),
        b'H' => fixed(input, 3, |b| Command::HriPosition(b[2] & 0x03)),
        b'f' => fixed(input, 3, |b| Command::HriFont(b[2] & 0x01)),
        b'/' | b'E' | b'T' | b'a' | b'b' | b'j' => skip(input, 3),
        // Two parameters: page-mode vertical positions, motion units.
        b'$' | b'\\' | b'P' => skip(input, 4),
        // GS ^ r t m: execute macro.
        b'^' => skip(input, 5),
        // GS g 0/2 m nL nH: maintenance counters.
        b'g' => skip(input, 6),
        // Macro definition start/end, print counter.
        b':' | b'c' => Step::Done(Command::Ignored, 2),
        _ => Step::Done(Command::Unknown([GS, code]), 2),
    }
}

/// `GS V m` (m = 0, 1, 48, 49) or `GS V m n` (m = 65, 66, 97, 98, 103, 104: feed n first).
fn gs_cut(input: &[u8]) -> Step {
    let Some(&mode) = input.get(2) else {
        return Step::Incomplete;
    };
    let partial = matches!(mode, 1 | 49 | 66 | 98 | 104);
    match mode {
        0 | 1 | 48 | 49 => Step::Done(Command::Cut { partial, feed: 0 }, 3),
        65 | 66 | 97 | 98 | 103 | 104 => fixed(input, 4, |b| Command::Cut {
            partial,
            feed: b[3],
        }),
        _ => Step::Done(Command::Ignored, 3),
    }
}

/// `GS v 0 m xL xH yL yH d1…dk`: k = x bytes per row × y rows.
fn gs_raster(input: &[u8]) -> Step {
    let Some(&[b'0', _mode, x_low, x_high, y_low, y_high]) = input.get(2..8) else {
        return match input.get(2) {
            Some(&b'0') | None => Step::Incomplete,
            Some(_) => Step::Done(Command::Unknown([GS, b'v']), 2),
        };
    };
    let (row_bytes, height) = (le16(x_low, x_high), le16(y_low, y_high));
    let len = 8 + usize::from(row_bytes) * usize::from(height);
    fixed(input, len, |b| Command::Raster {
        mode: b[3] & 0x03,
        row_bytes,
        height,
        data: b[8..].to_vec(),
    })
}

/// At most this many stops in `ESC D`.
const MAX_TAB_STOPS: usize = 32;

/// `ESC D n1…nk NUL`. Per the reference, the list also ends at a value that does not ascend
/// (that byte and what follows are normal data) and after 32 values: a NUL-only rule would
/// swallow the rest of the receipt, cut included.
fn tab_stops(input: &[u8]) -> Step {
    let mut stops: Vec<u8> = Vec::new();
    for (index, &byte) in input[2..].iter().enumerate() {
        if byte == 0 {
            return Step::Done(Command::TabStops(stops), index + 3);
        }
        if stops.len() == MAX_TAB_STOPS || stops.last().is_some_and(|&last| byte <= last) {
            return Step::Done(Command::TabStops(stops), index + 2);
        }
        stops.push(byte);
    }
    Step::Incomplete
}

/// The payload of `GS ( L` / `GS 8 L` after its length: `m fn [parameters]`.
fn graphics(payload: &[u8]) -> Command {
    match payload {
        [48, 2 | 50, ..] => Command::PrintGraphics,
        // m fn a bx by c xL xH yL yH d…, raster format.
        [48, 112, 48, scale_x, scale_y, _color, x_low, x_high, y_low, y_high, data @ ..] => {
            let (width, height) = (le16(*x_low, *x_high), le16(*y_low, *y_high));
            // The reference sizes the data exactly: ceil(x / 8) × y bytes. Trusting the
            // header alone, 16 bytes could ask for a 512 MB image.
            let size = usize::from(width.div_ceil(8)) * usize::from(height);
            match data.get(..size) {
                Some(data) if size > 0 => Command::StoreGraphics {
                    width,
                    height,
                    scale_x: (*scale_x).clamp(1, 2),
                    scale_y: (*scale_y).clamp(1, 2),
                    data: data.to_vec(),
                },
                _ => Command::Ignored,
            }
        }
        _ => Command::Ignored,
    }
}

/// Longest data looked through for the NUL of `GS k` function A. Real barcodes are far
/// shorter; without a bound, every read would rescan everything pending (quadratic).
const MAX_BARCODE_A_DATA: usize = 255;

/// `GS k m d1…dk NUL` (m = 0–6) or `GS k m n d1…dn` (m = 65–79).
fn gs_barcode(input: &[u8]) -> Step {
    let Some(&kind) = input.get(2) else {
        return Step::Incomplete;
    };
    if kind <= 6 {
        let window = &input[3..input.len().min(3 + MAX_BARCODE_A_DATA + 1)];
        match window.iter().position(|&byte| byte == 0) {
            Some(end) => Step::Done(
                Command::Barcode {
                    symbology: kind,
                    data: input[3..3 + end].to_vec(),
                },
                end + 4,
            ),
            // No NUL where one must be: skip `GS k m`, the rest is read as normal data.
            None if window.len() > MAX_BARCODE_A_DATA => Step::Done(Command::Ignored, 3),
            None => Step::Incomplete,
        }
    } else {
        match input.get(3) {
            Some(&len) => fixed(input, 4 + usize::from(len), |b| Command::Barcode {
                symbology: kind,
                data: b[4..].to_vec(),
            }),
            None => Step::Incomplete,
        }
    }
}

/// The payload of `GS ( k` after its length: `cn fn [parameters]`. Only QR (`cn` 49) is
/// drawn; PDF417, MaxiCode, DataMatrix, Aztec are skipped.
fn two_dimensional(payload: &[u8]) -> Command {
    match payload {
        [49, 67, size, ..] => Command::QrModuleSize(*size),
        [49, 69, level, ..] => Command::QrErrorCorrection(*level),
        [49, 80, 48, data @ ..] => Command::QrStore(data.to_vec()),
        [49, 81, 48, ..] => Command::QrPrint,
        _ => Command::Ignored,
    }
}

fn parse_fs(input: &[u8]) -> Step {
    let Some(&code) = input.get(1) else {
        return Step::Incomplete;
    };
    match code {
        // Kanji mode on/off.
        b'&' | b'.' => Step::Done(Command::Ignored, 2),
        // One parameter: Kanji print mode, underline, code system, quadruple size.
        b'!' | b'-' | b'C' | b'W' => skip(input, 3),
        // Two parameters: Kanji spacing, print NV bit image.
        b'S' | b'p' => skip(input, 4),
        // FS 2 c1 c2 d1…d72: define a user Kanji character.
        b'2' => skip(input, 76),
        // FS ( fn pL pH data.
        b'(' => match input.get(3..5) {
            Some(&[low, high]) => skip(input, 5 + usize::from(le16(low, high))),
            _ => Step::Incomplete,
        },
        b'q' => fs_nv_images(input),
        _ => Step::Done(Command::Unknown([FS, code]), 2),
    }
}

/// `FS q n [xL xH yL yH d1…dk]…`: define n NV images, k = x × y × 8 bytes each.
fn fs_nv_images(input: &[u8]) -> Step {
    let Some(&count) = input.get(2) else {
        return Step::Incomplete;
    };
    let mut position = 3;
    for _ in 0..count {
        let Some(&[x_low, x_high, y_low, y_high]) = input.get(position..position + 4) else {
            return Step::Incomplete;
        };
        let size = usize::from(le16(x_low, x_high)) * usize::from(le16(y_low, y_high)) * 8;
        position += 4 + size;
    }
    skip(input, position)
}

fn parse_dle(input: &[u8]) -> Step {
    let Some(&code) = input.get(1) else {
        return Step::Incomplete;
    };
    match code {
        EOT => match input.get(2) {
            // DLE EOT 7 a / DLE EOT 8 a carry one more byte.
            Some(&(7 | 8)) => fixed(input, 4, |b| Command::Status(StatusRequest::Realtime(b[2]))),
            Some(&request) => Step::Done(Command::Status(StatusRequest::Realtime(request)), 3),
            None => Step::Incomplete,
        },
        ENQ => skip(input, 3),
        DC4 => match input.get(2) {
            Some(&1) => fixed(input, 5, |_| Command::DrawerPulse),
            Some(&(2 | 3)) => skip(input, 5),
            Some(&7) => skip(input, 4),
            Some(&8) => skip(input, 10),
            Some(_) => skip(input, 3),
            None => Step::Incomplete,
        },
        // A lone DLE is ignored by the printer; the next byte is data again.
        _ => Step::Done(Command::Ignored, 1),
    }
}

#[cfg(test)]
mod tests;
