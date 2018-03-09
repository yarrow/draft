BlockParse
==========

Blah `BlockParse::new(text)` returns an iterator over the block names in `text`: for
each occurence of a section name `⟨ … ⟩` in `text` the iterator returns a
`(start, end)` pair such that `text[start..end]` is the section name found.

However, if '⟨' occurs inside a comment or quote, it doesn't start a section
name.

Here's the outline of the implementation:

```rust
#![allow(unused)]
struct BlockParse<'a> {
    text: &'a str,
    scan_from: usize,
}

type TextRange = ::std::ops::Range<usize>;
impl<'a> BlockParse<'a> {
    pub(crate) fn new(text: &'a str) -> BlockParse<'a> {
        BlockParse{text, scan_from: 0}
    }
    ⟨Utility methods⟩
}

use regex::Regex;

type StrBounds = (usize, usize);
impl<'a> Iterator for BlockParse<'a> {
    type Item = StrBounds;
    fn next(&mut self) -> Option<StrBounds> {
        ⟨Find the start and end of the next block name, if any⟩
    }
}
```

We want to find the start of the next block name, which will begin with '⟨'; but
we need to take into account comments and character or string literals, since a
'⟨' in the middle of one of those does not start a section name. Our strategy
will be to find the start of the next item — comment, character/string literal,
or block name — and then use specialized routines to find the end of the item.
If the item is a block name, we return its bounds; otherwise we continue.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    const NONSTARTERS: &'static [&str] = &[
        "",
        "//⟦\n", // line comment
        "/* ⟦ /* ⟦y⟧ */ ⟧*/", // block comment  
        "'⟦'",          // char
        r"'\n'",        // byte escaped char
        r"'\b'",        // illegal byte escaped char
        r"'\x7F'",      // hex escaped char
        r"'\x77GGGG'",  // illegal hex escaped char
        r"'\u{77FF}'",  // Unicode escaped char
        r"'\u{77⟦F⟧F}'",// illegal Unicode escaped char
        r#""⟦y⟧""#,     // string, not block name
        r#""\"⟦y⟧""#,   // string with escaped double quote
        r#""\\" == """#,// string with escaped escape, compared to the empty string
        r#""⟦\n\b\x7F\x77GGG\u{77FF}\u{77⟦F⟧F}""#,  // the char examples, as one big string
        r##"r#"""#"##,  // a raw string with just one character, a double quote
    ]; 
    const BLOCK_X: &str = "⟦x⟧";
    #[test]
    fn block_parse_nonstarters() {
        let mut nonstarters = Vec::new();
        for non in NONSTARTERS.iter() {
            nonstarters.push(String::from(*non));
            if non.is_empty() { continue }
            match non.as_bytes()[0] {
                b'\'' | b'"' | b'r' => { nonstarters.push(String::from("b") + non) }
                _ => ()
            }
        }
        for non in nonstarters.iter() {
            let start = non.len();
            let end = start + BLOCK_X.len();
            let text = non.clone() + BLOCK_X;
            let names: Vec<StrBounds> = BlockParse::new(&text).collect();
            assert_eq!(names, [(start,end)],
                "Checking {} after {:?} ({})", BLOCK_X, non, non
            );
        }
    }
}
// Check strings with ending backslashes.
//        r"'\'",         // char literal doesn't end
//        "\"\n\"",       // string with internal newline
//        "'^\n",         // unterminated char
//        "'^⟦We don't find this section name⟧\n", // confusing unterminated char
```

We'll often want to scan for a pattern and return the pattern's start and end as
indexes into `self.text`. We simultaneously update `self.scan_from` to point to
the end of the pattern.

```rust
⟨Utility methods⟩≡
    fn fynd(&mut self, pattern: &Regex) -> Option<TextRange> {
        let found = pattern.find(&self.text[self.scan_from..])?;
        let found = TextRange{
            start: self.scan_from+found.start(),
            end: self.scan_from+found.end()
        };
        self.scan_from = found.end;
        Some(found)
    }
```

Even more often we only want the side effect of updating `self.scan_from`.

```rust
⟨Utility methods⟩≡
    fn consume(&mut self, pattern: &Regex) -> Option<()> {
        self.fynd(pattern)?;
        Some(())
    }
