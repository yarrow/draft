// #![cfg_attr(feature = "cargo-clippy", deny(clippy, clippy_pedantic))]
#![cfg_attr(feature = "cargo-clippy", deny(clippy))]
#![cfg_attr(feature = "cargo-clippy",
    allow(missing_docs_in_private_items, // This is the Markdown source
    ))]
//! See README
// #![allow(dead_code)] // FIXME remove when we're done
// #![allow(unused_variables)] // FIXME remove when we're done
mod code_filter;
pub use code_filter::CodeFilter; // FIXME this will eventually be private
