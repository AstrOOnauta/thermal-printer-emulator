//! The printer: applies `Command`s to its state and lays text out in dots, producing the
//! print model the webview draws (`Block`s) plus the side effects (cut, drawer, beep).
//!
//! Geometry is a 203 dpi, 80 mm (576 dots) or 58 mm (384 dots) printer with Epson's fonts:
//! font A 12×24 dots, font B 9×17, default line spacing 30 dots.

use super::barcode::Symbology;
use super::bitmap::Bitmap;
use super::codepage::CodePage;
use super::command::{Alignment, Command, StatusRequest};
use super::model::{Block, Font, Output, Paper, Placed, Segment};

pub const DEFAULT_LINE_SPACING: u16 = 30;
/// Default tab stops: every 8 character columns.
const DEFAULT_TAB_COLUMNS: u16 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Style {
    font: Font,
    bold: bool,
    underline: u8,
    reverse: bool,
    width: u8,
    height: u8,
}

impl Style {
    const DEFAULT: Self = Self {
        font: Font::A,
        bold: false,
        underline: 0,
        reverse: false,
        width: 1,
        height: 1,
    };
}

/// What an idle printer with paper, cover closed and no error answers (decision 5).
fn status_reply(request: StatusRequest) -> Option<Vec<u8>> {
    // DLE EOT: bits 1 and 4 are fixed to 1; every other bit 0 means "all good".
    const REALTIME_OK: u8 = 0x12;
    match request {
        StatusRequest::Realtime(1..=4 | 7 | 8) => Some(vec![REALTIME_OK]),
        // GS r 1/2/4: paper present, drawer pin low, ink fine.
        StatusRequest::Status(1 | 2 | 4 | 49 | 50 | 52) => Some(vec![0x00]),
        StatusRequest::PrinterId(n) => printer_id(n),
        _ => None,
    }
}

/// `GS I n`: 1–3 answer one byte; 65+ answer `_` + text + NUL. An honest identity, not a
/// real printer model.
fn printer_id(n: u8) -> Option<Vec<u8>> {
    let text = match n {
        1 | 49 => return Some(vec![0x20]),
        // Type: autocutter installed.
        2 | 50 => return Some(vec![0x02]),
        3 | 51 => return Some(vec![0x10]),
        65 => "1.0",
        66 => "Thermal Printer Emulator",
        67 => "Virtual 80mm",
        68 => "0000000001",
        69 => "ANK",
        _ => return None,
    };
    let mut reply = vec![b'_'];
    reply.extend_from_slice(text.as_bytes());
    reply.push(0);
    Some(reply)
}

#[derive(Clone, Copy)]
struct BarcodeSettings {
    height: u16,
    module: u16,
    /// Bit 0: HRI above, bit 1: below.
    hri: u8,
    hri_font: Font,
}

impl BarcodeSettings {
    /// Epson's power-on values.
    const DEFAULT: Self = Self {
        height: 162,
        module: 3,
        hri: 0,
        hri_font: Font::A,
    };
}

struct QrSettings {
    module: u16,
    level: u8,
    data: Vec<u8>,
}

impl Default for QrSettings {
    fn default() -> Self {
        Self {
            module: 3,
            level: 48,
            data: Vec::new(),
        }
    }
}

/// The line being filled. Alignment and the print area are taken when it starts: Epson
/// applies `ESC a`, `GS L` and `GS W` only at the beginning of a line.
struct Line {
    segments: Vec<Segment>,
    images: Vec<Placed>,
    /// Tallest `ESC *` stripe so far.
    image_height: u16,
    /// Next character position, dots from the line's left edge.
    x: u16,
    /// Tallest character so far.
    ascent: u16,
    alignment: Alignment,
    left: u16,
    width: u16,
}

pub struct Printer {
    paper: Paper,
    /// What `ESC @` goes back to: the configured default (decision 4).
    default_code_page: CodePage,
    code_page: CodePage,
    style: Style,
    alignment: Alignment,
    char_spacing: u16,
    line_spacing: u16,
    left_margin: u16,
    area_width: u16,
    /// Tab stops in character columns, ascending.
    tab_stops: Vec<u16>,
    line: Option<Line>,
    /// `GS ( L` function 112 stores it, function 50 prints it.
    graphics: Option<Bitmap>,
    barcode: BarcodeSettings,
    qr: QrSettings,
    /// Commands the parser did not know, for the job log.
    pub unknown_commands: usize,
}

