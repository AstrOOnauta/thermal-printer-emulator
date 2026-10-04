//! 1-bit images: rows top to bottom, each `stride` bytes, most significant bit leftmost,
//! 1 = black. Sent to the webview as base64 and drawn without any image format in between.

use base64::Engine as _;
use serde::{Serialize, Serializer};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Bitmap {
    pub width: u16,
    pub height: u16,
    #[serde(serialize_with = "base64")]
    pub data: Vec<u8>,
}

fn base64<S: Serializer>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&base64::engine::general_purpose::STANDARD.encode(data))
}

fn stride(width: u16) -> usize {
    usize::from(width).div_ceil(8)
}

impl Bitmap {
    pub fn blank(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            data: vec![0; stride(width) * usize::from(height)],
        }
    }

    /// From raster rows of `row_bytes` each (the ESC/POS layout). Short data leaves the
    /// rest white.
    pub fn from_rows(row_bytes: u16, height: u16, rows: &[u8]) -> Self {
        let mut bitmap = Self::blank(row_bytes.saturating_mul(8), height);
        let len = bitmap.data.len().min(rows.len());
        bitmap.data[..len].copy_from_slice(&rows[..len]);
        bitmap
    }

    pub fn get(&self, x: u16, y: u16) -> bool {
        let byte = self.data[usize::from(y) * stride(self.width) + usize::from(x / 8)];
        byte & (0x80 >> (x % 8)) != 0
    }

    pub fn set(&mut self, x: u16, y: u16) {
        let index = usize::from(y) * stride(self.width) + usize::from(x / 8);
        self.data[index] |= 0x80 >> (x % 8);
    }

    /// Every dot becomes a `sx × sy` block.
    pub fn scaled(self, sx: u16, sy: u16) -> Self {
        if sx == 1 && sy == 1 {
            return self;
        }
        let mut scaled = Self::blank(
            self.width.saturating_mul(sx),
            self.height.saturating_mul(sy),
        );
        for y in 0..scaled.height {
            for x in 0..scaled.width {
                if self.get(x / sx, y / sy) {
                    scaled.set(x, y);
                }
            }
        }
        scaled
    }

    /// Drops the columns past `max_width`: the printer does not print outside its area.
    pub fn cropped(self, max_width: u16) -> Self {
        if self.width <= max_width {
            return self;
        }
        let mut cropped = Self::blank(max_width, self.height);
        for y in 0..self.height {
            for x in 0..max_width {
                if self.get(x, y) {
                    cropped.set(x, y);
                }
            }
        }
        cropped
    }
}

#[cfg(test)]
mod tests {
    use super::Bitmap;

    fn ascii(bitmap: &Bitmap) -> Vec<String> {
        (0..bitmap.height)
            .map(|y| {
                (0..bitmap.width)
                    .map(|x| if bitmap.get(x, y) { '#' } else { '.' })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn rows_scale_and_crop() {
        let bitmap = Bitmap::from_rows(1, 2, &[0b1000_0001, 0b0100_0000]);
        assert_eq!(ascii(&bitmap), vec!["#......#", ".#......"]);
        let scaled = bitmap.clone().scaled(2, 1);
        assert_eq!(scaled.width, 16);
        assert_eq!(ascii(&scaled)[1], "..##............");
        assert_eq!(ascii(&bitmap.cropped(3)), vec!["#..", ".#."]);
    }

    #[test]
    fn short_data_stays_white() {
        let bitmap = Bitmap::from_rows(2, 2, &[0xff]);
        assert_eq!(ascii(&bitmap), vec!["########........", "................"]);
    }

    #[test]
    fn serializes_data_as_base64() {
        let json = serde_json::to_string(&Bitmap::from_rows(1, 1, &[0xf0])).expect("serializes");
        assert_eq!(json, r#"{"width":8,"height":1,"data":"8A=="}"#);
    }
}
