# Tangle

The job of `Tangle` is to extract the code blocks from a Markdown string,
discard those that are irrelevant, and concatenate the remainder as output. It's
actually a little more complicated than that: a code block is part of a code
*section*, either named or unnamed.  A named section is introduced as in the
following example:

    ```rust
    ⟨Example⟩≡
        some.code.goes("here");
    ```

where the name is a phrase or sentence prefixed by `⟨` and suffixed by `⟩` and
followed by `≡`.  Other sections may reference the named section, inserting it
as if it were a parameterless macro.  A section may consist of multiple disjoint
code blocks: the following would cause `more_code("goes here")` to be appended
to the `⟨Example⟩` section:

    ```rust
    ⟨Example⟩≡
        more_code("goes here")
    ```

Blocks not introduced by a name are treated as part of the unnamed section
string.  If there are no named blocks, the result of
`Tangle.new(&source).get("")` is the simply concatenation of the code blocks
labeled `rust` in `source`.

Code sections may reference one another:

    ```rust
    ⟨Get cheese or panic⟩≡
        if Moon::is_made_of("green cheese") {
            ⟨Export cheese to Earth⟩
        } else {
            panic!("No cheese!")
        }
    ```

would cause the body of the `⟨Export cheese to Earth⟩` section to be inserted in
the ⟨Get cheese or panic⟩ section.

## Tests

Let's test that:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn rust(r: &str) -> String { format!("\n```rust\n{}```\n", r) }
    fn cpp(c: &str) -> String { format!("\n```c++\n{}```\n", c) }
    fn clip(text: &str) -> String {
        let comments = Regex::new(r"//.*\n").unwrap();
        let spaces = Regex::new(r"\s+").unwrap();
        let uncomment = comments.replace_all(text, "\n");
        spaces.replace_all(&uncomment, " ").to_string()
    }
    fn tangle(text: &str) -> String {
        Tangle::new(text).get("").unwrap()
    }
    #[test]
    fn tangle_plain_code() {
        let (a, b, c, d) = ("a\n", "b\n", "c\n", "d\n");
        let src = format!(
            "# TITLE\nblah blah\n{}x{}y{}{}",
            rust(a), rust(b), cpp(c), rust(d)
        );
        let tangled = tangle(&src);
        let expected = String::from(a)+b+d;
        assert_eq!(clip(&tangled), clip(&expected));
    }
    ⟨Other tests and helpers⟩
}
```

As we said above, a block might contain a reference to a code section:

    ```rust
    pub fn isqrt(n: u32) -> u32 {
        ⟨Set `root` to the least `r` such that `r * r ≥ n`⟩
        root
    }
    ```

...where the named section might later be defined as:

    ```rust
    ⟨Set `root` to the least `r` such that `r * r ≥ n`⟩≡
        let root = (n as f64).sqrt() as u32;
    ```

This example may seem like overkill — but suppose the named section was actually
an implementation of Newton's method, intended for a machine with very slow
floating point operations.

Here's a test that named sections do indeed get expanded:

```rust
⟨Other tests and helpers⟩≡
    fn block_with(text: &str) -> String {
        format!("fn that_uses() {{ {} }}\n", text)
    }
    #[test]
    fn tangle_section_names() {
        let body = "section.body";
        let name = "⟨Section⟩";
        let definition = format!("{}≡\n{}\n", name, body);
        let source = rust(&block_with(name)) + "# Head\n" + &rust(&definition);
        assert_eq!(clip(&tangle(&source)), clip(&block_with(body)));
    }
```

Tangle will also report user errors.

```rust
⟨Other tests and helpers⟩≡
    fn errors(body: &str) -> Vec<String> {
        let rb = rust(body);
        let mut t = Tangle::new(&rb);
        t.get("").unwrap();
        t.errors()
    }
    #[test]
    fn tangle_simple_error() {
        assert_eq!(
            errors("    \"strin"),
            vec![String::from("Unterminated double quote string at line 2, col 4\n")]
        );
    }
```
        
## Implementation

blah blah blah
```rust
use std::collections::HashMap;
use std::fmt;

use code_extractor::CodeExtractor;
use block_parse::BlockParse;
use line_counter::LineCounter;
use failure::Error;

use Span;
use Ilk;
```

`Tangle` has three public methods:
- `let tangle = Tangle::new(text)` creates a new `Tangle` from a Markdown `str`.
- `tangle.get("")` gets the final product, concatenating the expansions of all
  the blocks in the unnamed section, while `tangle.get(section_name)` returns
  the section with the given name.
- `tangle.errors()` returns a vector of strings with error messages.

```rust
pub struct Tangle<'a> {
    sections: SectionMap<'a>,
    errors: ErrMsgs,
}

