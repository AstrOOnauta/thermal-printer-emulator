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
