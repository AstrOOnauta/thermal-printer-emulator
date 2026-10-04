//! `GS k` barcode symbologies, from their public specifications. Each encoder returns the
//! bar/space widths (starting with a bar) and the human-readable text (HRI).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Symbology {
    UpcA,
    Ean13,
    Ean8,
    Code39,
    Itf,
    Codabar,
    Code128,
    /// UPC-E, CODE93, GS1 DataBar…: parsed, not drawn.
    Unsupported,
}

impl Symbology {
    /// `GS k m`: m 0–6 (NUL-terminated data) or 65–79 (counted data).
    pub fn from_code(m: u8) -> Self {
        match m {
            0 | 65 => Self::UpcA,
            2 | 67 => Self::Ean13,
            3 | 68 => Self::Ean8,
            4 | 69 => Self::Code39,
            5 | 70 => Self::Itf,
            6 | 71 => Self::Codabar,
            73 => Self::Code128,
            _ => Self::Unsupported,
        }
    }
}

/// How the widths are counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Widths {
    /// Each width is a number of modules (EAN, UPC, CODE128).
    Modules,
    /// Each width is 1 (narrow) or 2 (wide) (CODE39, ITF, CODABAR).
    NarrowWide,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Encoded {
    /// Alternating bar, space, bar…
    pub widths: Vec<u8>,
    pub kind: Widths,
    pub hri: String,
}

impl Encoded {
    /// Width in dots of each element for a `GS w` module width.
    pub fn dots(&self, module: u16) -> impl Iterator<Item = u16> + '_ {
        // Epson's wide elements: 2→5, 3→8, 4→10, 5→13, 6→15 dots.
        let wide = (5 * module).div_ceil(2);
        self.widths.iter().map(move |&width| match self.kind {
            Widths::Modules => u16::from(width) * module,
            Widths::NarrowWide if width == 2 => wide,
            Widths::NarrowWide => module,
        })
    }
}

pub fn encode(symbology: Symbology, data: &[u8]) -> Option<Encoded> {
    match symbology {
        Symbology::UpcA => upc_a(data),
        Symbology::Ean13 => ean13(data),
        Symbology::Ean8 => ean8(data),
        Symbology::Code39 => code39(data),
        Symbology::Itf => itf(data),
        Symbology::Codabar => codabar(data),
        Symbology::Code128 => code128(data),
        Symbology::Unsupported => None,
    }
}

/// `0`/`1` module patterns → run lengths, starting with a bar.
fn runs(modules: &str) -> Vec<u8> {
    let mut widths: Vec<u8> = Vec::new();
    let mut previous = '1';
    for (index, module) in modules.chars().enumerate() {
        if index > 0 && module == previous {
            *widths.last_mut().expect("pushed below") += 1;
        } else {
            widths.push(1);
        }
        previous = module;
    }
    widths
}

fn digits(data: &[u8]) -> Option<Vec<u8>> {
    data.iter()
        .map(|&byte| byte.is_ascii_digit().then(|| byte - b'0'))
        .collect()
}

/// EAN/UPC check digit: weights 3, 1, 3… from the rightmost data digit.
fn check_digit(digits: &[u8]) -> u8 {
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(index, &digit)| u32::from(digit) * if index % 2 == 0 { 3 } else { 1 })
        .sum();
    ((10 - sum % 10) % 10) as u8
}

const EAN_L: [&str; 10] = [
    "0001101", "0011001", "0010011", "0111101", "0100011", "0110001", "0101111", "0111011",
    "0110111", "0001011",
];
const EAN_G: [&str; 10] = [
    "0100111", "0110011", "0011011", "0100001", "0011101", "0111001", "0000101", "0010001",
    "0001001", "0010111",
];
const EAN_R: [&str; 10] = [
    "1110010", "1100110", "1101100", "1000010", "1011100", "1001110", "1010000", "1000100",
    "1001000", "1110100",
];
/// EAN-13: the first digit picks L or G for each digit of the left half.
const EAN13_PARITY: [&str; 10] = [
    "LLLLLL", "LLGLGG", "LLGGLG", "LLGGGL", "LGLLGG", "LGGLLG", "LGGGLL", "LGLGLG", "LGLGGL",
    "LGGLGL",
];

/// Digits with their check digit: `data_len` digits get one computed, one more is taken
/// as given.
fn with_check(data: &[u8], data_len: usize) -> Option<Vec<u8>> {
    let mut digits = digits(data)?;
    match digits.len() {
        len if len == data_len => digits.push(check_digit(&digits)),
        len if len == data_len + 1 => {}
        _ => return None,
    }
    Some(digits)
}

