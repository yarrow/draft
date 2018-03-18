BlockParse
==========

The call `BlockParse::new(text)` returns an iterator over the parts of `text`.
The uninteresting parts (`JustCode`) are simply appended to the output by
Tangle. A `Span` is interesting if it is a section name (which `Tangle` must
expand), or an unterminated comment or quote (which `Tangle` must report to the
user).  Specifically, for each occurence of a section name `⟨ … ⟩` in `text` the
iterator returns a `Span{lo, hi, ilk: SectionName}` value such that
`text[lo..hi]` is the section name found.  While if it finds a comment, quote,
or block name that begins within the block but doesn't end, it returns a
`Span{lo, hi, ilk: Unterminated(s)}` value where `s` describes the unterminated
item.

The iterator will also return the `JustCode` sections that occur between
interesting sections.

(We need to parse comments and quotes because an angle bracket '⟨' that occurs
inside one of those has no special significance, so it doesn't start a section
name. Assuming they are properly terminated they are treated as `JustCode`.)

Here's the implementation outline:

```rust
use regex::Regex;
use Span;
use Ilk;
use Ilk::{JustCode, SectionName, Unterminated};

pub(crate) struct BlockParse<'a> {
    text: &'a str,
    scan_from: usize,
    pending: Option<Span>,
}

impl<'a> BlockParse<'a> {
    pub(crate) fn new(text: &'a str) -> BlockParse<'a> {
        BlockParse{text, scan_from: 0, pending: None}
    }
    ⟨Private methods⟩
}

impl<'a> Iterator for BlockParse<'a> {
    type Item = Span;
    ⟨The `next` method⟩
}

⟨Other definitions⟩
```

## Tests

Here are some tests that the iterator successfully skips quotes and comments. (Details later)

```rust
⟨Tests⟩≡
    skip_through!(empty_string, "");
    skip_through!(line_comment, "//⟨\n");
    skip_through!(nested_block_comment, "/* ⟨ /* ⟨y⟩ */ ⟩*/");
    skip_through!(char_literal, "'⟨'");
    skip_through!(long_illegal_char_literal, "'⟨not a section name⟩'");
    skip_through!(byte_escaped_char, r"'\n'");
    skip_through!(illegal_byte_escaped_char, r"'\b'");
    skip_through!(hex_escaped_char, r"'\x7F'");
    skip_through!(illegal_hex_escaped_char, r"'\x77GGGG'");
    skip_through!(unicode_escaped_char, r"'\u{77FF}'");
    skip_through!(illegal_unicode_escaped_char, r"'\u{77⟨F⟩F}'");
    skip_through!(string_with_embedded_section_name, r#""⟨y⟩""#);
    skip_through!(string_with_escaped_double_quote, r#""\"⟨y⟩""#);
    skip_through!(string_with_escaped_escape_is_compared_to_the_empty_string, r#""\\"== """#);
    skip_through!(all_char_examples_in_one_string, r#""⟨\n\b\x7F\x77GGG\u{77FF}\u{77⟨F⟩F}""#);
    skip_through!(string_with_internal_newline, "\"\n\"");
    skip_through!(raw_string, r##"r#"""#"##);
    skip_through!(char_thats_also_identifier, "'r'");
    skip_through!(illegal_char_thats_also_identifier, "'arrrr'");
```

And here's a test that we can handle raw quotes with hundreds of hash marks in
the opening and closing quotes: specifically, a raw-quoted string with 1000 `#`
characters in the opening and closing quotes, in which is embedded a raw-quoted
string with 999 `#` characters in its quotes.

```rust
⟨Tests⟩≡
    #[test]
    fn test_very_fat_raw_quotes() {
        let hash = "#".repeat(999);
        let raw = format!(r##"r{}#"r"{}⟨y⟩"{}"#{}"##, hash, hash, hash, hash);
        expect_ok(&raw, here!());
    }
```

Here are tests to check that an unterminated comment, quote, or section name
causes the iterator to return a `Span` value whose `ilk` is `Unterminated`.

