//! A receipt's raw bytes as a list of commands, for the "Commands" view: what the POS sent
//! and how the emulator read it. Re-parses with the same parser the printer used.

use serde::Serialize;

use super::codepage::CodePage;
use super::command::{Alignment, Command, StatusRequest};
use super::parser::Parser;

/// More rows than anyone reads; keeps a huge receipt from freezing the window.
pub const MAX_ROWS: usize = 5000;
/// Raw bytes shown per row; data-heavy commands (images) are cut with `…`.
const MAX_BYTES_SHOWN: usize = 16;
const MAX_TEXT_SHOWN: usize = 60;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommandRow {
    /// Byte offset of the command in the receipt.
    pub offset: u64,
    pub length: usize,
    /// Hex of the first bytes, `…` when cut.
    pub bytes: String,
    /// As written in the ESC/POS reference: `ESC a`, `GS V`, `LF`. Empty for text.
    pub mnemonic: String,
    /// What it does, as an i18n key under `inspect.kinds` (`align_center`, `cut_partial`).
    pub kind: &'static str,
    /// Parameters in the reference's notation (`n=1`, `96×32`, `CP850`) or the text itself.
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Inspection {
    pub rows: Vec<CommandRow>,
    /// More commands than `MAX_ROWS`; only the first ones are listed.
    pub truncated: bool,
}

pub fn inspect(raw: &[u8], default_code_page: CodePage) -> Inspection {
    let mut parser = Parser::default();
    let mut page = default_code_page;
    let mut rows = Vec::new();
    let mut start = 0u64;
    let mut truncated = false;
    parser.feed(raw, |command, end| {
        let bytes = &raw[start as usize..end as usize];
        if rows.len() < MAX_ROWS {
            let (kind, detail) = describe(&command, &mut page, default_code_page);
            rows.push(CommandRow {
                offset: start,
                length: bytes.len(),
                bytes: hex(bytes),
                mnemonic: mnemonic(&command, bytes),
                kind,
                detail,
            });
        } else {
            truncated = true;
        }
        start = end;
    });
    Inspection { rows, truncated }
}

fn hex(bytes: &[u8]) -> String {
    let shown: Vec<String> = bytes
        .iter()
        .take(MAX_BYTES_SHOWN)
        .map(|byte| format!("{byte:02X}"))
        .collect();
    let mut text = shown.join(" ");
    if bytes.len() > MAX_BYTES_SHOWN {
        text.push_str(" …");
    }
    text
}

fn byte_name(byte: u8) -> String {
    match byte {
        0x0a => "LF".into(),
        0x0d => "CR".into(),
        0x09 => "HT".into(),
        0x10 => "DLE".into(),
        0x1b => "ESC".into(),
        0x1c => "FS".into(),
        0x1d => "GS".into(),
        0x04 => "EOT".into(),
        0x05 => "ENQ".into(),
        0x14 => "DC4".into(),
        0x0c => "FF".into(),
        b' ' => "SP".into(),
        0x21..=0x7e => char::from(byte).to_string(),
        _ => format!("0x{byte:02X}"),
    }
}

/// `ESC a`, `GS ( k`, `DLE EOT`: the prefix and the function byte(s), as the manual writes.
fn mnemonic(command: &Command, bytes: &[u8]) -> String {
    match (command, bytes) {
        (Command::Text(_), _) => String::new(),
        (_, [first @ 0x1b..=0x1d, b'(' | b'8', function, ..]) => {
            format!(
                "{} {} {}",
                byte_name(*first),
                char::from(bytes[1]),
                byte_name(*function)
            )
        }
        (_, [first @ (0x10 | 0x1b | 0x1c | 0x1d), second, ..]) => {
            format!("{} {}", byte_name(*first), byte_name(*second))
        }
        (_, [only, ..]) => byte_name(*only),
        (_, []) => String::new(),
    }
}

fn shorten(text: String, max: usize) -> String {
    if text.chars().count() <= max {
        return text;
    }
    let mut short: String = text.chars().take(max).collect();
    short.push('…');
    short
}

fn printable(bytes: &[u8]) -> String {
    shorten(String::from_utf8_lossy(bytes).into_owned(), MAX_TEXT_SHOWN)
}

/// The i18n kind and the detail. Tracks `ESC t` / `ESC @` so text shows in the code page
/// the printer used.
fn describe(command: &Command, page: &mut CodePage, default: CodePage) -> (&'static str, String) {
    let n = |value: &dyn std::fmt::Display| format!("n={value}");
    match command {
        Command::Text(bytes) => {
            let text: String = bytes.iter().map(|&byte| page.decode(byte)).collect();
            ("text", shorten(text, MAX_TEXT_SHOWN))
        }
        Command::LineFeed => ("line_feed", String::new()),
        Command::CarriageReturn => ("carriage_return", String::new()),
        Command::Tab => ("tab", String::new()),
        Command::Initialize => {
            *page = default;
            ("initialize", String::new())
        }
        Command::PrintMode(mode) => ("print_mode", format!("n=0x{mode:02X}")),
        Command::Emphasis(true) => ("emphasis_on", String::new()),
        Command::Emphasis(false) => ("emphasis_off", String::new()),
        Command::Underline(thickness) => ("underline", n(thickness)),
        Command::Align(Alignment::Left) => ("align_left", String::new()),
        Command::Align(Alignment::Center) => ("align_center", String::new()),
        Command::Align(Alignment::Right) => ("align_right", String::new()),
        Command::Font(font) => ("font", if *font == 0 { "A".into() } else { "B".into() }),
        Command::CharSize { width, height } => ("char_size", format!("{width}×{height}")),
        Command::Reverse(true) => ("reverse_on", String::new()),
        Command::Reverse(false) => ("reverse_off", String::new()),
        Command::CodeTable(table) => match CodePage::from_table(*table) {
            Some(selected) => {
                *page = selected;
                ("code_table", selected.name().into())
            }
            None => ("code_table_unsupported", n(table)),
        },
        Command::CharSpacing(dots) => ("char_spacing", n(dots)),
        Command::LineSpacing(None) => ("line_spacing_default", String::new()),
        Command::LineSpacing(Some(dots)) => ("line_spacing", n(dots)),
        Command::FeedDots(dots) => ("feed_dots", n(dots)),
        Command::FeedLines(lines) => ("feed_lines", n(lines)),
        Command::LeftMargin(dots) => ("left_margin", n(dots)),
        Command::PrintAreaWidth(dots) => ("area_width", n(dots)),
        Command::AbsolutePosition(dots) => ("absolute_position", n(dots)),
        Command::RelativePosition(dots) => ("relative_position", n(dots)),
        Command::TabStops(columns) => (
            "tab_stops",
            columns
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(", "),
        ),
        Command::Cut {
            partial: false,
            feed,
        } => ("cut_full", n(feed)),
        Command::Cut {
            partial: true,
            feed,
        } => ("cut_partial", n(feed)),
        Command::DrawerPulse => ("drawer", String::new()),
        Command::Beep => ("beep", String::new()),
        Command::Status(StatusRequest::Realtime(request)) => ("status_realtime", n(request)),
        Command::Status(StatusRequest::Status(request)) => ("status", n(request)),
        Command::Status(StatusRequest::PrinterId(request)) => ("printer_id", n(request)),
        Command::Raster {
            mode,
            row_bytes,
            height,
            ..
        } => (
            "raster",
            format!("{}×{height} m={mode}", u32::from(*row_bytes) * 8),
        ),
        Command::BitImage { mode, columns, .. } => ("bit_image", format!("m={mode} {columns}")),
        Command::StoreGraphics { width, height, .. } => {
            ("graphics_store", format!("{width}×{height}"))
        }
        Command::PrintGraphics => ("graphics_print", String::new()),
        Command::BarcodeHeight(dots) => ("barcode_height", n(dots)),
        Command::BarcodeWidth(dots) => ("barcode_width", n(dots)),
        Command::HriPosition(position) => ("hri_position", n(position)),
        Command::HriFont(font) => ("hri_font", n(font)),
        Command::Barcode { symbology, data } => {
            ("barcode", format!("m={symbology} {}", printable(data)))
        }
        Command::QrModuleSize(dots) => ("qr_size", n(dots)),
        Command::QrErrorCorrection(level) => (
            "qr_level",
            match level {
                49 => "M",
                50 => "Q",
                51 => "H",
                _ => "L",
            }
            .into(),
        ),
        Command::QrStore(data) => ("qr_store", printable(data)),
        Command::QrPrint => ("qr_print", String::new()),
        Command::Ignored => ("ignored", String::new()),
        Command::Unknown(_) => ("unknown", String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(raw: &[u8]) -> Vec<CommandRow> {
        inspect(raw, CodePage::DEFAULT).rows
    }

    #[test]
    fn lists_commands_with_offsets_and_mnemonics() {
        let all = rows(b"\x1b@\x1ba\x01Hi\n\x1dVB\x10");
        let summary: Vec<(u64, &str, &str, &str)> = all
            .iter()
            .map(|row| {
                (
                    row.offset,
                    row.mnemonic.as_str(),
                    row.kind,
                    row.detail.as_str(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            vec![
                (0, "ESC @", "initialize", ""),
                (2, "ESC a", "align_center", ""),
                (5, "", "text", "Hi"),
                (7, "LF", "line_feed", ""),
                (8, "GS V", "cut_partial", "n=16"),
            ]
        );
        assert_eq!(all[4].bytes, "1D 56 42 10");
    }

    #[test]
    fn text_follows_the_selected_code_page() {
        let all = rows(b"\x1bt\x02\xc6\x1b@\xc6");
        assert_eq!(all[0].detail, "CP850");
        assert_eq!(all[1].detail, "ã");
        assert_eq!(all[3].detail, "╞", "ESC @ goes back to the default");
    }

    #[test]
    fn data_heavy_commands_are_summarized() {
        let mut raster = b"\x1dv0\x00\x0c\x00\x20\x00".to_vec();
        raster.extend([0xff; 12 * 32]);
        let row = &rows(&raster)[0];
        assert_eq!(
            (row.mnemonic.as_str(), row.kind, row.detail.as_str()),
            ("GS v", "raster", "96×32 m=0")
        );
        assert!(row.bytes.ends_with('…'));
        assert_eq!(row.length, 8 + 12 * 32);
        let qr = rows(b"\x1d(k\x05\x00\x31\x50\x30hi");
        assert_eq!(
            (qr[0].mnemonic.as_str(), qr[0].kind, qr[0].detail.as_str()),
            ("GS ( k", "qr_store", "hi")
        );
    }

    #[test]
    fn caps_the_number_of_rows() {
        let raw = b"\n".repeat(MAX_ROWS + 10);
        let inspection = inspect(&raw, CodePage::DEFAULT);
        assert_eq!(inspection.rows.len(), MAX_ROWS);
        assert!(inspection.truncated);
    }
}