fn ean13_digits(digits: &[u8]) -> Encoded {
    let parity = EAN13_PARITY[usize::from(digits[0])].as_bytes();
    let mut modules = String::from("101");
    for (index, &digit) in digits[1..7].iter().enumerate() {
        let table = if parity[index] == b'L' {
            &EAN_L
        } else {
            &EAN_G
        };
        modules.push_str(table[usize::from(digit)]);
    }
    modules.push_str("01010");
    for &digit in &digits[7..13] {
        modules.push_str(EAN_R[usize::from(digit)]);
    }
    modules.push_str("101");
    Encoded {
        widths: runs(&modules),
        kind: Widths::Modules,
        hri: digits
            .iter()
            .map(|digit| char::from(b'0' + digit))
            .collect(),
    }
}

fn ean13(data: &[u8]) -> Option<Encoded> {
    Some(ean13_digits(&with_check(data, 12)?))
}

/// UPC-A is EAN-13 with a leading 0; its HRI keeps the 12 UPC digits.
fn upc_a(data: &[u8]) -> Option<Encoded> {
    let digits = with_check(data, 11)?;
    let mut ean = vec![0];
    ean.extend_from_slice(&digits);
    let mut encoded = ean13_digits(&ean);
    encoded.hri.remove(0);
    Some(encoded)
}

fn ean8(data: &[u8]) -> Option<Encoded> {
    let digits = with_check(data, 7)?;
    let mut modules = String::from("101");
    for &digit in &digits[..4] {
        modules.push_str(EAN_L[usize::from(digit)]);
    }
    modules.push_str("01010");
    for &digit in &digits[4..] {
        modules.push_str(EAN_R[usize::from(digit)]);
    }
    modules.push_str("101");
    Some(Encoded {
        widths: runs(&modules),
        kind: Widths::Modules,
        hri: digits
            .iter()
            .map(|digit| char::from(b'0' + digit))
            .collect(),
    })
}

/// CODE39: 9 elements per character, `1` = wide.
fn code39_pattern(character: u8) -> Option<&'static str> {
    Some(match character {
        b'0' => "000110100",
        b'1' => "100100001",
        b'2' => "001100001",
        b'3' => "101100000",
        b'4' => "000110001",
        b'5' => "100110000",
        b'6' => "001110000",
        b'7' => "000100101",
        b'8' => "100100100",
        b'9' => "001100100",
        b'A' => "100001001",
        b'B' => "001001001",
        b'C' => "101001000",
        b'D' => "000011001",
        b'E' => "100011000",
        b'F' => "001011000",
        b'G' => "000001101",
        b'H' => "100001100",
        b'I' => "001001100",
        b'J' => "000011100",
        b'K' => "100000011",
        b'L' => "001000011",
        b'M' => "101000010",
        b'N' => "000010011",
        b'O' => "100010010",
        b'P' => "001010010",
        b'Q' => "000000111",
        b'R' => "100000110",
        b'S' => "001000110",
        b'T' => "000010110",
        b'U' => "110000001",
        b'V' => "011000001",
        b'W' => "111000000",
        b'X' => "010010001",
        b'Y' => "110010000",
        b'Z' => "011010000",
        b'-' => "010000101",
        b'.' => "110000100",
        b' ' => "011000100",
        b'$' => "010101000",
        b'/' => "010100010",
        b'+' => "010001010",
        b'%' => "000101010",
        b'*' => "010010100",
        _ => return None,
    })
}

/// Narrow/wide patterns joined by a narrow space between characters.
fn narrow_wide(patterns: &[&str]) -> Vec<u8> {
    let mut widths = Vec::new();
    for (index, pattern) in patterns.iter().enumerate() {
        if index > 0 {
            widths.push(1);
        }
        widths.extend(
            pattern
                .bytes()
                .map(|element| if element == b'1' { 2 } else { 1 }),
        );
    }
    widths
}

