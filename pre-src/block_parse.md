BlockParse
==========

The call `BlockParse::new(text)` returns an iterator over the block names in
`text`: for each occurence of a section name `⟨ … ⟩` in `text` the iterator
returns a `(start, end)` pair such that `text[start..end]` is the section name
found.

However, if '⟨' occurs inside a comment or quote, it doesn't start a section
name.

Here's the outline of the implementation:

```rust
#![allow(unused)]
use regex::Regex;

pub(crate) struct BlockParse<'a> {
    text: &'a str,
    scan_from: usize,
}

impl<'a> BlockParse<'a> {
    pub(crate) fn new(text: &'a str) -> BlockParse<'a> {
        BlockParse{text, scan_from: 0}
    }
    ⟨Private methods⟩
}

impl<'a> Iterator for BlockParse<'a> {
    type Item = ParseResult;
    ⟨The `next` method⟩
}

type ParseResult = Result<StrBounds, Unterminated>;
type StrBounds = (usize, usize);
#[derive(Debug, Fail, PartialEq, Eq)]
#[fail(display = "unterminated {} starting {}, ending {}", kind, start, end)]
pub(crate) struct Unterminated {
    pub(crate) kind: &'static str,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

⟨Other definitions⟩
```

## Tests

We want to find the start of the next block name, which will begin with '⟨'; but
we need to take into account comments and character or string literals, since a
'⟨' in the middle of one of those does not start a section name. Our strategy
will be to find the start of the next item — comment, character/string literal,
or block name — and then use specialized routines to find the end of the item.
If the item is a block name, we return its bounds; otherwise we skip to the next
item.

Here are some tests that we can skip over each of those things. (Details later)

```rust
⟨Tests⟩≡
    skip_through!(empty_string, "");
    skip_through!(line_comment, "//⟨\n");
    skip_through!(nested_block_comment, "/* ⟨ /* ⟨y⟩ */ ⟩*/");
    skip_through!(char_literal, "'⟨'");
    skip_through!(byte_escaped_char, r"'\n'");
    skip_through!(illegal_byte_escaped_char, r"'\b'");
    skip_through!(hex_escaped_char, r"'\x7F'");
    skip_through!(illegal_hex_escaped_char, r"'\x77GGGG'");
    skip_through!(unicode_escaped_char, r"'\u{77FF}'");
    skip_through!(illegal_unicode_escaped_char, r"'\u{77⟨F⟩F}'");
    skip_through!(string_with_embedded_block_name, r#""⟨y⟩""#);
    skip_through!(string_with_escaped_double_quote, r#""\"⟨y⟩""#);
    skip_through!(string_with_escaped_escape_is_compared_to_the_empty_string, r#""\\"== """#);
    skip_through!(all_char_examples_in_one_string, r#""⟨\n\b\x7F\x77GGG\u{77FF}\u{77⟨F⟩F}""#);
    skip_through!(string_with_internal_newline, "\"\n\"");
    skip_through!(raw_string, r##"r#"""#"##);
    skip_through!(char_thats_also_identifier, "'r'");
    skip_through!(illegal_char_thats_also_identifier, "'arrrr'");
```


And here's a test that we can handle raw quotes with hundreds of hash marks in
the opening and closing quotes.

```rust
⟨Tests⟩≡
    #[test]
    fn test_very_fat_raw_quotes() {
        let hash = "#".repeat(999);
        // Set `raw` to a raw-quoted string with 1000 #'s, containing
        // a raw-quoted string with 999 #'s
            let raw = format!(r##"r{}#"r"{}⟨y⟩"{}"#{}"##, hash, hash, hash, hash);
        expect_ok(&raw, here!());
    }
```

But if a comment or character or string literal isn't properly terminated, we
want to return an error.

```rust
⟨Tests⟩≡
    complain_about!(unterminated_block_name, "⟨A block name");
    complain_about!(unterminated_char_literal, "'⟨");
    complain_about!(unterminated_char_literal_with_newline, "'^\nabcdef\n");
    complain_about!(confusing_unterminated_char, "'^⟨Not a section name⟩\n");
    complain_about!(unterminated_line_comment, "//⟨");
    complain_about!(unterminated_nested_block_comment, "/* ⟨ /* ⟨y⟩ */ ⟩");
    complain_about!(unterminated_byte_escaped_char, r"'\n");
    complain_about!(unterminated_illegal_byte_escaped_char, r"'\b");
    complain_about!(unterminated_hex_escaped_char, r"'\x7F");
    complain_about!(unterminated_illegal_hex_escaped_char, r"'\x77GGGG");
    complain_about!(unterminated_unicode_escaped_char, r"'\u{77FF}");
    complain_about!(unterminated_illegal_unicode_escaped_char, r"'\u{77⟨F⟩F}");
    complain_about!(unterminated_string_with_embedded_block_name, r#""⟨y⟩"#);
    complain_about!(unterminated_string_with_escaped_double_quote, r#""\"⟨y⟩"#);
    complain_about!(unterminated_string_with_internal_newline, "\"\n");
    complain_about!(unterminated_raw_string, r####"r###"""##"####);
```

