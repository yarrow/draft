// #![cfg_attr(feature = "cargo-clippy", deny(clippy, clippy_pedantic))]
#![cfg_attr(feature = "cargo-clippy", deny(clippy))]
#![cfg_attr(feature = "cargo-clippy", warn(clippy_pedantic))]
#![cfg_attr(feature = "cargo-clippy",
    allow(
        redundant_field_names, // Bug in clippy v0.0.187?
        missing_docs_in_private_items, // For now, the Markdown source contains the private docs
        print_stdout,
        // for readability
        non_ascii_literal,
        option_unwrap_used,
        result_unwrap_used,
        shadow_same,
        string_add,
    ))]
//! See README

extern crate memchr;
extern crate pulldown_cmark;
extern crate regex;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate failure;

mod block_parse;
pub mod tangle;
mod code_extractor;
use code_extractor::CodeExtractor;

#[derive(Debug, PartialEq, Eq)]
enum Ilk {
    SectionName,
    Unterminated(&'static str),
}

#[derive(Debug, PartialEq, Eq)]
struct Span {
    lo: usize,
    hi: usize,
    ilk: Ilk,
}

pub fn show_raw(text: &str) {
    // DELETEME: Just to silence dead code warnings
    let blocks = CodeExtractor::new(text);
    for (code, info) in blocks {
        println!("Code block ({})", info);
        println!("{}", code);
    }
}
