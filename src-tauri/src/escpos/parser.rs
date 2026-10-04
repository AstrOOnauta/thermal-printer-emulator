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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

/// A real-time status request. The printer answers these on the socket.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusRequest {
    /// `DLE EOT n`.
    Realtime(u8),
    /// `GS r n`.
    Status(u8),
    /// `GS I n`.
    PrinterId(u8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Printable bytes, still in the selected code page.
    Text(Vec<u8>),
    LineFeed,
    CarriageReturn,
    Tab,
    /// `ESC @`.
    Initialize,
    /// `ESC ! n`: font, emphasis, double height/width, underline at once.
    PrintMode(u8),
    /// `ESC E n` / `ESC G n` (double-strike prints like emphasis).
    Emphasis(bool),
    /// `ESC - n`: 0 off, 1 thin, 2 thick.
    Underline(u8),
    /// `ESC a n`.
    Align(Alignment),
    /// `ESC M n`: 0 font A, 1 font B, 2 font C.
    Font(u8),
    /// `GS ! n`: width and height multipliers, 1–8.
    CharSize {
        width: u8,
        height: u8,
    },
    /// `GS B n`.
    Reverse(bool),
    /// `ESC t n`.
    CodeTable(u8),
    /// `ESC SP n`: extra dots to the right of each character.
    CharSpacing(u8),
    /// `ESC 2` (default) / `ESC 3 n` (dots).
    LineSpacing(Option<u8>),
    /// `ESC J n`: print and feed n dots.
    FeedDots(u8),
    /// `ESC d n`: print and feed n lines.
    FeedLines(u8),
    /// `GS L nL nH`, dots.
    LeftMargin(u16),
    /// `GS W nL nH`, dots.
    PrintAreaWidth(u16),
    /// `ESC $ nL nH`, dots from the start of the print area.
    AbsolutePosition(u16),
    /// `ESC \ nL nH`, dots from the current position (signed).
    RelativePosition(i16),
    /// `ESC D n1…nk NUL`, in character columns.
    TabStops(Vec<u8>),
    /// `GS V`, `ESC i`, `ESC m`. `feed` dots before the cut.
    Cut {
        partial: bool,
        feed: u8,
    },
    /// `ESC p` / `DLE DC4 1`: open the cash drawer.
    DrawerPulse,
    /// `ESC B n t` (buzzer on many ESC/POS printers).
    Beep,
    Status(StatusRequest),
    /// `GS v 0 m xL xH yL yH d…`: raster image, `row_bytes` × `height`; `mode` 1 doubles
    /// the width, 2 the height, 3 both.
    Raster {
        mode: u8,
        row_bytes: u16,
        height: u16,
        data: Vec<u8>,
    },
    /// `ESC * m nL nH d…`: one stripe of a column-format image, part of the current line.
    BitImage {
        mode: u8,
        columns: u16,
        data: Vec<u8>,
    },
    /// `GS ( L` / `GS 8 L` function 112: store a raster graphic, `scale` 1 or 2.
    StoreGraphics {
        width: u16,
        height: u16,
        scale_x: u8,
        scale_y: u8,
        data: Vec<u8>,
    },
    /// `GS ( L` / `GS 8 L` function 50 (or 2): print the stored graphic.
    PrintGraphics,
    /// `GS h n`: barcode height in dots.
    BarcodeHeight(u8),
    /// `GS w n`: barcode module width in dots.
    BarcodeWidth(u8),
    /// `GS H n`: HRI text 0 none, 1 above, 2 below, 3 both.
    HriPosition(u8),
    /// `GS f n`: HRI font, 0 A, 1 B.
    HriFont(u8),
    /// `GS k m …`: `symbology` is the raw `m`.
    Barcode {
        symbology: u8,
        data: Vec<u8>,
    },
    /// `GS ( k` function 167: QR module size in dots.
    QrModuleSize(u8),
    /// `GS ( k` function 169: error correction, 48 L, 49 M, 50 Q, 51 H.
    QrErrorCorrection(u8),
    /// `GS ( k` function 180: store the QR data.
    QrStore(Vec<u8>),
    /// `GS ( k` function 181: print the stored QR.
    QrPrint,
    /// A known command with no visible effect here.
    Ignored,
    /// An `ESC`/`GS`/`FS` command this parser does not know. Two bytes skipped.
    Unknown([u8; 2]),
}

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
        b'D' => match input[2..].iter().position(|&byte| byte == 0) {
            Some(end) => Step::Done(Command::TabStops(input[2..2 + end].to_vec()), end + 3),
            None => Step::Incomplete,
        },
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

