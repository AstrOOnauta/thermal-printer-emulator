//! The printer: applies `Command`s to its state and lays text out in dots, producing the
//! print model the webview draws (`Block`s) plus the side effects (cut, drawer, beep).
//!
//! Geometry is a 203 dpi, 80 mm (576 dots) or 58 mm (384 dots) printer with Epson's fonts:
//! font A 12×24 dots, font B 9×17, default line spacing 30 dots.

use serde::Serialize;

use super::barcode::{self, Symbology};
use super::bitmap::Bitmap;
use super::codepage::CodePage;
use super::parser::{Alignment, Command};

pub const DEFAULT_LINE_SPACING: u16 = 30;
/// Default tab stops: every 8 character columns.
const DEFAULT_TAB_COLUMNS: u16 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Paper {
    Mm80,
    Mm58,
}

impl Paper {
    /// Printable width in dots.
    pub fn dots(self) -> u16 {
        match self {
            Self::Mm80 => 576,
            Self::Mm58 => 384,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Font {
    A,
    B,
}

impl Font {
    /// Character cell (width, height) in dots.
    pub fn cell(self) -> (u16, u16) {
        match self {
            Self::A => (12, 24),
            Self::B => (9, 17),
        }
    }
}

/// A run of characters in one style. Character `i` sits at `x + i × advance`; its glyph
/// fills a `font cell × (width, height)` box, bottom-aligned on the line's `ascent`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Segment {
    /// Dots from the left edge of the printable area.
    pub x: u16,
    pub text: String,
    pub font: Font,
    /// Size multipliers, 1–8.
    pub width: u8,
    pub height: u8,
    /// Dots from one character to the next (cell + spacing, times `width`).
    pub advance: u16,
    pub bold: bool,
    /// 0 off, 1 thin, 2 thick.
    pub underline: u8,
    pub reverse: bool,
}

/// A bitmap and the dots from the printable area's left edge to its left side.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Placed {
    pub x: u16,
    #[serde(flatten)]
    pub bitmap: Bitmap,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    /// One printed line, `height` dots tall. Glyph bottoms sit `ascent` dots from its top;
    /// `ESC *` image stripes in the line hang from its top.
    Line {
        height: u16,
        ascent: u16,
        segments: Vec<Segment>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        images: Vec<Placed>,
    },
    /// An image band of its own (raster, graphics), as tall as the image.
    Image(Placed),
    /// Blank paper.
    Feed { height: u16 },
}

/// What applying a command produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Output {
    Block(Block),
    Cut { partial: bool },
    DrawerPulse,
    Beep,
}

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
        Self {
            paper,
            code_page: CodePage::DEFAULT,
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
                *self = Self::new(self.paper);
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
            Command::TabStops(columns) => {
                // Must ascend; the printer ignores the rest from the first one that doesn't.
                let mut stops: Vec<u16> = Vec::new();
                for column in columns.into_iter().map(u16::from) {
                    if stops.last().is_some_and(|&last| column <= last) {
                        break;
                    }
                    stops.push(column);
                }
                self.tab_stops = stops;
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
            Command::BarcodeHeight(dots) => self.barcode.height = u16::from(dots.max(1)),
            Command::BarcodeWidth(dots) => self.barcode.module = u16::from(dots.clamp(1, 6)),
            Command::HriPosition(position) => self.barcode.hri = position,
            Command::HriFont(font) => {
                self.barcode.hri_font = if font == 0 { Font::A } else { Font::B };
            }
            Command::Barcode { symbology, data } => {
                self.print_barcode(Symbology::from_code(symbology), &data, out);
            }
            Command::QrModuleSize(dots) => self.qr.module = u16::from(dots.clamp(1, 16)),
            Command::QrErrorCorrection(level) => self.qr.level = level,
            Command::QrStore(data) => self.qr.data = data,
            Command::QrPrint => self.print_qr(out),
            Command::Status(_) | Command::Ignored => {}
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
        // Wrap like the printer: a character that does not fit starts the next line.
        if line.x + glyph_width > line.width && !line.segments.is_empty() {
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
        if let Some(x) = stops
            .iter()
            .map(|&stop| stop.saturating_mul(column))
            .find(|&x| x > line.x && x < line.width)
        {
            line.x = x;
        }
    }

    /// `GS k`: bars as an image band, HRI text above and/or below it, centered on the
    /// bars. Like the printer, a barcode wider than the print area is not printed.
    fn print_barcode(&mut self, symbology: Symbology, data: &[u8], out: &mut Vec<Output>) {
        let Some(encoded) = barcode::encode(symbology, data) else {
            return;
        };
        let settings = self.barcode;
        let widths: Vec<u16> = encoded.dots(settings.module).collect();
        let total: u16 = widths.iter().copied().fold(0, u16::saturating_add);
        let area = self.area_width.min(self.paper.dots() - self.left_margin);
        if total > area {
            return;
        }
        let mut bars = Bitmap::blank(total, settings.height);
        let mut x = 0;
        for (index, width) in widths.into_iter().enumerate() {
            if index % 2 == 0 {
                for dx in x..x + width {
                    for y in 0..settings.height {
                        bars.set(dx, y);
                    }
                }
            }
            x += width;
        }
        self.end_line_if_started(out);
        let left = self.left_margin + self.align_offset(area, total);
        if settings.hri & 1 != 0 {
            self.hri_line(&encoded.hri, left, total, out);
        }
        out.push(Output::Block(Block::Image(Placed {
            x: left,
            bitmap: bars,
        })));
        if settings.hri & 2 != 0 {
            self.hri_line(&encoded.hri, left, total, out);
        }
    }

    fn hri_line(&self, text: &str, left: u16, bars_width: u16, out: &mut Vec<Output>) {
        let font = self.barcode.hri_font;
        let (cell_width, cell_height) = font.cell();
        let text_width = cell_width.saturating_mul(text.chars().count() as u16);
        out.push(Output::Block(Block::Line {
            height: cell_height,
            ascent: cell_height,
            segments: vec![Segment {
                x: left + bars_width.saturating_sub(text_width) / 2,
                text: text.to_owned(),
                font,
                width: 1,
                height: 1,
                advance: cell_width,
                bold: false,
                underline: 0,
                reverse: false,
            }],
            images: Vec::new(),
        }));
    }

    /// `GS ( k` print: the stored data at the selected size and error correction, no quiet
    /// zone, aligned like any image. Data that does not fit a QR prints nothing.
    fn print_qr(&mut self, out: &mut Vec<Output>) {
        if self.qr.data.is_empty() {
            return;
        }
        let level = match self.qr.level {
            49 => qrcode::EcLevel::M,
            50 => qrcode::EcLevel::Q,
            51 => qrcode::EcLevel::H,
            _ => qrcode::EcLevel::L,
        };
        let Ok(code) = qrcode::QrCode::with_error_correction_level(&self.qr.data, level) else {
            return;
        };
        let modules = code.width() as u16;
        let mut bitmap = Bitmap::blank(modules, modules);
        for (index, color) in code.to_colors().into_iter().enumerate() {
            if color == qrcode::Color::Dark {
                bitmap.set(index as u16 % modules, index as u16 / modules);
            }
        }
        let module = self.qr.module;
        self.print_image(bitmap, module, module, out);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::escpos::parser::Parser;

    fn print(paper: Paper, bytes: &[u8]) -> Vec<Output> {
        let mut parser = Parser::default();
        let mut printer = Printer::new(paper);
        let mut out = Vec::new();
        parser.feed(bytes, |command, _| printer.apply(command, &mut out));
        printer.finish(&mut out);
        out
    }

    fn lines(bytes: &[u8]) -> Vec<Vec<Segment>> {
        print(Paper::Mm80, bytes)
            .into_iter()
            .filter_map(|output| match output {
                Output::Block(Block::Line { segments, .. }) => Some(segments),
                _ => None,
            })
            .collect()
    }

    fn texts(segments: &[Segment]) -> Vec<(u16, &str)> {
        segments
            .iter()
            .map(|segment| (segment.x, segment.text.as_str()))
            .collect()
    }

    #[test]
    fn prints_a_line_in_font_a() {
        let out = print(Paper::Mm80, b"\x1b@Hello\n");
        let Output::Block(Block::Line {
            height,
            ascent,
            segments,
            ..
        }) = &out[0]
        else {
            panic!("{out:?}");
        };
        assert_eq!((*height, *ascent), (30, 24));
        assert_eq!(texts(segments), vec![(0, "Hello")]);
        assert_eq!(segments[0].advance, 12);
        assert_eq!(segments[0].font, Font::A);
    }

    #[test]
    fn aligns_center_and_right_within_the_paper() {
        assert_eq!(texts(&lines(b"\x1ba\x01Hi\n")[0]), vec![(276, "Hi")]);
        assert_eq!(texts(&lines(b"\x1ba\x02Hi\n")[0]), vec![(552, "Hi")]);
        let out = print(Paper::Mm58, b"\x1ba\x01Hi\n");
        let Output::Block(Block::Line { segments, .. }) = &out[0] else {
            panic!("{out:?}");
        };
        assert_eq!(segments[0].x, 180);
    }

    #[test]
    fn alignment_applies_from_the_next_line() {
        let all = lines(b"Left\x1ba\x01\nMid\n");
        assert_eq!(all[0][0].x, 0);
        assert_eq!(all[1][0].x, (576 - 36) / 2);
    }

    #[test]
    fn a_style_change_mid_line_stays_on_one_line() {
        let all = lines(b"Total: \x1bE\x011,50\x1bE\x00!\n");
        assert_eq!(all.len(), 1);
        assert_eq!(
            texts(&all[0]),
            vec![(0, "Total: "), (84, "1,50"), (132, "!")]
        );
        assert!(all[0][1].bold && !all[0][0].bold && !all[0][2].bold);
    }

    #[test]
    fn double_size_doubles_cells_and_line_height() {
        let out = print(Paper::Mm80, b"\x1b!\x30AB\n");
        let Output::Block(Block::Line {
            height,
            ascent,
            segments,
            ..
        }) = &out[0]
        else {
            panic!("{out:?}");
        };
        assert_eq!((*height, *ascent), (48, 48));
        assert_eq!(
            (segments[0].width, segments[0].height, segments[0].advance),
            (2, 2, 24)
        );
        let gs = lines(b"\x1d!\x21A\n");
        assert_eq!((gs[0][0].width, gs[0][0].height), (3, 2));
    }

    #[test]
    fn wraps_at_48_columns_in_font_a_and_64_in_font_b() {
        let line_a = [b'x'; 49];
        let all = lines(&[&line_a[..], b"\n"].concat());
        assert_eq!(all.len(), 2);
        assert_eq!(all[0][0].text.len(), 48);
        assert_eq!(texts(&all[1]), vec![(0, "x")]);

        let line_b = [b'y'; 64];
        let all = lines(&[&b"\x1bM\x01"[..], &line_b, b"\n"].concat());
        assert_eq!(all.len(), 1);
        assert_eq!(all[0][0].advance, 9);
    }

    #[test]
    fn empty_lines_and_feeds_become_one_feed_block() {
        let out = print(Paper::Mm80, b"A\n\n\n\x1bJ\x0a\x1bd\x02B\n");
        assert_eq!(out.len(), 3, "{out:?}");
        assert_eq!(
            out[1],
            Output::Block(Block::Feed {
                height: 30 + 30 + 10 + 60
            })
        );
        assert!(matches!(out[2], Output::Block(Block::Line { .. })));
    }

    #[test]
    fn feed_lines_after_text_counts_the_printed_line() {
        let out = print(Paper::Mm80, b"A\x1bd\x03");
        assert!(matches!(
            out[0],
            Output::Block(Block::Line { height: 30, .. })
        ));
        assert_eq!(out[1], Output::Block(Block::Feed { height: 60 }));
    }

    #[test]
    fn decodes_text_in_the_selected_code_page() {
        assert_eq!(lines(b"\x1bt\x02Caf\x82 \x87\n")[0][0].text, "Café ç");
        // 0xC6 is "ã" in CP850 but "╞" in CP437.
        assert_eq!(
            lines(b"\x1bt\x02\x1b@\xc6\n")[0][0].text,
            "╞",
            "ESC @ restores CP437"
        );
    }

    #[test]
    fn initialize_resets_style_and_drops_the_buffer() {
        let all = lines(b"\x1b!\x38lost\x1b@kept\n");
        assert_eq!(all.len(), 1);
        assert_eq!(texts(&all[0]), vec![(0, "kept")]);
        assert_eq!((all[0][0].width, all[0][0].bold), (1, false));
    }

    #[test]
    fn margins_tabs_and_positions() {
        assert_eq!(lines(b"\x1dL\x20\x00A\n")[0][0].x, 32);
        assert_eq!(texts(&lines(b"A\tB\n")[0]), vec![(0, "A"), (96, "B")]);
        assert_eq!(
            texts(&lines(b"\x1bD\x04\x00A\tB\n")[0]),
            vec![(0, "A"), (48, "B")]
        );
        assert_eq!(texts(&lines(b"\x1b$\x64\x00A\n")[0]), vec![(100, "A")]);
        assert_eq!(
            texts(&lines(b"A\x1b\\\x0a\x00B\n")[0]),
            vec![(0, "A"), (22, "B")]
        );
        // Centered inside a 288-dot area that starts at 32.
        assert_eq!(
            lines(b"\x1dL\x20\x00\x1dW\x20\x01\x1ba\x01AB\n")[0][0].x,
            32 + 132
        );
    }

    #[test]
    fn character_spacing_widens_the_advance() {
        let all = lines(b"\x1b \x02\x1b!\x20AB\n");
        assert_eq!(all[0][0].advance, (12 + 2) * 2);
    }

    #[test]
    fn cut_ends_the_line_and_feeds_first() {
        let out = print(Paper::Mm80, b"A\x1dVB\x10");
        assert!(matches!(out[0], Output::Block(Block::Line { .. })));
        assert_eq!(out[1], Output::Block(Block::Feed { height: 16 }));
        assert_eq!(out[2], Output::Cut { partial: true });
        assert_eq!(
            print(Paper::Mm80, b"\x1dV\x00"),
            vec![Output::Cut { partial: false }]
        );
    }

    #[test]
    fn side_effects_and_unknown_commands() {
        assert_eq!(
            print(Paper::Mm80, b"\x1bp\x00\x19\xfa\x1bB\x01\x01"),
            vec![Output::DrawerPulse, Output::Beep]
        );
        let mut printer = Printer::new(Paper::Mm80);
        let mut out = Vec::new();
        printer.apply(Command::Unknown([0x1b, 0x01]), &mut out);
        assert_eq!(printer.unknown_commands, 1);
    }

    #[test]
    fn text_without_a_final_line_feed_still_prints() {
        assert_eq!(
            texts(&lines(b"\x1b@no newline")[0]),
            vec![(0, "no newline")]
        );
    }

    #[test]
    fn random_bytes_never_panic() {
        let mut state: u64 = 0x2545_f491_4f6c_dd1d;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..2_000 {
            let len = (next() % 512) as usize;
            let bytes: Vec<u8> = (0..len)
                .map(|_| match next() % 4 {
                    0 => [0x1b, 0x1d, 0x0a, 0x09][(next() % 4) as usize],
                    _ => next() as u8,
                })
                .collect();
            for paper in [Paper::Mm80, Paper::Mm58] {
                for output in print(paper, &bytes) {
                    if let Output::Block(Block::Line { segments, .. }) = output {
                        assert!(segments.iter().all(|segment| segment.x < paper.dots() * 2));
                    }
                }
            }
        }
    }

    fn images(bytes: &[u8]) -> Vec<Placed> {
        print(Paper::Mm80, bytes)
            .into_iter()
            .filter_map(|output| match output {
                Output::Block(Block::Image(placed)) => Some(placed),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn raster_images_are_aligned_scaled_and_cropped() {
        let centered = images(b"\x1ba\x01\x1dv0\x00\x01\x00\x02\x00\xff\x81");
        assert_eq!(
            (
                centered[0].x,
                centered[0].bitmap.width,
                centered[0].bitmap.height
            ),
            (284, 8, 2)
        );
        let quadruple = images(b"\x1dv0\x03\x01\x00\x01\x00\x80");
        assert_eq!(
            (quadruple[0].bitmap.width, quadruple[0].bitmap.height),
            (16, 2)
        );
        assert!(quadruple[0].bitmap.get(1, 1) && !quadruple[0].bitmap.get(2, 0));
        let mut wide = b"\x1dv0\x00\x50\x00\x01\x00".to_vec();
        wide.extend([0xff; 80]);
        assert_eq!(images(&wide)[0].bitmap.width, 576, "cropped to the paper");
    }

    #[test]
    fn raster_after_text_prints_the_text_first() {
        let out = print(Paper::Mm80, b"Logo:\x1dv0\x00\x01\x00\x01\x00\xff");
        assert!(matches!(out[0], Output::Block(Block::Line { .. })));
        assert!(matches!(out[1], Output::Block(Block::Image(_))));
    }

    #[test]
    fn bit_image_stripes_stack_without_gaps() {
        // ESC 3 24, then two 24-dot double-density stripes of 2 columns, each ended by LF.
        let stripe = b"\x1b*\x21\x02\x00\xff\x00\x00\x00\x00\x01\n";
        let out = print(Paper::Mm80, &[&b"\x1b3\x18"[..], stripe, stripe].concat());
        assert_eq!(out.len(), 2, "{out:?}");
        for block in &out {
            let Output::Block(Block::Line {
                height,
                images,
                segments,
                ..
            }) = block
            else {
                panic!("{block:?}");
            };
            assert_eq!(*height, 24);
            assert!(segments.is_empty());
            let bitmap = &images[0].bitmap;
            assert_eq!((bitmap.width, bitmap.height), (2, 24));
            assert!(bitmap.get(0, 0) && bitmap.get(0, 7) && !bitmap.get(0, 8));
            assert!(bitmap.get(1, 23) && !bitmap.get(1, 0));
        }
    }

    #[test]
    fn eight_dot_single_density_is_stretched() {
        let out = print(Paper::Mm80, b"\x1b*\x00\x01\x00\x80\n");
        let Output::Block(Block::Line { images, height, .. }) = &out[0] else {
            panic!("{out:?}");
        };
        let bitmap = &images[0].bitmap;
        assert_eq!((bitmap.width, bitmap.height), (2, 24));
        assert!(bitmap.get(1, 2) && !bitmap.get(0, 3));
        assert_eq!(*height, 30, "the line spacing is taller than the stripe");
    }

    #[test]
    fn stored_graphics_print_on_request() {
        let store = b"\x1d(L\x0b\x000p0\x02\x01\x31\x08\x00\x01\x00\xf0";
        assert!(images(store).is_empty(), "storing prints nothing");
        let printed = images(&[&store[..], b"\x1ba\x02\x1d(L\x02\x0002"].concat());
        assert_eq!((printed[0].x, printed[0].bitmap.width), (576 - 16, 16));
        assert!(images(b"\x1d(L\x02\x0002").is_empty(), "nothing stored");
    }

    #[test]
    fn barcodes_are_centered_with_hri_below() {
        let out = print(Paper::Mm80, b"\x1ba\x01\x1dH\x02\x1dk\x02590123412345\x00");
        let Output::Block(Block::Image(bars)) = &out[0] else {
            panic!("{out:?}");
        };
        // EAN-13 is 95 modules of 3 dots, 162 dots tall by default.
        assert_eq!(
            (bars.x, bars.bitmap.width, bars.bitmap.height),
            (145, 285, 162)
        );
        assert!(bars.bitmap.get(0, 0) && !bars.bitmap.get(3, 0) && bars.bitmap.get(6, 161));
        let Output::Block(Block::Line { segments, .. }) = &out[1] else {
            panic!("{out:?}");
        };
        assert_eq!(
            texts(segments),
            vec![(145 + (285 - 13 * 12) / 2, "5901234123457")]
        );
    }

    #[test]
    fn barcode_settings_and_hri_above() {
        let out = print(
            Paper::Mm80,
            b"\x1dh\x28\x1dw\x02\x1dH\x01\x1df\x01\x1dkI\x06{BAb12",
        );
        assert!(
            matches!(&out[0], Output::Block(Block::Line { segments, .. })
            if segments[0].text == "Ab12" && segments[0].font == Font::B)
        );
        let Output::Block(Block::Image(bars)) = &out[1] else {
            panic!("{out:?}");
        };
        // CODE128: start + 4 characters + checksum (11 modules each) + stop (13), 2 dots.
        assert_eq!(
            (bars.bitmap.width, bars.bitmap.height),
            ((6 * 11 + 13) * 2, 40)
        );
    }

    #[test]
    fn barcodes_wider_than_the_paper_or_invalid_are_skipped() {
        let long = [&b"\x1dw\x06\x1dk\x49\x20{B"[..], &[b'X'; 30]].concat();
        assert!(images(&long).is_empty());
        assert!(
            images(b"\x1dk\x02ABC\x00").is_empty(),
            "EAN-13 takes digits"
        );
    }

    #[test]
    fn qr_codes_use_the_stored_data_and_module_size() {
        let qr = b"\x1ba\x01\x1d(k\x03\x00\x31\x43\x04\x1d(k\x05\x00\x31\x50\x30hi\x1d(k\x03\x00\x31\x51\x30";
        let printed = images(qr);
        // "hi" fits version 1: 21 modules of 4 dots.
        assert_eq!(
            (
                printed[0].x,
                printed[0].bitmap.width,
                printed[0].bitmap.height
            ),
            (246, 84, 84)
        );
        // Top-left finder pattern: dark corner, light ring at module 1.
        assert!(printed[0].bitmap.get(0, 0) && printed[0].bitmap.get(3, 3));
        assert!(!printed[0].bitmap.get(4, 4));
        assert!(
            images(b"\x1d(k\x03\x00\x31\x51\x30").is_empty(),
            "nothing stored"
        );
    }
}