```rust
⟨Tests⟩≡
    complain_about!(unterminated_section_name, "⟨A block name");
    complain_about!(unterminated_line_comment, "//⟨");
    complain_about!(unterminated_nested_block_comment, "/* ⟨ /* ⟨y⟩ */ ⟩");
    complain_about!(unterminated_string_with_embedded_section_name, r#""⟨y⟩"#);
    complain_about!(unterminated_string_with_escaped_double_quote, r#""\"⟨y⟩"#);
    complain_about!(unterminated_string_with_internal_newline, "\"\n");
    complain_about!(unterminated_raw_string, r####"r###"""##"####);
    complain_about!(unterminated_byte_escaped_char, r"'\n");
    complain_about!(unterminated_illegal_byte_escaped_char, r"'\b");
    complain_about!(unterminated_hex_escaped_char, r"'\x7F");
    complain_about!(unterminated_illegal_hex_escaped_char, r"'\x77GGGG");
    complain_about!(unterminated_unicode_escaped_char, r"'\u{77FF}");
    complain_about!(unterminated_illegal_unicode_escaped_char, r"'\u{77⟨F⟩F}");
    complain_about!(confusing_unterminated_char, "'^⟨section name⟩\n");
    complain_about!(unterminated_char_literal, "'⟨x");
    complain_about!(unterminated_char_literal_with_newline, "'^\nabcdef\n");
    complain_about!(unterminated_backslash_char, "'\\");
```

Here are the promised test details, including definitions for the `skip_through`
and `complain_about` macros.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    const SECTION_NAME: &str = "⟨x⟩";

    macro_rules! here {
         () => { &format!("(called at line {})", line!()) }
    }
    macro_rules! skip_through {
        ($name:ident, $prefix:expr) => {
            #[test] fn $name() { expect_ok($prefix, here!()) }
        }
    }

    fn expect_ok(prefix: &str, msg: &str) {
        let lo = prefix.len();
        let text = String::from(prefix) + SECTION_NAME;
        let names: Vec<Span> = BlockParse::new(&text).filter(
            |s| match s.ilk { JustCode => false, _ => true}
        ).collect();
        assert_eq!(names, [Span{lo, hi:text.len(), ilk: SectionName}], "{}", msg);
    }

    macro_rules! complain_about {
        ($name:ident, $prefix:expr) => {
            #[test] fn $name() { expect_unterminated($prefix, here!()) }
        }
    }

    fn expect_unterminated(text: &str, msg: &str) {
        let Span{lo, hi, ilk} = BlockParse::new(&text).nth(0).unwrap();
        ⟨Set `end` to the expected end of the unterminated `Span`⟩
        assert_eq!((lo, hi), (0, end), "{}", msg);
        match ilk {
            Unterminated(_) => (),
            _ => panic!("Expected Unterminated {}", msg),
        }
    }

    ⟨Tests⟩
}
```

Unterminated comments and string literals generally consume the remainder of
their block; unterminated characters stop after one possibly escaped character.

```rust
⟨Set `end` to the expected end of the unterminated `Span`⟩≡
    let ch: Vec<char> = text.chars().take(3).collect();
    let mut end = text.len();
    if ch.len() > 1 && ch[0] == '\'' {
        end = if ch.len() > 2 && ch[1] == '\\' { 2 + ch[2].len_utf8() }
        else { 1 + ch[1].len_utf8() }
    }
```

And some regression tests:

```rust
⟨Tests⟩≡
    #[test]
    fn test_just_code() {
        let span_vec: Vec<Span> = BlockParse::new("a\n").collect();
        let expected: Vec<Span> = vec![Span{lo: 0, hi: 2, ilk: JustCode}];
        assert_eq!(span_vec, expected);
    }