/// The printer adds the `*` start/stop characters when the data does not have them.
fn code39(data: &[u8]) -> Option<Encoded> {
    let inner = data
        .strip_prefix(b"*")
        .and_then(|rest| rest.strip_suffix(b"*"))
        .unwrap_or(data);
    if inner.is_empty() || inner.contains(&b'*') {
        return None;
    }
    let mut text = vec![b'*'];
    text.extend_from_slice(inner);
    text.push(b'*');
    let patterns: Option<Vec<&str>> = text.iter().map(|&byte| code39_pattern(byte)).collect();
    Some(Encoded {
        widths: narrow_wide(&patterns?),
        kind: Widths::NarrowWide,
        hri: String::from_utf8_lossy(&text).into_owned(),
    })
}

/// ITF: 5 elements per digit, 2 wide; digit pairs interleave bars and spaces.
const ITF_DIGIT: [&str; 10] = [
    "00110", "10001", "01001", "11000", "00101", "10100", "01100", "00011", "10010", "01010",
];

fn itf(data: &[u8]) -> Option<Encoded> {
    let digits = digits(data)?;
    if digits.is_empty() || digits.len() % 2 != 0 {
        return None;
    }
    let mut widths = vec![1, 1, 1, 1];
    for pair in digits.chunks(2) {
        let bars = ITF_DIGIT[usize::from(pair[0])].as_bytes();
        let spaces = ITF_DIGIT[usize::from(pair[1])].as_bytes();
        for (bar, space) in bars.iter().zip(spaces) {
            widths.push(if *bar == b'1' { 2 } else { 1 });
            widths.push(if *space == b'1' { 2 } else { 1 });
        }
    }
    widths.extend([2, 1, 1]);
    Some(Encoded {
        widths,
        kind: Widths::NarrowWide,
        hri: String::from_utf8_lossy(data).into_owned(),
    })
}

/// CODABAR: 7 elements per character, `1` = wide. Data starts and ends with A–D.
fn codabar_pattern(character: u8) -> Option<&'static str> {
    Some(match character.to_ascii_uppercase() {
        b'0' => "0000011",
        b'1' => "0000110",
        b'2' => "0001001",
        b'3' => "1100000",
        b'4' => "0010010",
        b'5' => "1000010",
        b'6' => "0100001",
        b'7' => "0100100",
        b'8' => "0110000",
        b'9' => "1001000",
        b'-' => "0001100",
        b'$' => "0011000",
        b':' => "1000101",
        b'/' => "1010001",
        b'.' => "1010100",
        b'+' => "0010101",
        b'A' => "0011010",
        b'B' => "0101001",
        b'C' => "0001011",
        b'D' => "0001110",
        _ => return None,
    })
}

fn codabar(data: &[u8]) -> Option<Encoded> {
    let is_guard = |byte: &u8| matches!(byte.to_ascii_uppercase(), b'A'..=b'D');
    if data.len() < 2 || !is_guard(&data[0]) || !is_guard(&data[data.len() - 1]) {
        return None;
    }
    let patterns: Option<Vec<&str>> = data.iter().map(|&byte| codabar_pattern(byte)).collect();
    Some(Encoded {
        widths: narrow_wide(&patterns?),
        kind: Widths::NarrowWide,
        hri: String::from_utf8_lossy(data).into_owned(),
    })
}

