//! What the parser makes of the bytes: one `Command` per ESC/POS command, with its
//! parameters. The printer (`printer.rs`) and the Commands view (`inspect.rs`) consume them.

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
