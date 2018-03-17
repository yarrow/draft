// #![cfg_attr(feature = "cargo-clippy", deny(clippy, clippy_pedantic))]
#![allow(unused)]
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

pub mod tangle;
mod block_parse;
mod code_extractor;
mod line_counter;

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
