//! Barcodes (`GS k`) and QR codes (`GS ( k`) as image bands, with their HRI text.

use super::Printer;
use crate::escpos::barcode::{self, Symbology};
use crate::escpos::bitmap::Bitmap;
use crate::escpos::model::{Block, Output, Placed, Segment};

impl Printer {
    /// `GS k`: bars as an image band, HRI text above and/or below it, centered on the
    /// bars. Like the printer, a barcode wider than the print area is not printed.
    pub(super) fn print_barcode(
        &mut self,
        symbology: Symbology,
        data: &[u8],
        out: &mut Vec<Output>,
    ) {
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
    /// zone, aligned like any image. Data that does not fit a QR prints nothing, and so does
    /// a symbol wider than the print area, like a barcode: cropped, it could not be read.
    pub(super) fn print_qr(&mut self, out: &mut Vec<Output>) {
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
        let area = self.area_width.min(self.paper.dots() - self.left_margin);
        if modules.saturating_mul(self.qr.module) > area {
            return;
        }
        let mut bitmap = Bitmap::blank(modules, modules);
        for (index, color) in code.to_colors().into_iter().enumerate() {
            if color == qrcode::Color::Dark {
                bitmap.set(index as u16 % modules, index as u16 / modules);
            }
        }
        let module = self.qr.module;
        self.print_image(bitmap, module, module, out);
    }
}