/// CODE128 symbol widths (bar, space, …), values 0–105; 106 is the stop.
const CODE128: [&str; 107] = [
    "212222", "222122", "222221", "121223", "121322", "131222", "122213", "122312", "132212",
    "221213", "221312", "231212", "112232", "122132", "122231", "113222", "123122", "123221",
    "223211", "221132", "221231", "213212", "223112", "312131", "311222", "321122", "321221",
    "312212", "322112", "322211", "212123", "212321", "232121", "111323", "131123", "131321",
    "112313", "132113", "132311", "211313", "231113", "231311", "112133", "112331", "132131",
    "113123", "113321", "133121", "313121", "211331", "231131", "213113", "213311", "213131",
    "311123", "311321", "331121", "312113", "312311", "332111", "314111", "221411", "431111",
    "111224", "111422", "121124", "121421", "141122", "141221", "112214", "112412", "122114",
    "122411", "142112", "142211", "241211", "221114", "413111", "241112", "134111", "111242",
    "121142", "121241", "114212", "124112", "124211", "411212", "421112", "421211", "212141",
    "214121", "412121", "111143", "111341", "131141", "114113", "114311", "411113", "411311",
    "113141", "114131", "311141", "411131", "211412", "211214", "211232", "2331112",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum CodeSet {
    A,
    B,
    C,
}

/// ESC/POS CODE128 data: starts with `{A`, `{B` or `{C`; `{A`/`{B`/`{C` switch sets, `{S`
/// shifts one character, `{1`–`{4` are FNC1–4, `{{` is a literal `{`. In set C every byte
/// (0–99) is a pair of digits.
fn code128(data: &[u8]) -> Option<Encoded> {
    let (mut set, start) = match data.get(..2)? {
        b"{A" => (CodeSet::A, 103),
        b"{B" => (CodeSet::B, 104),
        b"{C" => (CodeSet::C, 105),
        _ => return None,
    };
    let mut values: Vec<u8> = vec![start];
    let mut hri = String::new();
    let mut index = 2;
    let mut shift = false;
    while index < data.len() {
        let byte = data[index];
        index += 1;
        if byte == b'{' {
            let code = *data.get(index)?;
            index += 1;
            let value = match (code, set) {
                (b'{', _) => {
                    push_char(&mut values, &mut hri, b'{', set, shift)?;
                    shift = false;
                    continue;
                }
                (b'A', CodeSet::A) | (b'B', CodeSet::B) | (b'C', CodeSet::C) => continue,
                (b'A', _) => {
                    set = CodeSet::A;
                    101
                }
                (b'B', _) => {
                    set = CodeSet::B;
                    100
                }
                (b'C', _) => {
                    set = CodeSet::C;
                    99
                }
                (b'S', CodeSet::A | CodeSet::B) => {
                    shift = true;
                    98
                }
                (b'1', _) => 102,
                (b'2', CodeSet::A | CodeSet::B) => 97,
                (b'3', CodeSet::A | CodeSet::B) => 96,
                (b'4', CodeSet::A) => 101,
                (b'4', CodeSet::B) => 100,
                _ => return None,
            };
            values.push(value);
            continue;
        }
        if set == CodeSet::C {
            if byte > 99 {
                return None;
            }
            values.push(byte);
            hri.push_str(&format!("{byte:02}"));
        } else {
            push_char(&mut values, &mut hri, byte, set, shift)?;
            shift = false;
        }
    }
    if values.len() < 2 {
        return None;
    }
    let checksum = values
        .iter()
        .enumerate()
        .map(|(position, &value)| position.max(1) as u32 * u32::from(value))
        .sum::<u32>()
        % 103;
    values.push(checksum as u8);
    values.push(106);
    let widths = values
        .iter()
        .flat_map(|&value| {
            CODE128[usize::from(value)]
                .bytes()
                .map(|width| width - b'0')
        })
        .collect();
    Some(Encoded {
        widths,
        kind: Widths::Modules,
        hri,
    })
}

/// A character in set A (0x00–0x5F) or B (0x20–0x7F); `shift` reads it in the other set.
fn push_char(
    values: &mut Vec<u8>,
    hri: &mut String,
    byte: u8,
    set: CodeSet,
    shift: bool,
) -> Option<()> {
    let use_a = (set == CodeSet::A) != shift;
    let value = if use_a {
        match byte {
            0x00..=0x1f => byte + 64,
            0x20..=0x5f => byte - 0x20,
            _ => return None,
        }
    } else {
        match byte {
            0x20..=0x7f => byte - 0x20,
            _ => return None,
        }
    };
    values.push(value);
    if byte >= 0x20 {
        hri.push(char::from(byte));
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bars as `1` and spaces as `0`, `narrow`/`wide` modules for NarrowWide codes: the
    /// same notation python-barcode prints, which generated the expected strings below.
    fn modules(encoded: &Encoded, narrow: usize, wide: usize) -> String {
        encoded
            .widths
            .iter()
            .enumerate()
            .map(|(index, &width)| {
                let count = match encoded.kind {
                    Widths::Modules => usize::from(width),
                    Widths::NarrowWide if width == 2 => wide,
                    Widths::NarrowWide => narrow,
                };
                (if index % 2 == 0 { "1" } else { "0" }).repeat(count)
            })
            .collect()
    }

    fn encoded(symbology: Symbology, data: &[u8]) -> Encoded {
        encode(symbology, data).expect("valid data")
    }

    #[test]
    fn ean13_adds_its_check_digit() {
        let barcode = encoded(Symbology::Ean13, b"590123412345");
        assert_eq!(barcode.hri, "5901234123457");
        assert_eq!(
            modules(&barcode, 1, 1),
            "10100010110100111011001100100110111101001110101010110011011011001000010101110010011101000100101"
        );
        assert_eq!(encoded(Symbology::Ean13, b"5901234123457"), barcode);
    }

    #[test]
    fn ean8_and_upc_a() {
        let ean8 = encoded(Symbology::Ean8, b"9638507");
        assert_eq!(ean8.hri, "96385074");
        assert_eq!(
            modules(&ean8, 1, 1),
            "1010001011010111101111010110111010101001110111001010001001011100101"
        );
        let upc = encoded(Symbology::UpcA, b"03600029145");
        assert_eq!(upc.hri, "036000291452");
        assert_eq!(
            modules(&upc, 1, 1),
            "10100011010111101010111100011010001101000110101010110110011101001100110101110010011101101100101"
        );
    }

    #[test]
    fn code39_adds_start_and_stop() {
        let barcode = encoded(Symbology::Code39, b"AB-12");
        assert_eq!(barcode.hri, "*AB-12*");
        assert_eq!(
            modules(&barcode, 1, 3),
            "100010111011101011101010001011101011101000101110100010101110111011101000101011101011100010101110100010111011101"
        );
        assert_eq!(encoded(Symbology::Code39, b"*AB-12*"), barcode);
    }

    #[test]
    fn code128_code_set_b_with_checksum() {
        let barcode = encoded(Symbology::Code128, b"{BAb12");
        assert_eq!(barcode.hri, "Ab12");
        assert_eq!(
            modules(&barcode, 1, 1),
            "1101001000010100011000100100001101001110011011001110010101111001001100011101011"
        );
    }

    #[test]
    fn code128_code_set_c_takes_digit_pairs() {
        let barcode = encoded(Symbology::Code128, b"{C\x0c\x22\x38");
        assert_eq!(barcode.hri, "123456");
        // Start C (105) + 12, 34, 56: checksum (105 + 12 + 68 + 168) % 103 = 44.
        let start_c = "211232".bytes().map(|w| w - b'0');
        assert!(barcode.widths.iter().copied().take(6).eq(start_c));
        let checksum = &barcode.widths[barcode.widths.len() - 13];
        assert_eq!(*checksum, CODE128[44].as_bytes()[0] - b'0');
    }

    #[test]
    fn itf_and_codabar() {
        let itf = encoded(Symbology::Itf, b"12345678");
        assert_eq!(
            modules(&itf, 2, 5),
            "1100110011111001100000110011001111100000111110011111001100000110011000001111100110000011111000001100110011000001100110011111000001111100111110011"
        );
        let codabar = encoded(Symbology::Codabar, b"A40156B");
        assert_eq!(
            modules(&codabar, 2, 5),
            "11001111100000110000011001100111110011000001100110011001100000111110011001100111110000011001111100110011000001100110000011001100111110011000001100000110011111"
        );
    }

    #[test]
    fn rejects_invalid_data() {
        assert_eq!(encode(Symbology::Ean13, b"12345"), None);
        assert_eq!(encode(Symbology::Ean13, b"59012341234X"), None);
        assert_eq!(encode(Symbology::Itf, b"123"), None, "ITF needs pairs");
        assert_eq!(
            encode(Symbology::Code39, b"ab"),
            None,
            "lowercase is not CODE39"
        );
        assert_eq!(encode(Symbology::Codabar, b"1234"), None, "no start/stop");
        assert_eq!(encode(Symbology::Code128, b"AB"), None, "no code set");
        assert_eq!(
            encode(Symbology::Code128, b"{C\x64"),
            None,
            "set C byte over 99"
        );
        assert_eq!(encode(Symbology::Unsupported, b"123"), None);
    }

    #[test]
    fn code128_table_is_well_formed() {
        for (value, pattern) in CODE128.iter().enumerate() {
            let widths: Vec<u32> = pattern.bytes().map(|w| u32::from(w - b'0')).collect();
            let total: u32 = widths.iter().sum();
            assert_eq!(total, if value == 106 { 13 } else { 11 }, "value {value}");
            let bars: u32 = widths.iter().step_by(2).sum();
            assert_eq!(bars % 2, 0, "value {value}: bar modules must be even");
        }
    }

    #[test]
    fn dots_follow_epson_wide_ratio() {
        let barcode = encoded(Symbology::Code39, b"1");
        // Skip the "*" start character and the gap after it.
        let widths: Vec<u16> = barcode.dots(3).skip(10).take(9).collect();
        // "1" is wide, narrow, narrow, wide, narrow, narrow, narrow, narrow, wide.
        assert_eq!(widths, vec![8, 3, 3, 8, 3, 3, 3, 3, 8]);
    }
}
