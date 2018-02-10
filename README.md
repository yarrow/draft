Draft: Expository Programming in Rust
=====================================

With `draft` you can write something like this:

```rust
for x in something().iter() {
    ⟨break if x is invalid⟩;
    ⟨Process x⟩;
}
```

followed by a paragraph or two explaining that we process `x` by analyzing it in
to two pieces, `y` and `z`, where `y` is added to a database and `z` is logged:

```rust
⟨Process x⟩≡
    let (y, z) = analyze(x);
    database.add(y);
    log(z);
```

The validation and error handling implied by `⟨break if x is invalid⟩` can be
defined later in the program.

In the 1980's Donald Knuth created something he called "literate programming",
and used it to write the widely-used TeX system for mathematical typesetting, as
a book-length essay that was also a program.  TeX is still a mainstay of
scientific and technical publishing, while literate programming has not been so
successful.

One reason for this is that not every piece of code is best appreciated as an
essay. If we're more interested in using a Rust crate than in understanding its
internals, then `rustdoc` is our friend: we get well organized, good looking
documentation for a modest investment in doc comments.

But for guides and tutorials, perhaps expository programming has a place. At any
rate, the `draft` system is an experiment in just that: a tool for expository
programming that is itself written as an expository program.  (I prefer
"expository programming" to "literate programming" because I don't believe that
the average programmer is illiterate! Nor do I believe that every program or
library should be written as an essay.)

Knuth's `WEB` system had two tools: `tangle` and `weave`.  For a given source
file (say `metafont.web`), the command `tangle metafont.web` produced
`metafont.pas` and `weave metafont.web` produced `metafont.tex`. These days
literate programming tools usually work on Markdown files and have no equivalent
to `weave`. (Or rather, the various markdown-to-HTML tools are their `weave`
equivalent. Some, like `pandoc`, will even produce TeX.)

[More to come...]

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
