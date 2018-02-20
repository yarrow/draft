# Tangle

The job of `Tangle` is to extract the code blocks from a Markdown `str`, discard
those that are irrelevant, and concatenate the remainder as output. It's
actually a little more complicated than that: a code block may optionally have a
name (a phrase or sentence prefixed by `⟨` and suffixed by '⟩', such as
`⟨Do an interesting thing⟩`).  Other block may reference the named block,
inserting it as if it were a parameterless macro.

Unnamed blocks are treated as if their name was the empty string.  If there are
no named blocks, the result of `Tangle.new(&source).get("")` is the simply
concatenation of the code blocks labeled `rust` in `source`.

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

As we said above, a block might contain a reference to another block:

    ```rust
    pub fn isqrt(n: u32) -> u32 {
        ⟨Set `root` to the least `r` such that `r * r ≥ n`⟩
        root
    }
    ```

...where the named block might later be defined as:

    ```rust
    ⟨Set `root` to the least `r` such that `r * r ≥ n`⟩≡
		let root = (n as f64).sqrt() as u32;
    ```

Admittedly this seems like overkill — but suppose the named section was actually
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
        let name = "⟦Section⟧";
        let definition = format!("{}≡\n{}\n", name, body);
        let source = rust(&block_with(name)) + "# Head\n" + &rust(&definition);
        assert_eq!(clip(&tangle(&source)), clip(&block_with(body)));
    }
```

## Implementation

`Tangle` has three methods:
- `let tangle = Tangle::new(text)` creates a new `Tangle` from a Markdown `str`.
- `tangle.get("")` gets the final product, concatenating the expansions of all
  the unnamed blocks, while `tangle.get(title)` returns the block with the name
  contained in `title`.
- The private method `expand` does the actual expansion.

```rust
use std::collections::HashMap;
use code_extractor::{CodeExtractor};

type CodeBlocks<'a> = HashMap<String, Vec<&'a str>>;
pub struct Tangle<'a> {
    code_blocks: CodeBlocks<'a>,
}

use failure::Error;
impl<'a> Tangle<'a> {
    pub fn new(text: &'a str) -> Tangle<'a> {
        let mut code_blocks = CodeBlocks::default();
        for (info, code) in CodeExtractor::new(text) {
            if info == "rust" {
                let (key, code) = extract_key(code); 
                let mut blocks = code_blocks.entry(key).or_insert_with(|| vec![]);
                blocks.push(code);
            }
        }
        Tangle{code_blocks}
    }
    pub fn get(&self, key: &str) -> Result<String, Error> {
        match self.code_blocks.get(key) {
            Some(code) => Ok(self.expand(code)?),
            None => ⟨Complain that `key` was not found⟩
        }
    }
    fn expand(&self, blocks: &[&str]) -> Result<String, Error> {
        lazy_static! {
            static ref SECTION: Regex = Regex::new(r"(?s)⟦.*?⟧").unwrap();
        }
        let mut expansion = String::new();
        for block in blocks {
            let mut current = 0; // Index of the last unprocessed byte of block
            for section_name in SECTION.find_iter(block) {
                let (start, end) = (section_name.start(), section_name.end());
                // Append anything before the section name to `expansion`
                    if current < start {
                        expansion += &block[current..start];
                    }
                    current = end;
                // Append the section name as a comment
                    expansion += "\n// ";
                    expansion += section_name.as_str();
                    expansion += "\n";
                // Append the section body
                let key = normalize_whitespace(&block[start+3..end-3]);
                expansion += &self.get(&key)?;
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
empty string isn't found, there were no unnamed code sections in the Markdown
file.

```rust
⟨Complain that `key` was not found⟩≡
    if key.is_empty() {
        bail!("No unnamed code blocks were found")
    } else {
        bail!("No code block named ⟦{}⟧ was found", key)
    }
```

To add a block to the `code_blocks` table, we must first check for a section
definition (something of the form `⟨...⟩≡`). If it exists, the key is the part
between `⟨` and `⟩`, with whitespace normalized.

```rust
use regex::Regex;
fn extract_key(text: &str) -> (String, &str) {
    lazy_static! {
        static ref TITLE: Regex = Regex::new(r"(?s)^\s*⟦(.*?)⟧(?:\+?)≡[ \t\r]*").unwrap();
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

