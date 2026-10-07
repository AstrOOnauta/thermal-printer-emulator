//! ESC/POS decoder, written from scratch from Epson's public command reference
//! (`.ai/rules.md` § ESC/POS decoder).

pub mod barcode;
pub mod bitmap;
pub mod codepage;
mod codepage_tables;
pub mod command;
pub mod inspect;
pub mod model;
pub mod parser;
pub mod printer;

use codepage::CodePage;
use model::{Output, Paper};
use parser::Parser;
use printer::Printer;

/// Parser and printer for one connection.
pub struct Decoder {
    parser: Parser,
    printer: Printer,
    scratch: Vec<Output>,
}

impl Decoder {
    pub fn new(paper: Paper, code_page: CodePage) -> Self {
        Self {
            parser: Parser::default(),
            printer: Printer::with_code_page(paper, code_page),
            scratch: Vec::new(),
        }
    }

    /// Decodes `bytes`; each output comes with the stream offset right after the command
    /// that produced it.
    pub fn feed(&mut self, bytes: &[u8], out: &mut Vec<(Output, u64)>) {
        let printer = &mut self.printer;
        let scratch = &mut self.scratch;
        self.parser.feed(bytes, |command, end| {
            printer.apply(command, scratch);
            out.extend(scratch.drain(..).map(|output| (output, end)));
        });
    }

    /// End of the stream: what is left in the line buffer.
    pub fn finish(&mut self, out: &mut Vec<Output>) {
        self.printer.finish(out);
    }

    /// The code page in force after the bytes fed so far.
    pub fn code_page(&self) -> CodePage {
        self.printer.code_page()
    }

    /// Commands the parser did not know.
    pub fn unknown_commands(&self) -> usize {
        self.printer.unknown_commands
    }

    /// Bytes of an unfinished command, dropped when the stream ends.
    pub fn pending_len(&self) -> usize {
        self.parser.pending_len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The outputs, and the stream offset of each cut. Other offsets may differ by chunking:
    /// a line wrapped inside a text run carries the end of that run, and runs follow reads.
    /// Cuts are what receipts are split on.
    fn decode<'a>(chunks: impl Iterator<Item = &'a [u8]>) -> (Vec<Output>, Vec<u64>) {
        let mut decoder = Decoder::new(Paper::Mm80, CodePage::DEFAULT);
        let mut fed = Vec::new();
        for chunk in chunks {
            decoder.feed(chunk, &mut fed);
        }
        let cuts = fed
            .iter()
            .filter(|(output, _)| matches!(output, Output::Cut { .. }))
            .map(|&(_, end)| end)
            .collect();
        let mut outputs: Vec<Output> = fed.into_iter().map(|(output, _)| output).collect();
        decoder.finish(&mut outputs);
        (outputs, cuts)
    }

    /// Streaming, end to end: whatever the TCP reads, the same receipt, cut at the same byte.
    #[test]
    fn random_streams_decode_the_same_in_any_chunks() {
        let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..1_000 {
            let len = (next() % 384) as usize;
            let bytes: Vec<u8> = (0..len)
                .map(|_| match next() % 3 {
                    0 => [0x1b, 0x1d, 0x0a, 0x10][(next() % 4) as usize],
                    _ => next() as u8,
                })
                .collect();
            let whole = decode(std::iter::once(&bytes[..]));
            assert_eq!(decode(bytes.chunks(1)), whole, "{bytes:02x?}");
            let size = 1 + (next() % 16) as usize;
            assert_eq!(decode(bytes.chunks(size)), whole, "{bytes:02x?}");
        }
    }
}
