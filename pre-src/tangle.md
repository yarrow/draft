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
    ⟨Get cheese or panic⟩
        if ⟨Moon is made of green cheese⟩ {
            ⟨Export cheese to Earth⟩
        } else {
            panic!("No cheese!")
        }
    ```

would cause the bodies of the `⟨Moon is made of green cheese⟩` and `⟨Export
cheese to Earth⟩` sections to be inserted in the ⟨Get cheese or panic⟩ section.

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

## Implementation

`Tangle` has three methods:
- `let tangle = Tangle::new(text)` creates a new `Tangle` from a Markdown `str`.
- `tangle.get("")` gets the final product, concatenating the expansions of all
  the blocks in the unnamed section, while `tangle.get(section_name)` returns
  the section with the given name.
- The private method `expand` does the actual expansion.

```rust
use std::collections::HashMap;
use code_extractor::{CodeExtractor};
use block_parse::{BlockParse};
use Span;
use Ilk;

type CodeBlocks<'a> = HashMap<String, Vec<&'a str>>;
pub struct Tangle<'a> {
    sections: CodeBlocks<'a>,
}

use failure::Error;
impl<'a> Tangle<'a> {
    pub fn new(text: &'a str) -> Tangle<'a> {
        let mut sections = CodeBlocks::default();
        for (info, code) in CodeExtractor::new(text) {
            if info == "rust" {
                let (key, code) = extract_key(code); 
                let mut section = sections.entry(key).or_insert_with(|| vec![]);
                section.push(code);
            }
        }
        Tangle{sections}
    }
    pub fn get(&self, key: &str) -> Result<String, Error> {
        match self.sections.get(key) {
            Some(section) => Ok(self.expand(section)?),
            None => ⟨Complain that `key` was not found⟩
        }
    }
    fn expand(&self, section: &[&str]) -> Result<String, Error> {
        let mut expansion = String::new();
        for block in section {
            let mut current = 0; // Index of the last unprocessed byte of block
            for Span{lo, hi, ilk} in BlockParse::new(block) {
                // Append anything before the `Span` to `expansion`
                    if current < lo {
                        expansion += &block[current..lo];
                    }
                    current = hi;
                match ilk {
                    Ilk::SectionName => {
                        // Append the section name as a comment
                            expansion += "\n// ";
                            expansion += &block[lo..hi];
                            expansion += "\n";
                        // Append the section body
                        let key = normalize_whitespace(&block[lo+3..hi-3]);
                        expansion += &self.get(&key)?;
                    }
                    Ilk::Unterminated(kind) => {
                        eprintln!("unterminated {}:\n|{}|", kind, &block[lo..hi]);
                    }
                }
            }
            // Append anything after the last section name
                if current < block.len() {
                    expansion += &block[current..];
                }
        }
        Ok(expansion)
    }
}
```

If a section name isn't found, we reference it in our error message.  If the
empty string isn't found, there were no unnamed code blocks in the Markdown
file.

```rust
⟨Complain that `key` was not found⟩≡
    if key.is_empty() {
        bail!("No unnamed code blocks were found")
    } else {
        bail!("No section named ⟨{}⟩ was found", key)
    }
```

To add a block to the `sections` table, we must first check for a section
definition (something of the form `⟨...⟩≡`). If it exists, the key is the part
between `⟨` and `⟩`, with whitespace normalized.

```rust
use regex::Regex;
fn extract_key(text: &str) -> (String, &str) {
    lazy_static! {
        static ref TITLE: Regex = Regex::new(r"(?s)^\s*⟨(.*?)⟩(?:\+?)≡[ \t\r]*").unwrap();
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
    }
    String::from(WHITESPACE.replace_all(text.trim(), " "))
}
```