```
    
## Implementation

The `next` method can return `JustCode`, text that should be output as-is; a
`SectionName`, which Tangle will expand; or an `Unterminated` item, for which
Tangle will emit an error message.  We proceed by skipping forward to a
`SectionName` or `Unterminated` item, saving it in `pending`, and emitting the `JustCode`
that we skipped. Then on the next call to `next`, we see there is a `pending`
item and return it.

```rust
⟨The `next` method⟩≡
    fn next(&mut self) -> Option<Span> {

        if self.pending.is_some() {
            return { self.pending.take() }
        }
        
        if self.scan_from == self.text.len() { return None }

        let scan_start = self.scan_from;
        loop {
            match self.item_start() {
                None => {
                    self.scan_from = self.text.len();
                    return just_code(scan_start, self.text.len());
                }
                Some((found_start, start_len)) => {
                    let lo = self.scan_from + found_start;
                    self.scan_from = lo;
                    if let Some((hi, ilk)) = self.scan_item(start_len) {
                        self.scan_from = hi;
                        self.pending = Some(Span{lo, hi, ilk});
                        if lo == scan_start { return self.pending.take() }
                        else { return just_code(scan_start, lo) }
                    }
                }
            }
        }
    }
```

```rust
⟨Other definitions⟩≡
    fn just_code(lo: usize, hi: usize) -> Option<Span> { Some(Span{lo, hi, ilk: JustCode}) }