impl<'a> Tangle<'a> {
    pub fn new(text: &'a str) -> Tangle<'a> {
        let mut lc = LineCounter::new(text);
        let mut sections = SectionMap::default();
        for (code, info_string, offset) in CodeExtractor::new(text) {
            if info_string == "rust" {
                let (key, block) = extract_key(code); 
                let (block_line, block_col) = lc.line_and_column(offset);
                let mut section = sections.entry(key).or_insert_with(|| vec![]);
                section.push(BlockInfo{block, block_line, block_col});
            }
        }
        Tangle{sections, errors: Vec::new()}
    }
    pub fn errors(&self) -> Vec<String> {
        self.errors.iter().map(|e| format!("{}", e)).collect()
    }

    pub fn get(&mut self, key: &str) -> Result<String, Error> {
        let compressed = match self.sections.get_section(key) {
            Ok(section) => section,
            Err(ilk) => bail!("{}", ilk),
        };
        let mut errors = Vec::new();
        let expansion = self.expand(compressed, &mut errors);
        self.errors = errors;
        Ok(expansion)
    }
```

The bulk of the work is done in the private `expand` method. It examines each
block in the given `section`, and each `span` in the block, expanding section
name references and adding appropriate errors.

```rust
    fn expand(&self, section: &[BlockInfo], mut errors: &mut ErrMsgs) -> String {
        let mut expansion = String::new();
        for block_info in section {
            let mut lc = LineCounter::new(block_info.block);
            for span in BlockParse::new(block_info.block) {
                match span.ilk {
                    Ilk::SectionName => {
                        let key = slice(block_info, &span);
                        match self.sections.get_section(key) {
                            Ok(section_body) => {
                                // Append the section name as a comment, then the body
                                expansion += "\n// ";
                                expansion += key;
                                expansion += "\n";
                                expansion += &self.expand(section_body, &mut errors);
                            }
                            Err(ilk) => {
                                expansion += key;
                                errors.push(err_msg(&mut lc, block_info, span));
                            }
                        }
                    }
                    Ilk::Unterminated(kind) => errors.push(err_msg(&mut lc, block_info, span)),
                    Ilk::JustCode => expansion += slice(block_info, &span),
                    _ => unreachable!(),
                }
            }
        }
        expansion
    }
}
```

The `SectionMap` field of `Tangle` is a map from strings to vectors of
`BlockInfo`, and `BlockInfo` struct consists of a pointer to a string slice
together with the line and column of the start of the string in the `text`
passed to `Tangle::new`.

```rust
type SectionMap<'a> = HashMap<String, SectionBlocks<'a>>;
type SectionBlocks<'a> = Vec<BlockInfo<'a>>; 
struct BlockInfo<'a>  {
    block: &'a str,
    block_line: usize,
    block_col: usize,
}

fn slice<'a>(info: &BlockInfo<'a>, span: &Span) -> &'a str {
    &info.block[span.lo..span.hi]
}
```

A `Span` has line and column numbers relative to the beginning of its block.  To
produce accurate line numbers, we need to add the block's line number to the
span's line number. And if (as may happen) a block doesn't begin in column zero,
then spans in the first line of the block need to have an adjusted colum.

```rust
fn err_msg(mut lc: &mut LineCounter, info: &BlockInfo, span: Span) -> ErrMsg {
    let (line, col) = {
        let (l, c) = lc.line_and_column(span.lo);
        if l == 0 { (info.block_line, info.block_col + c) }
        else { (info.block_line + l, c ) }
    };
    ErrMsg{pos: Pos{line, col}, ilk: span.ilk}
}

#[derive(Debug, PartialEq, Eq)]
struct Pos {
    line: usize,
    col: usize,
}
struct ErrMsg {
    pos: Pos,
    ilk: Ilk,
}
impl fmt::Display for ErrMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} at line {}, col {}\n", self.ilk, self.pos.line, self.pos.col)
    }
}
type ErrMsgs = Vec<ErrMsg>;
```

We use a specialized method, `get_section`, to look up sections in a
`SectionMap`: removing extraneous whitespace from the key, and returning an
`Ilk` value if the key is not found.

```rust
trait GetSection {
    fn get_section(&self, key: &str) -> Result<&SectionBlocks, Ilk>;
}
impl<'a> GetSection for SectionMap<'a> {
    fn get_section(&self, key: &str) -> Result<&SectionBlocks, Ilk> {
        match self.get(&normalize_whitespace(key)) {
            Some(section) => Ok(section),
            None => Err(Ilk::NotFound(
                if key.is_empty() { "No unnamed code blocks were found".to_string() }
                else { format!("Section ⟨{}⟩ was never defined", key) }
            ))
        }
    }
}
```

To add a block to the `sections` table, we must first check for a section
definition (something of the form `⟨...⟩≡`). If it exists, the key is the part
between `⟨` and `⟩`, with whitespace normalized.

```rust
use regex::Regex;
fn extract_key(text: &str) -> (String, &str) {
    lazy_static! {
        static ref TITLE: Regex = Regex::new(r"(?s)^\s*(⟨.*?⟩)(?:\+?)≡[ \t\r]*").unwrap();
    }
    let mut text = text;
    let mut key = String::from("");
    if let Some(title) = TITLE.captures(text) {
        let raw_key = title.get(1).unwrap();
        key = normalize_whitespace(raw_key.as_str());

        let next_byte = title.get(0).unwrap().end();
        text = &text[next_byte..];
        if !text.is_empty() && text.as_bytes()[0] == b'\n' {
            text = &text[1..];
        }
    }
    (key, text)
}
fn normalize_whitespace(text: &str) -> String {
    lazy_static! {
        static ref WHITESPACE: Regex = Regex::new(r"\s+").unwrap();
        static ref SECTION_NAME: Regex = Regex::new(r"(?s:^⟨\s*(.*?)\s*⟩$)").unwrap();
    }
    if let Some(inside) = SECTION_NAME.captures(text) {
        let inside = inside.get(1).unwrap();
        String::from(WHITESPACE.replace_all(inside.as_str(), (" ")))
    }
    else {
        String::from(text)
    }
}
```