impl Printer {
    pub fn new(paper: Paper) -> Self {
        Self::with_code_page(paper, CodePage::DEFAULT)
    }

    pub fn with_code_page(paper: Paper, code_page: CodePage) -> Self {
        Self {
            paper,
            default_code_page: code_page,
            code_page,
            style: Style::DEFAULT,
            alignment: Alignment::Left,
            char_spacing: 0,
            line_spacing: DEFAULT_LINE_SPACING,
            left_margin: 0,
            area_width: paper.dots(),
            tab_stops: (1..=32).map(|n| n * DEFAULT_TAB_COLUMNS).collect(),
            line: None,
            graphics: None,
            barcode: BarcodeSettings::DEFAULT,
            qr: QrSettings::default(),
            unknown_commands: 0,
        }
    }

    pub fn apply(&mut self, command: Command, out: &mut Vec<Output>) {
        match command {
            Command::Text(bytes) => {
                for byte in bytes {
                    let character = self.code_page.decode(byte);
                    self.put(character, out);
                }
            }
            Command::LineFeed => self.end_line(None, out),
            // With automatic line feed off (the default), CR does nothing.
            Command::CarriageReturn => {}
            Command::Tab => self.tab(),
            Command::Initialize => {
                let unknown = self.unknown_commands;
                *self = Self::with_code_page(self.paper, self.default_code_page);
                self.unknown_commands = unknown;
            }
            Command::PrintMode(mode) => {
                self.style.font = if mode & 0x01 != 0 { Font::B } else { Font::A };
                self.style.bold = mode & 0x08 != 0;
                self.style.height = if mode & 0x10 != 0 { 2 } else { 1 };
                self.style.width = if mode & 0x20 != 0 { 2 } else { 1 };
                self.style.underline = if mode & 0x80 != 0 { 1 } else { 0 };
            }
            Command::Emphasis(on) => self.style.bold = on,
            Command::Underline(thickness) => self.style.underline = thickness.min(2),
            Command::Align(alignment) => self.alignment = alignment,
            // Font C, where a printer has it, is the size of font B.
            Command::Font(font) => self.style.font = if font == 0 { Font::A } else { Font::B },
            Command::CharSize { width, height } => {
                self.style.width = width;
                self.style.height = height;
            }
            Command::Reverse(on) => self.style.reverse = on,
            Command::CodeTable(table) => {
                if let Some(page) = CodePage::from_table(table) {
                    self.code_page = page;
                }
            }
            Command::CharSpacing(dots) => self.char_spacing = u16::from(dots),
            Command::LineSpacing(dots) => {
                self.line_spacing = dots.map_or(DEFAULT_LINE_SPACING, u16::from);
            }
            Command::FeedDots(dots) => self.end_line(Some(u16::from(dots)), out),
            Command::FeedLines(lines) => {
                // A line with content counts as the first of the n lines it feeds.
                let had_text = self.has_content();
                if had_text {
                    self.end_line(None, out);
                }
                let lines = u16::from(lines).saturating_sub(u16::from(had_text));
                self.feed(lines.saturating_mul(self.line_spacing), out);
            }
            Command::LeftMargin(dots) => self.left_margin = dots.min(self.paper.dots()),
            Command::PrintAreaWidth(dots) => self.area_width = dots,
            Command::AbsolutePosition(dots) => {
                let line = self.line_mut();
                if dots < line.width {
                    line.x = dots;
                }
            }
            Command::RelativePosition(dots) => {
                let line = self.line_mut();
                if let Some(x) = line.x.checked_add_signed(dots).filter(|&x| x < line.width) {
                    line.x = x;
                }
            }
            // Ascending: the parser ends the list at the first value that doesn't.
            Command::TabStops(columns) => {
                self.tab_stops = columns.into_iter().map(u16::from).collect();
            }
            Command::Cut { partial, feed } => {
                self.end_line_if_started(out);
                self.feed(u16::from(feed), out);
                out.push(Output::Cut { partial });
            }
            Command::DrawerPulse => out.push(Output::DrawerPulse),
            Command::Beep => out.push(Output::Beep),
            Command::Raster {
                mode,
                row_bytes,
                height,
                data,
            } => {
                let (sx, sy) = match mode {
                    1 => (2, 1),
                    2 => (1, 2),
                    3 => (2, 2),
                    _ => (1, 1),
                };
                let bitmap = Bitmap::from_rows(row_bytes, height, &data);
                self.print_image(bitmap, sx, sy, out);
            }
            Command::BitImage {
                mode,
                columns,
                data,
            } => self.bit_image(mode, columns, &data),
            Command::StoreGraphics {
                width,
                height,
                scale_x,
                scale_y,
                data,
            } => {
                let bitmap = Bitmap::from_rows(width.div_ceil(8), height, &data);
                let max = self.paper.dots();
                self.graphics = Some(
                    bitmap
                        .cropped(width.min(max))
                        .scaled(u16::from(scale_x), u16::from(scale_y))
                        .cropped(max),
                );
            }
            Command::PrintGraphics => {
                if let Some(bitmap) = self.graphics.take() {
                    self.print_image(bitmap, 1, 1, out);
                }
            }
            // Out of range, these keep the setting they had, as the printer does.
            Command::BarcodeHeight(dots) if dots > 0 => self.barcode.height = u16::from(dots),
            Command::BarcodeWidth(dots) if (1..=6).contains(&dots) => {
                self.barcode.module = u16::from(dots);
            }
            Command::HriPosition(position) => self.barcode.hri = position,
            Command::HriFont(font) => {
                self.barcode.hri_font = if font == 0 { Font::A } else { Font::B };
            }
            Command::Barcode { symbology, data } => {
                self.print_barcode(Symbology::from_code(symbology), &data, out);
            }
            Command::QrModuleSize(dots) if (1..=16).contains(&dots) => {
                self.qr.module = u16::from(dots);
            }
            Command::QrErrorCorrection(level) if (48..=51).contains(&level) => {
                self.qr.level = level;
            }
            Command::BarcodeHeight(_)
            | Command::BarcodeWidth(_)
            | Command::QrModuleSize(_)
            | Command::QrErrorCorrection(_) => {}
            Command::QrStore(data) => self.qr.data = data,
            Command::QrPrint => self.print_qr(out),
            Command::Status(request) => {
                if let Some(reply) = status_reply(request) {
                    out.push(Output::Reply(reply));
                }
            }
            Command::Ignored => {}
            Command::Unknown(_) => self.unknown_commands += 1,
        }
    }