```

With those tools in hand we can find block names in code blocks. We need to skip
comments, string literals, and character literals. We'll accept even illegal
string and character literals — if we miss a block name because we're too
generous in accepting such a literal, the compiler will require a rewrite
anyway.

[Talk about unclosed strings and block comments]

```rust
⟨Find the start and end of the next block name, if any⟩≡
    #![cfg_attr(feature = "cargo-clippy", allow(trivial_regex))]
    //const RAW_STR: &str = "\"###################################################################";
    const RAW_STR: &str = "\"##";
    lazy_static! {
        static ref START: Regex = Regex::new(r##"(?x) // | ⟦ | /\* | r\#*" | " | ' "##).unwrap();
        static ref BLOCK_NAME: Regex = Regex::new(r"⟧").unwrap();
        static ref LINE_COMMENT: Regex = Regex::new(r"\n").unwrap();
        static ref BLOCK_COMMENT_SEGMENT: Regex = Regex::new(r"\*/|/\*").unwrap();
    }
    ⟨Define the `STR_QUOTE` and `CHAR_OR_LIFETIME` patterns⟩

    loop {
        let beginning = self.fynd(&START)?;
        let item_start = beginning.start;
        let mut key_byte = self.byte_at(item_start);
        if key_byte == b'/' {
            key_byte = self.byte_at(item_start+1);
        }
        match key_byte {
            226 => { // first byte of ⟦
                self.consume(&BLOCK_NAME)?;
                return Some((item_start, self.scan_from));
            }
            b'/' => self.consume(&LINE_COMMENT)?,
            b'\'' => self.consume(&CHAR_OR_LIFETIME)?,
            b'"' => self.consume(&STR_QUOTE)?,
            b'*' => {
                let mut level = 0;
                loop {
                    let boundary = self.fynd(&BLOCK_COMMENT_SEGMENT)?;
                    if self.byte_at(boundary.start) == b'/' { level += 1 }
                    else if level == 0 { break }
                    else { level -= 1 }
                }
            }
            b'r' => {
                let stop_len = beginning.len() - 1;
                let stopped_at = self.text[beginning.end..].find(&RAW_STR[..stop_len])?;
                self.scan_from = beginning.end + stopped_at + stop_len;
            }
            _ => unreachable!(),
        }
    }
```

```rust
⟨Utility methods⟩≡
    fn byte_at(&self, j: usize) -> u8 { self.text.as_bytes()[j] }
```

A single quote can start either a character literal or a lifetime identifier.
We parse non-ASCII identifiers since they're present in nightly.  See the
[unstable book][non-ascii].

[non-ascii]: https://doc.rust-lang.org/nightly/unstable-book/language-features/non-ascii-idents.html

```rust
⟨Define the `STR_QUOTE` and `CHAR_OR_LIFETIME` patterns⟩≡
    const LIFETIME_SEQUENCE: &str = r"^(?:\p{XID_Start}\p{XID_Continue}*\b | _\p{XID_Continue}+\b)";
```

For both string and character literals we'll want to recognize a sequence of
zero or more possibly escaped characters, starting at the initial `"` or `'` and
ending at the final `"` or `'`. We don't care if the escape is legal, just that
we parse `\"`, `\'` and `\\` correctly. We'll let the compiler do the
complaining about, say, an attempted Unicode escape like `"\u{eh?"`.

Our sequence must be lazy: we want to stop at the first unescaped `"` we see for
string literals or `'` for character literals.  And it must be anchored in order
to parse escaped characters as escaped: without the anchoring `^`, our
`STR_QUOTE` pattern would match `\"` at the `"` rather than reporting an
unterminated string. 

String literals can include literal newline characters (so we specify the `s`
flag), while character literals cannot (so we specify the `-s` flag).

```rust
⟨Define the `STR_QUOTE` and `CHAR_OR_LIFETIME` patterns⟩≡
    const CHAR_SEQUENCE: &str = r#"^(?x-s: \\. | [^\\\n] )*?"#;
    const STR_SEQUENCE: &str =  r#"^(?xs:  \\. | [^\\])*?"#;
    lazy_static! {
        static ref CHAR_OR_LIFETIME: Regex = Regex::new(
            &format!(r"(?x) {} | {}' ", LIFETIME_SEQUENCE, CHAR_SEQUENCE)
        ).unwrap();
        static ref STR_QUOTE: Regex = Regex::new(
            &format!("{}\"", STR_SEQUENCE)
        ).unwrap();
    }
```
