//! ESC/POS decoder, written from scratch from Epson's public command reference
//! (`.ai/rules.md` § ESC/POS decoder).

pub mod bitmap;
pub mod codepage;
mod codepage_tables;
pub mod parser;
pub mod printer;