    /// End of the stream: prints what is left in the line buffer. A real printer would
    /// hold it until the next `LF`; an emulator that hid it would hide a bug from you.
    pub fn finish(&mut self, out: &mut Vec<Output>) {
        self.end_line_if_started(out);
    }

    fn line_mut(&mut self) -> &mut Line {
        let paper = self.paper.dots();
        let left = self.left_margin;
        let width = self.area_width.min(paper - left);
        let alignment = self.alignment;
        self.line.get_or_insert_with(|| Line {
            segments: Vec::new(),
            images: Vec::new(),
            image_height: 0,
            x: 0,
            ascent: 0,
            alignment,
            left,
            width,
        })
    }

    fn put(&mut self, character: char, out: &mut Vec<Output>) {
        let style = self.style;
        let (cell_width, cell_height) = style.font.cell();
        let glyph_width = cell_width * u16::from(style.width);
        let advance = (cell_width + self.char_spacing) * u16::from(style.width);

        let line = self.line_mut();
        // Wrap like the printer: a character that does not fit starts the next line, also
        // when only the position moved (`ESC $`, `HT`, an `ESC *` stripe). At the line's
        // start it is placed anyway: wrapping could not make it fit.
        if line.x > 0 && line.x + glyph_width > line.width {
            self.end_line(None, out);
        }
        let line = self.line_mut();
        let x = line.x;
        line.ascent = line.ascent.max(cell_height * u16::from(style.height));
        match line.segments.last_mut() {
            Some(segment)
                if segment.font == style.font
                    && segment.width == style.width
                    && segment.height == style.height
                    && segment.bold == style.bold
                    && segment.underline == style.underline
                    && segment.reverse == style.reverse
                    && segment.advance == advance
                    && segment.x + segment.text.chars().count() as u16 * advance == x =>
            {
                segment.text.push(character);
            }
            _ => line.segments.push(Segment {
                x,
                text: character.to_string(),
                font: style.font,
                width: style.width,
                height: style.height,
                advance,
                bold: style.bold,
                underline: style.underline,
                reverse: style.reverse,
            }),
        }
        line.x = x.saturating_add(advance);
    }

    fn tab(&mut self) {
        let (cell_width, _) = self.style.font.cell();
        let column = (cell_width + self.char_spacing) * u16::from(self.style.width);
        let stops = self.tab_stops.clone();
        let line = self.line_mut();
        // Past the print area, the reference moves to its end: the next character wraps.
        if let Some(x) = stops
            .iter()
            .map(|&stop| stop.saturating_mul(column))
            .find(|&x| x > line.x)
        {
            line.x = x.min(line.width);
        }
    }

