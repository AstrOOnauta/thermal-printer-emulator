//! The print model: what the printer produces and the webview draws (serialized as is),
//! plus the side effects of a command. Geometry in dots of a 203 dpi printer.

use serde::{Deserialize, Serialize};

use super::bitmap::Bitmap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

impl Block {
    /// Paper it takes, in dots.
    pub fn height(&self) -> u16 {
        match self {
            Self::Line { height, .. } | Self::Feed { height } => *height,
            Self::Image(placed) => placed.bitmap.height,
        }
    }
}

/// What applying a command produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Output {
    Block(Block),
    Cut {
        partial: bool,
    },
    DrawerPulse,
    Beep,
    /// Bytes to send back to the POS (status requests).
    Reply(Vec<u8>),
}