/// The payload of `GS ( L` / `GS 8 L` after its length: `m fn [parameters]`.
fn graphics(payload: &[u8]) -> Command {
    match payload {
        [48, 2 | 50, ..] => Command::PrintGraphics,
        // m fn a bx by c xL xH yL yH d…, raster format.
        [48, 112, 48, scale_x, scale_y, _color, x_low, x_high, y_low, y_high, data @ ..] => {
            Command::StoreGraphics {
                width: le16(*x_low, *x_high),
                height: le16(*y_low, *y_high),
                scale_x: (*scale_x).clamp(1, 2),
                scale_y: (*scale_y).clamp(1, 2),
                data: data.to_vec(),
            }
        }
        _ => Command::Ignored,
    }
}

/// `GS k m d1…dk NUL` (m = 0–6) or `GS k m n d1…dn` (m = 65–79).
fn gs_barcode(input: &[u8]) -> Step {
    let Some(&kind) = input.get(2) else {
        return Step::Incomplete;
    };
    if kind <= 6 {
        match input[3..].iter().position(|&byte| byte == 0) {
            Some(end) => Step::Done(
                Command::Barcode {
                    symbology: kind,
                    data: input[3..3 + end].to_vec(),
                },
                end + 4,
            ),
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
mod tests {
    use super::*;

    fn commands(bytes: &[u8]) -> Vec<Command> {
        let mut parser = Parser::default();
        let mut out = Vec::new();
        parser.feed(bytes, |command, _| out.push(command));
        assert_eq!(parser.pending_len(), 0, "left incomplete: {bytes:02x?}");
        out
    }

    fn one(bytes: &[u8]) -> Command {
        let mut all = commands(bytes);
        assert_eq!(all.len(), 1, "{bytes:02x?} → {all:?}");
        all.remove(0)
    }

    #[test]
    fn text_runs_stop_at_control_bytes() {
        assert_eq!(
            commands(b"Hello\nWorld"),
            vec![
                Command::Text(b"Hello".to_vec()),
                Command::LineFeed,
                Command::Text(b"World".to_vec()),
            ]
        );
        assert_eq!(one(b"\x80\xff"), Command::Text(vec![0x80, 0xff]));
    }

    #[test]
    fn esc_commands() {
        assert_eq!(one(b"\x1b@"), Command::Initialize);
        assert_eq!(one(b"\x1b!\x38"), Command::PrintMode(0x38));
        assert_eq!(one(b"\x1bE\x01"), Command::Emphasis(true));
        assert_eq!(one(b"\x1bG\x00"), Command::Emphasis(false));
        assert_eq!(one(b"\x1b-\x02"), Command::Underline(2));
        assert_eq!(one(b"\x1b-1"), Command::Underline(1));
        assert_eq!(one(b"\x1ba\x01"), Command::Align(Alignment::Center));
        assert_eq!(one(b"\x1ba2"), Command::Align(Alignment::Right));
        assert_eq!(one(b"\x1bM1"), Command::Font(1));
        assert_eq!(one(b"\x1bt\x02"), Command::CodeTable(2));
        assert_eq!(one(b"\x1b \x04"), Command::CharSpacing(4));
        assert_eq!(one(b"\x1b2"), Command::LineSpacing(None));
        assert_eq!(one(b"\x1b3\x40"), Command::LineSpacing(Some(0x40)));
        assert_eq!(one(b"\x1bJ\x10"), Command::FeedDots(0x10));
        assert_eq!(one(b"\x1bd\x03"), Command::FeedLines(3));
        assert_eq!(one(b"\x1b$\x00\x01"), Command::AbsolutePosition(256));
        assert_eq!(one(b"\x1b\\\xf6\xff"), Command::RelativePosition(-10));
        assert_eq!(one(b"\x1bD\x08\x10\x00"), Command::TabStops(vec![8, 16]));
        assert_eq!(one(b"\x1bp\x00\x19\xfa"), Command::DrawerPulse);
        assert_eq!(one(b"\x1bB\x02\x01"), Command::Beep);
        assert_eq!(
            one(b"\x1bm"),
            Command::Cut {
                partial: true,
                feed: 0
            }
        );
    }

    #[test]
    fn gs_commands() {
        assert_eq!(
            one(b"\x1d!\x11"),
            Command::CharSize {
                width: 2,
                height: 2
            }
        );
        assert_eq!(
            one(b"\x1d!\x70"),
            Command::CharSize {
                width: 8,
                height: 1
            }
        );
        assert_eq!(one(b"\x1dB\x01"), Command::Reverse(true));
        assert_eq!(one(b"\x1dL\x10\x00"), Command::LeftMargin(16));
        assert_eq!(one(b"\x1dW\x80\x01"), Command::PrintAreaWidth(384));
        assert_eq!(
            one(b"\x1dV\x00"),
            Command::Cut {
                partial: false,
                feed: 0
            }
        );
        assert_eq!(
            one(b"\x1dV1"),
            Command::Cut {
                partial: true,
                feed: 0
            }
        );
        assert_eq!(
            one(b"\x1dVB\x05"),
            Command::Cut {
                partial: true,
                feed: 5
            }
        );
        assert_eq!(one(b"\x1dr\x01"), Command::Status(StatusRequest::Status(1)));
        assert_eq!(
            one(b"\x1dI\x42"),
            Command::Status(StatusRequest::PrinterId(0x42))
        );
    }

    #[test]
    fn dle_real_time_commands() {
        assert_eq!(
            one(b"\x10\x04\x01"),
            Command::Status(StatusRequest::Realtime(1))
        );
        assert_eq!(
            one(b"\x10\x04\x07\x01"),
            Command::Status(StatusRequest::Realtime(7))
        );
        assert_eq!(one(b"\x10\x14\x01\x00\x01"), Command::DrawerPulse);
        assert_eq!(
            one(b"\x10\x14\x08\x01\x03\x14\x01\x06\x02\x08"),
            Command::Ignored
        );
        assert_eq!(
            commands(b"\x10A"),
            vec![Command::Ignored, Command::Text(b"A".to_vec())]
        );
    }

    #[test]
    fn skips_data_carrying_commands_by_their_length() {
        // Each one is followed by text that must survive intact.
        for skipped in [
            &b"\x1d8L\x02\x00\x00\x00\x30\x45"[..], // graphics, unknown function
            b"\x1d*\x01\x01\x01\x02\x03\x04\x05\x06\x07\x08", // downloaded image 1×1
            b"\x1b&\x03\x41\x42\x01\x01\x02\x03\x01\x04\x05\x06", // 2 user chars
            b"\x1cq\x01\x01\x00\x01\x00\x01\x02\x03\x04\x05\x06\x07\x08", // NV image
            b"\x1b(A\x02\x00\x01\x02",              // ESC ( A
        ] {
            let mut stream = skipped.to_vec();
            stream.extend_from_slice(b"ok");
            assert_eq!(
                commands(&stream),
                vec![Command::Ignored, Command::Text(b"ok".to_vec())],
                "{skipped:02x?}"
            );
        }
    }

    #[test]
    fn image_commands_carry_their_data() {
        assert_eq!(
            one(b"\x1dv0\x01\x02\x00\x02\x00\xaa\xbb\xcc\xdd"),
            Command::Raster {
                mode: 1,
                row_bytes: 2,
                height: 2,
                data: vec![0xaa, 0xbb, 0xcc, 0xdd]
            }
        );
        assert_eq!(
            one(b"\x1b*\x00\x03\x00\x01\x02\x03"),
            Command::BitImage {
                mode: 0,
                columns: 3,
                data: vec![1, 2, 3]
            }
        );
        assert_eq!(
            one(b"\x1b*\x21\x02\x00\x01\x02\x03\x04\x05\x06"),
            Command::BitImage {
                mode: 33,
                columns: 2,
                data: vec![1, 2, 3, 4, 5, 6]
            }
        );
        assert_eq!(
            one(b"\x1d(L\x0b\x000p0\x01\x01\x31\x08\x00\x01\x00\xff"),
            Command::StoreGraphics {
                width: 8,
                height: 1,
                scale_x: 1,
                scale_y: 1,
                data: vec![0xff]
            }
        );
        assert_eq!(
            one(b"\x1d8L\x0b\x00\x00\x000p0\x02\x02\x31\x08\x00\x01\x00\x0f"),
            Command::StoreGraphics {
                width: 8,
                height: 1,
                scale_x: 2,
                scale_y: 2,
                data: vec![0x0f]
            }
        );
        assert_eq!(one(b"\x1d(L\x02\x0002"), Command::PrintGraphics);
    }

    #[test]
    fn barcode_and_qr_commands() {
        assert_eq!(one(b"\x1dh\x50"), Command::BarcodeHeight(80));
        assert_eq!(one(b"\x1dw\x02"), Command::BarcodeWidth(2));
        assert_eq!(one(b"\x1dH2"), Command::HriPosition(2));
        assert_eq!(one(b"\x1df\x01"), Command::HriFont(1));
        assert_eq!(
            one(b"\x1dk\x04AB\x00"),
            Command::Barcode {
                symbology: 4,
                data: b"AB".to_vec()
            }
        );
        assert_eq!(
            one(b"\x1dkI\x03{BA"),
            Command::Barcode {
                symbology: 73,
                data: b"{BA".to_vec()
            }
        );
        assert_eq!(one(b"\x1d(k\x03\x00\x31\x43\x05"), Command::QrModuleSize(5));
        assert_eq!(
            one(b"\x1d(k\x03\x00\x31\x45\x31"),
            Command::QrErrorCorrection(49)
        );
        assert_eq!(
            one(b"\x1d(k\x05\x00\x31\x50\x30hi"),
            Command::QrStore(b"hi".to_vec())
        );
        assert_eq!(one(b"\x1d(k\x03\x00\x31\x51\x30"), Command::QrPrint);
        assert_eq!(
            one(b"\x1d(k\x03\x00\x30\x43\x05"),
            Command::Ignored,
            "PDF417"
        );
    }

    #[test]
    fn unknown_commands_skip_two_bytes() {
        assert_eq!(
            commands(b"\x1b\x01ok"),
            vec![Command::Unknown([ESC, 0x01]), Command::Text(b"ok".to_vec())]
        );
        assert_eq!(one(b"\x1d\x7e"), Command::Unknown([GS, 0x7e]));
    }

    #[test]
    fn waits_for_incomplete_commands() {
        let mut parser = Parser::default();
        let mut out = Vec::new();
        parser.feed(b"A\x1b", |command, _| out.push(command));
        assert_eq!(out, vec![Command::Text(b"A".to_vec())]);
        assert_eq!(parser.pending_len(), 1);
        parser.feed(b"!", |command, _| out.push(command));
        assert_eq!(parser.pending_len(), 2);
        parser.feed(b"\x08", |command, _| out.push(command));
        assert_eq!(out.last(), Some(&Command::PrintMode(8)));
        assert_eq!(parser.pending_len(), 0);
    }

    #[test]
    fn reports_stream_offsets_after_each_command() {
        let mut parser = Parser::default();
        let mut ends = Vec::new();
        parser.feed(b"AB\x1dV", |_, end| ends.push(end));
        parser.feed(b"\x00C", |_, end| ends.push(end));
        assert_eq!(ends, vec![2, 5, 6]);
    }

    const SAMPLE: &[u8] = b"\x1b@\x1ba\x01\x1b!\x30Store\n\x1b!\x00\x1bt\x02Caf\x82 1,50\n\
        \x1dv0\x00\x01\x00\x02\x00\xf0\x0f\x1dkI\x04{B12\x1d(k\x03\x00\x31\x45\x31\
        \x1bD\x08\x00\tTab\n\x10\x04\x01\x1bd\x03\x1dVA\x10";

    #[test]
    fn any_split_parses_like_the_whole_stream() {
        let whole = commands(SAMPLE);
        for split in 1..SAMPLE.len() {
            let mut parser = Parser::default();
            let mut out = Vec::new();
            parser.feed(&SAMPLE[..split], |command, end| out.push((command, end)));
            parser.feed(&SAMPLE[split..], |command, end| out.push((command, end)));
            // Text may arrive in two runs; everything else must match exactly.
            let merged = merge_text(out.into_iter().map(|(command, _)| command).collect());
            assert_eq!(merged, merge_text(whole.clone()), "split at {split}");
        }
    }

    fn merge_text(commands: Vec<Command>) -> Vec<Command> {
        let mut merged: Vec<Command> = Vec::new();
        for command in commands {
            if let (Command::Text(more), Some(Command::Text(previous))) =
                (&command, merged.last_mut())
            {
                previous.extend_from_slice(more);
                continue;
            }
            merged.push(command);
        }
        merged
    }

    /// xorshift64: deterministic, no dependency.
    fn random(state: &mut u64) -> u64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        *state
    }

    #[test]
    fn random_bytes_never_panic_and_never_stall() {
        let mut state = 0x9e37_79b9_7f4a_7c15;
        for _ in 0..2_000 {
            let len = (random(&mut state) % 512) as usize;
            let bytes: Vec<u8> = (0..len)
                // Bias towards command bytes so the interesting paths run.
                .map(|_| match random(&mut state) % 4 {
                    0 => [ESC, GS, FS, DLE][(random(&mut state) % 4) as usize],
                    _ => random(&mut state) as u8,
                })
                .collect();
            let mut parser = Parser::default();
            let mut consumed = 0;
            for chunk in bytes.chunks(1 + (random(&mut state) % 16) as usize) {
                parser.feed(chunk, |_, end| consumed = end);
            }
            assert_eq!(consumed as usize + parser.pending_len(), bytes.len());
        }
    }
}
