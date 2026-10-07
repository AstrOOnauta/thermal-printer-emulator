//! A sample receipt, sent to our own listener through its socket like a POS would: it
//! proves the port is open and shows every kind of output the decoder draws. Text is in the
//! UI language and in the configured default code page, so it also shows that setting.

use std::io;
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::escpos::model::Paper;
use crate::locale::Strings;
use crate::settings::Settings;

const SEND_TIMEOUT: Duration = Duration::from_secs(5);
const ESC: u8 = 0x1b;
const GS: u8 = 0x1d;
const REPOSITORY: &str = "https://github.com/AstrOOnauta/thermal-printer-emulator";

/// ESC/POS bytes of the test receipt.
pub fn build(settings: &Settings, text: &Strings) -> Vec<u8> {
    let page = settings.code_page();
    let columns = match settings.paper {
        Paper::Mm80 => 48,
        Paper::Mm58 => 32,
    };
    let paper = match settings.paper {
        Paper::Mm80 => "80 mm",
        Paper::Mm58 => "58 mm",
    };
    let line = |out: &mut Vec<u8>, content: &str| {
        out.extend(content.chars().map(|character| page.encode(character)));
        out.push(b'\n');
    };
    let rule = "-".repeat(columns);

    let mut out = vec![ESC, b'@'];
    out.extend([ESC, b'a', 1, ESC, b'!', 0x38]);
    line(&mut out, text.test_title);
    out.extend([ESC, b'!', 0]);
    line(&mut out, "Thermal Printer Emulator");
    line(&mut out, text.test_working);
    out.extend([ESC, b'a', 0]);
    line(&mut out, &rule);
    line(&mut out, &format!("{} {}", text.test_port, settings.port));
    line(&mut out, &format!("{} {paper}", text.test_paper));
    line(
        &mut out,
        &format!("{} {}", text.test_code_page, page.name()),
    );
    line(&mut out, &rule);
    line(&mut out, "Ação Café Ñandú Ü € £ ½ ─│┌┐");
    out.extend([ESC, b'E', 1]);
    line(&mut out, text.test_bold);
    out.extend([ESC, b'E', 0, ESC, b'-', 1]);
    line(&mut out, text.test_underline);
    out.extend([ESC, b'-', 0, GS, b'B', 1]);
    line(&mut out, &format!(" {} ", text.test_reverse));
    out.extend([GS, b'B', 0, ESC, b'M', 1]);
    line(&mut out, text.test_font_b);
    out.extend([ESC, b'M', 0]);
    line(&mut out, &rule);

    // CODE128 with the text below it, then a QR code to the repository.
    out.extend([ESC, b'a', 1, GS, b'H', 2, GS, b'h', 60, GS, b'w', 2]);
    let code = b"{BTPE-TEST";
    out.extend([GS, b'k', 73, code.len() as u8]);
    out.extend_from_slice(code);
    out.extend([ESC, b'd', 1]);
    let qr = REPOSITORY.as_bytes();
    out.extend([GS, b'(', b'k', 3, 0, 49, 67, 4]);
    let store_len = (qr.len() + 3) as u16;
    out.extend([
        GS,
        b'(',
        b'k',
        store_len as u8,
        (store_len >> 8) as u8,
        49,
        80,
        48,
    ]);
    out.extend_from_slice(qr);
    out.extend([GS, b'(', b'k', 3, 0, 49, 81, 48]);
    out.extend([ESC, b'd', 3, GS, b'V', 66, 0]);
    out
}

/// Writes `bytes` to 127.0.0.1:`port` and closes the connection.
pub async fn send(port: u16, bytes: &[u8]) -> io::Result<()> {
    let attempt = async {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await?;
        stream.write_all(bytes).await?;
        stream.shutdown().await
    };
    timeout(SEND_TIMEOUT, attempt)
        .await
        .unwrap_or_else(|_| Err(io::ErrorKind::TimedOut.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::escpos::model::{Block, Output};
    use crate::escpos::Decoder;
    use crate::locale::Locale;

    fn decode(settings: &Settings) -> (Vec<Output>, usize) {
        let mut decoder = Decoder::new(settings.paper, settings.code_page());
        let mut tagged = Vec::new();
        decoder.feed(&build(settings, Locale::PtBr.strings()), &mut tagged);
        let mut outputs: Vec<Output> = tagged.into_iter().map(|(output, _)| output).collect();
        decoder.finish(&mut outputs);
        (outputs, decoder.unknown_commands())
    }

    #[test]
    fn decodes_into_text_codes_and_a_cut() {
        let (outputs, unknown) = decode(&Settings::default());
        assert_eq!(unknown, 0);
        let images = outputs
            .iter()
            .filter(|output| matches!(output, Output::Block(Block::Image(_))))
            .count();
        assert_eq!(images, 2, "barcode and QR");
        assert_eq!(outputs.last(), Some(&Output::Cut { partial: true }));
        let text: String = outputs
            .iter()
            .filter_map(|output| match output {
                Output::Block(Block::Line { segments, .. }) => Some(
                    segments
                        .iter()
                        .map(|segment| segment.text.as_str())
                        .collect::<String>(),
                ),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("CUPOM DE TESTE"));
        assert!(text.contains("CP437"));
        // CP437 has neither "ã" nor "€": the receipt shows why the code page matters.
        assert!(text.contains("Aç?o Café Ñandú Ü ? £ ½ ─│┌┐"), "{text}");
    }

    #[test]
    fn follows_the_configured_code_page_and_paper() {
        let settings = Settings {
            code_page: 19,
            paper: Paper::Mm58,
            ..Settings::default()
        };
        let (outputs, _) = decode(&settings);
        let all_text: String = outputs
            .iter()
            .filter_map(|output| match output {
                Output::Block(Block::Line { segments, .. }) => Some(
                    segments
                        .iter()
                        .map(|segment| segment.text.clone())
                        .collect::<String>(),
                ),
                _ => None,
            })
            .collect();
        assert!(
            all_text.contains("Ação Café Ñandú Ü €"),
            "CP858 has them: {all_text}"
        );
        let rule = outputs.iter().find_map(|output| match output {
            Output::Block(Block::Line { segments, .. }) if segments[0].text.starts_with("---") => {
                Some(segments[0].text.len())
            }
            _ => None,
        });
        assert_eq!(rule, Some(32), "58 mm has 32 columns");
    }
}