Here are the promised test details, including definitions for the `skip_through`
and `complain_about` macros.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    const BLOCK_X: &str = "⟨x⟩";

    macro_rules! here {
         () => { &format!("(called at line {})", line!()) }
    }
    macro_rules! skip_through {
        ($name:ident, $prefix:expr) => {
            #[test] fn $name() { expect_ok($prefix, here!()) }
        }
    }

    fn expect_ok(prefix: &str, msg: &str) {
        let start = prefix.len();
        let text = String::from(prefix) + BLOCK_X;
        let names: Vec<ParseResult> = BlockParse::new(&text).collect();
        assert_eq!(names, [Ok((start, text.len()))], "{}", msg);
    }

    macro_rules! complain_about {
        ($name:ident, $prefix:expr) => {
            #[test] fn $name() { expect_unterminated($prefix, here!()) }
        }
    }
    fn expect_unterminated(text: &str, msg: &str) {
        let last: ParseResult = BlockParse::new(&text).last().unwrap();
        assert!(last.is_err(), "Not Err {}", msg);
        if let Err(ref unterm) = last {
            assert!(unterm.kind.len() > 0, "Empty kind {}", msg);
            assert_eq!((unterm.start, unterm.end), (0, text.len()), "{}", msg);
        }
    }

    ⟨Tests⟩
}
```

## Implementation

The `BlockParse` iterator's `next` method looks for the start of an item: block
name, comment, or string or character literal, returning `None` if it can't find
one. If it finds a valid block name, it will return an `Ok(StrBounds(_))` value
with the starting and ending indices of the item in `text`.  If it finds an
invalid item, it returns and error, again with the starting and ending indices
of the invalid item, as well as a string describing the kind of the item.

```rust
⟨The `next` method⟩≡
    fn next(&mut self) -> Option<ParseResult> {
        lazy_static! {
            static ref START: Regex = Regex::new(r##"(?x)// | ⟨ | /\* | r\#*" | " | ' "##).unwrap();
        }
        loop {
            let (found_start, start_len) = {
                let found = START.find(self.unscanned())?;
                (found.start(), found.end() - found.start())
            };

            let item_start = self.scan_from + found_start;
            self.scan_from = item_start;

            if let Some((item_end, kind)) = self.scan_item(start_len) {
                self.scan_from = item_end;
                return if kind == VALID_BLOCK_NAME {
                    Some(Ok((item_start, item_end)))
                } else {
                    Some(Err(Unterminated{kind, start: item_start, end: item_end}))
                }
            }
        }
    }
```

The `scan_item` method's method is not a `Result` but a simple `Option`: `None`
when the item is succesfully parsed and is not a block name (and can therefore
be ignored); a tuple with containing the ending index of the item and its
description for a valid block name or an invalid item of any kind.

```rust
⟨Other definitions⟩≡
    type TokenInfo = Option<(usize, &'static str)>;
```

When we successfully parse a block name, `scan_item` returns the end of the
block name, and a string. The `next` method doesn't pass the string on, so it
might as well be the empty string.

```rust
⟨Other definitions⟩≡
    const VALID_BLOCK_NAME: &str = "";
```

The `scan_item` method expects `scan_from` to point to the first byte of the
item. It uses that byte to determine which pattern to use to find the end of the
item. (Unless the byte is `b'/'`, in which case `scan_from` looks at the next
byte, to distinguish block comments from line comments.)

```rust
⟨Private methods⟩≡
    fn byte_at(&self, j: usize) -> u8 { self.text.as_bytes()[j] }
    fn scan_item(&mut self, start_len: usize) -> TokenInfo {
        const BLOCK_NAME_END: &str = "⟩";
        const LINE_COMMENT_END: &str = "\n";
        const FIRST_BYTE_OF_OPEN_BRACKET: u8 = 226; // b"⟨" == [226, 159, 168]
        ⟨Define the `STR_QUOTE` and `CHAR_OR_LIFETIME` patterns⟩
        let mut key_byte = self.byte_at(self.scan_from);
        if key_byte == b'/' {
            key_byte = self.byte_at(self.scan_from+1);
        }
        self.scan_from += start_len;
        Some(match key_byte {
            FIRST_BYTE_OF_OPEN_BRACKET => {
                if let Some(info) = self.scan_through("block name", BLOCK_NAME_END) { info }
                else { (self.scan_from, VALID_BLOCK_NAME) }
            }
            b'\'' => self.scan_through("character literal", &*CHAR_OR_LIFETIME)?,
            b'"' => self.scan_through("double quote string", &*STR_QUOTE)?,
            b'/' => self.scan_through("line comment", LINE_COMMENT_END)?,
            b'*' => self.scan_to_end_of_block_comment()?,
            b'r' => self.scan_to_end_of_raw_quote(start_len - 1)?,
            _ => unreachable!(),
        })
    }
```

The generic method `scan_through` searches for a pattern, updating `scan_from`
to the end of the pattern. We use a trait implemented both for `str` and `Regex`
patterns, with a function `find_end` that returns the end of the pattern in a
string.

```rust
⟨Private methods⟩≡
    fn unscanned(&self) -> &str { &self.text[self.scan_from..] }
    fn scan_through<T: EndFinder + ?Sized>(&mut self, kind: &'static str, pattern: &T) -> TokenInfo
    {
        match pattern.find_end(self.unscanned()) {
            Some(len) => { self.scan_from += len; None }
            None => Some((self.text.len(), kind))
        }
    }
```

It might have been better to have provided different methods for `str` and
`Regex` patterns, since we need to use different calling patterns anyway.  A
drawback of `lazy_static` is that the references it generates to a value
ostensibly of type `T` are not of type `&T`, but of a hidden type that
dereferences to `T`.  So to use the generic `scan_through` with a `lazy_static`
pattern `P`, we can't say `scan_through(kind, &P)` but need to say
`scan_through(kind, &*P)` — to dereference `P` to type `T` and then take a
reference to that, which will be the desired `T`.

```rust
⟨Other definitions⟩≡
    trait EndFinder {
        fn find_end(&self, haystack: &str) -> Option<usize>;
    }
    // Don't nag me to use `haystack: &Self` – it would break the pattern
    #[cfg_attr(feature = "cargo-clippy", allow(use_self))]
    impl EndFinder for str {
        fn find_end(&self, haystack: &str) -> Option<usize> {
            Some(haystack.find(&self)? + self.len())
        }
    }
    impl EndFinder for Regex {
        fn find_end(&self, haystack: &str) -> Option<usize> {
            Some(self.find(haystack)?.end())
        }
    }
```



Since block comments can be nested, we need to keep track of the nesting level,
incrementing it every time we see `/*` and decrementing every time we see `*/`.

```rust
⟨Private methods⟩≡
    fn scan_to_end_of_block_comment(&mut self) -> TokenInfo {
        lazy_static! {
            static ref BLOCK_COMMENT_SEGMENT: Regex = Regex::new(r"\*/|/\*").unwrap();
        }
        let mut level = 0;
        loop {
            let info = self.scan_through("block comment", &*BLOCK_COMMENT_SEGMENT);
            if info.is_some() {
                return info
            }
            if self.byte_at(self.scan_from - 1) == b'*' { level += 1 }
            else if level == 0 { break }
            else { level -= 1 }
        }
        None
    }
```

If a raw string ends in '"' plus `end_len - 1` hash marks, and we had a str
`END` that began with `"` and continued with an infinite number of `#`
characters, we could scan through the string by calling
`self.scan_through(&END[..end_len])?` — and that is indeed what we do if
`end_len` is less than or equal to the length of our actual (and finite) str
`END`. To find the end of raw strings heftier endings, we construct a `String`
with the requisite number of ending `#` characters. Since `END` has 62 `#`
characters, we expect to almost never see that case, so we don't try to keep the
longer string around with `lazy_static` and mutexs, but construct it on the fly
every time.

```rust
⟨Private methods⟩≡
    fn scan_to_end_of_raw_quote(&mut self, end_len: usize) -> TokenInfo {
        const END: &str = "\"##############################################################";
        if end_len <= END.len() {
            self.scan_through("raw string", &END[..end_len])
        } else {
            let closing = String::from("\"") + &String::from("#").repeat(end_len-1);
            self.scan_through("raw string", &closing[..])
        }
    }
```

A single quote can start either a character literal or a lifetime identifier.
We parse non-ASCII identifiers since they're present in nightly.  (See the
[unstable book][non-ascii]).

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
            &format!(r"(?x) {}' | {} ", CHAR_SEQUENCE, LIFETIME_SEQUENCE)
        ).unwrap();
        static ref STR_QUOTE: Regex = Regex::new(
            &format!("{}\"", STR_SEQUENCE)
        ).unwrap();
    }
```