```

```rust
⟨Private methods⟩≡
    fn item_start(&self) -> Option<(usize, usize)> {
        lazy_static! {
            static ref START: Regex = Regex::new(r##"(?x)// | ⟨ | /\* | r\#*" | " | ' "##).unwrap();
        }
        let found = START.find(self.unscanned())?;
        Some((found.start(), found.end() - found.start()))
    }
```

The `scan_item` method returns `None` when the item is succesfully parsed and is
not a section name (and can therefore be ignored); but a tuple containing the
ending index of the item and its `Ilk` for a valid section name or an invalid
item of any kind.

```rust
⟨Other definitions⟩≡
    type TokenEnd = Option<(usize, Ilk)>;
```

The `scan_item` method expects `scan_from` to point to the first byte of the
item. It uses that byte to determine which pattern to use to find the end of the
item. (Unless the byte is `b'/'`, in which case `scan_from` looks at the next
byte, to distinguish block comments from line comments.)

If it scans a valid section name, the `Ilk` returned is `SectionName`.  If the
item is unterminated, the `Ilk` returned is `Unterminated`, with a string
describing the kind of the unterminated item.

```rust
⟨Private methods⟩≡
    fn byte_at(&self, j: usize) -> u8 { self.text.as_bytes()[j] }
    fn current_byte(&self) -> u8 { self.byte_at(self.scan_from) }
    fn next_byte(&self) -> u8 { self.byte_at(self.scan_from + 1) }
    fn previous_byte(&self) -> u8 { self.byte_at(self.scan_from - 1) }

    fn scan_item(&mut self, start_len: usize) -> TokenEnd {
        const SECTION_NAME_END: &str = "⟩";
        const LINE_COMMENT_END: &str = "\n";
        const FIRST_BYTE_OF_OPEN_BRACKET: u8 = 226; // b"⟨" == [226, 159, 168]
        let mut key_byte = self.current_byte();
        if key_byte == b'/' {
            key_byte = self.next_byte()
        }
        self.scan_from += start_len;
        Some(match key_byte {
            FIRST_BYTE_OF_OPEN_BRACKET => {
                if let Some(info) = self.scan_through("section name", SECTION_NAME_END) { info }
                else { (self.scan_from, SectionName) }
            }
            b'"' => self.scan_to_end_of_double_quote()?,
            b'\'' => self.scan_through_character_literal()?,
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
    fn end_of<T: EndFinder + ?Sized>(&mut self, pattern: &T) -> Option<usize> {
        pattern.find_end(self.unscanned())
    }
    fn scan_through<T: EndFinder + ?Sized>(&mut self, kind: &'static str, pattern: &T) -> TokenEnd
    {
        match self.end_of(pattern) {
            Some(len) => { self.scan_from += len; None }
            None => Some((self.text.len(), Unterminated(kind)))
        }
    }
```

A drawback of `lazy_static` is that the references it generates to a value
ostensibly of type `T` are not of type `&T`, but of a hidden type that
dereferences to `T`.  So to use the generic `scan_through` or `end_of` with a
`lazy_static` pattern `P`, we can't say `scan_through(kind, &P)` but need to say
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

Since a string literal can contain newlines, we turn on the `s` Regex flag. We
also use the `x` flag so we can space things out for a slightly more readable
regex.

Our pattern must be lazy: we want to stop at the first unescaped `"` we see.
And it must be anchored, in order to parse escaped characters as escaped: without
the anchoring `^`, the `STR_QUOTE` pattern would match `\"` at the `"` rather
than moving past the escaped `"`.

```rust
⟨Private methods⟩≡
    fn scan_to_end_of_double_quote(&mut self) -> TokenEnd {
        lazy_static! {
            static ref STR_QUOTE: Regex = Regex::new(
                 r#"(?xs) ^(?: \\ . | [^\\] )*? " "#
            ).unwrap();
        }
        self.scan_through("double quote string", &*STR_QUOTE)
    }
```

We don't complain about character literals with multiple codepoints, leaving that
to the Rust compiler. We recognize a sequence of possibly escaped characters,
not including newlines, followed by a single quote. If there is no single quote
before the next newline (or end of block), we just recognize one possibly
escaped character.  We don't care about multi-character escapes like `\x7F` or
`\u{2764}` — the only escaped characters that really affect us are `\'`, `\"`,
and `\⟨`. 

We do need to recognize lifetime identifiers, since they also start with a
single quote. We parse non-ASCII identifiers since they're present in nightly.
(See the [unstable book][non-ascii]).

[non-ascii]: https://doc.rust-lang.org/nightly/unstable-book/language-features/non-ascii-idents.html
```rust
⟨Private methods⟩≡
    fn scan_through_character_literal(&mut self) -> TokenEnd {
        const POSSIBLY_ESCAPED_CHAR: &str = r"(?x-s: \\ . | [^\\\n] )";
        lazy_static! {
            static ref IDENTIFIER: Regex = Regex::new(
                r"^(?x:\p{XID_Start}\p{XID_Continue}*\b | _\p{XID_Continue}+\b)"
            ).unwrap();
            static ref ONE_CHAR: Regex = Regex::new(POSSIBLY_ESCAPED_CHAR).unwrap();
            static ref CHARS: Regex = Regex::new(
                &format!("^{}*?'", POSSIBLY_ESCAPED_CHAR)
            ).unwrap();
        }
        if let Some(len) = self.end_of(&*IDENTIFIER) {
            self.scan_from += len;
            if self.scan_from < self.text.len() && self.current_byte() == b'\'' {
                self.scan_from += 1;
            }
            None
        } else if let Some(len) = self.end_of(&*CHARS) {
            self.scan_from += len;
            None
        } else {
            if let Some(len) = self.end_of(&*ONE_CHAR) {
                self.scan_from += len;
            }
            else {
                self.scan_from = self.text.len();
            }
            Some((self.scan_from, Unterminated("character literal")))
        }
    }
```

Since block comments can be nested, we need to keep track of the nesting level,
incrementing it every time we see `/*` and decrementing every time we see `*/`.

```rust
⟨Private methods⟩≡
    fn scan_to_end_of_block_comment(&mut self) -> TokenEnd {
        lazy_static! {
            static ref BLOCK_COMMENT_SEGMENT: Regex = Regex::new(r"\*/|/\*").unwrap();
        }
        let mut level = 0;
        loop {
            let info = self.scan_through("block comment", &*BLOCK_COMMENT_SEGMENT);
            if info.is_some() {
                return info
            }
            if self.previous_byte() == b'*' { level += 1 }
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
    fn scan_to_end_of_raw_quote(&mut self, end_len: usize) -> TokenEnd {
        const END: &str = "\"##############################################################";
        if end_len <= END.len() {
            self.scan_through("raw string", &END[..end_len])
        } else {
            let closing = String::from("\"") + &String::from("#").repeat(end_len-1);
            self.scan_through("raw string", &closing[..])
        }
    }
```