    fn align_offset(&self, area: u16, width: u16) -> u16 {
        match self.alignment {
            Alignment::Left => 0,
            Alignment::Center => area.saturating_sub(width) / 2,
            Alignment::Right => area.saturating_sub(width),
        }
    }

    /// A band of its own, aligned by `ESC a` inside the print area, after any started
    /// line. The bitmap is cropped to the area before scaling, so memory stays bounded.
    fn print_image(&mut self, bitmap: Bitmap, sx: u16, sy: u16, out: &mut Vec<Output>) {
        self.end_line_if_started(out);
        let left = self.left_margin;
        let width = self.area_width.min(self.paper.dots() - left);
        let bitmap = bitmap.cropped(width / sx).scaled(sx, sy).cropped(width);
        if bitmap.width == 0 || bitmap.height == 0 {
            return;
        }
        let offset = self.align_offset(width, bitmap.width);
        out.push(Output::Block(Block::Image(Placed {
            x: left + offset,
            bitmap,
        })));
    }

    /// `ESC *` stripe: modes 0/1 are 8 dots tall (each dot 3 printer dots tall at 203 dpi),
    /// 32/33 are 24 dots; modes 0 and 32 are single density (each column 2 dots wide).
    fn bit_image(&mut self, mode: u8, columns: u16, data: &[u8]) {
        let (bytes_per_column, dot_height, column_width) = match mode {
            0 => (1, 3, 2),
            1 => (1, 3, 1),
            32 => (3, 1, 2),
            33 => (3, 1, 1),
            _ => return,
        };
        let line = self.line_mut();
        let available = line.width.saturating_sub(line.x);
        let columns = columns.min(available / column_width);
        let mut stripe = Bitmap::blank(columns * column_width, bytes_per_column * 8 * dot_height);
        for column in 0..columns {
            for byte in 0..bytes_per_column {
                let index = usize::from(column) * usize::from(bytes_per_column) + usize::from(byte);
                let Some(&bits) = data.get(index) else {
                    continue;
                };
                for bit in 0..8 {
                    if bits & (0x80 >> bit) == 0 {
                        continue;
                    }
                    let top = (byte * 8 + bit) * dot_height;
                    for dy in 0..dot_height {
                        for dx in 0..column_width {
                            stripe.set(column * column_width + dx, top + dy);
                        }
                    }
                }
            }
        }
        if stripe.width == 0 {
            return;
        }
        line.image_height = line.image_height.max(stripe.height);
        let x = line.x;
        line.x += stripe.width;
        line.images.push(Placed { x, bitmap: stripe });
    }

    fn has_content(&self) -> bool {
        self.line
            .as_ref()
            .is_some_and(|line| !line.segments.is_empty() || !line.images.is_empty())
    }

    fn end_line_if_started(&mut self, out: &mut Vec<Output>) {
        if self.has_content() {
            self.end_line(None, out);
        }
    }

    /// Prints the line (or feeds an empty one) and advances `feed` dots, or the line
    /// spacing; never less than its tallest character or image stripe.
    fn end_line(&mut self, feed: Option<u16>, out: &mut Vec<Output>) {
        let advance = feed.unwrap_or(self.line_spacing);
        if !self.has_content() {
            self.line = None;
            self.feed(advance, out);
            return;
        }
        let Some(mut line) = self.line.take() else {
            return;
        };
        let used = line.x;
        let offset = match line.alignment {
            Alignment::Left => 0,
            Alignment::Center => line.width.saturating_sub(used) / 2,
            Alignment::Right => line.width.saturating_sub(used),
        };
        for segment in &mut line.segments {
            segment.x += line.left + offset;
        }
        for image in &mut line.images {
            image.x += line.left + offset;
        }
        out.push(Output::Block(Block::Line {
            height: advance.max(line.ascent).max(line.image_height),
            ascent: line.ascent,
            segments: line.segments,
            images: line.images,
        }));
    }

    /// Blank paper; consecutive feeds merge into one block.
    fn feed(&mut self, dots: u16, out: &mut Vec<Output>) {
        if dots == 0 {
            return;
        }
        if let Some(Output::Block(Block::Feed { height })) = out.last_mut() {
            *height = height.saturating_add(dots);
            return;
        }
        out.push(Output::Block(Block::Feed { height: dots }));
    }
}

mod codes;

#[cfg(test)]
mod tests;
