// #![cfg_attr(feature = "cargo-clippy", deny(clippy, clippy_pedantic))]
#![cfg_attr(feature = "cargo-clippy", deny(clippy))]
#![cfg_attr(feature = "cargo-clippy", warn(clippy_pedantic))]
#![cfg_attr(feature = "cargo-clippy",
    allow(
        missing_docs_in_private_items, // For now, the Markdown source contains the private docs
        print_stdout,
    ))]
//! See README
// #![allow(dead_code)] // FIXME remove when we're done
// #![allow(unused_variables)] // FIXME remove when we're done

mod line_counter;

mod code_extractor;
use code_extractor::{CodeExtractor, RawCode};

pub fn show_raw(text: &str) { // DELETEME: Just to silence dead code warnings
    let blocks = CodeExtractor::new(text);
    for RawCode{code, line, info} in blocks {
        println!("Code block ({}) at line {}", info, line);
        println!("{}", code);
    }
}
