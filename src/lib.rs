#[macro_use]
extern crate lazy_static;

extern crate regex;
use self::regex::Regex;

extern crate pulldown_cmark;
use pulldown_cmark::{Event, Tag};

#[derive(Clone, Debug, PartialEq)]
pub enum Chunklet<'a> {
    Text(&'a str),
    SectionName(&'a str),
}

impl<'a> Chunklet<'a> {
    pub fn key(&self) -> String {
        if let Chunklet::SectionName(key) = *self {
            const BRACKET_LEN: usize = 3;
            debug_assert!("⟨".as_bytes().len() == BRACKET_LEN);
            debug_assert!("⟩".as_bytes().len() == BRACKET_LEN);
            debug_assert!(key.find("⟨").unwrap() == 0);
            debug_assert!(key.rfind("⟩").unwrap() == key.len() - BRACKET_LEN);
            normalize_whitespace(&key[BRACKET_LEN..(key.len() - BRACKET_LEN)])
        } else {
            panic!("Only the SectionName variant has a key");
        }
    }
}

type Chunklets<'a> = Vec<Chunklet<'a>>;

#[derive(Clone, Debug, PartialEq)]
pub struct CodeChunk<'a> {
    code: Vec<Chunklet<'a>>,
    first: bool, // Is this the first chunk of the section?
    language: String,
    offset: usize,
}

pub struct CodeFilter<'a> {
    source: &'a str,
    pulldown: pulldown_cmark::Parser<'a>,
}

impl<'a> CodeFilter<'a> {
    pub fn new(text: &'a str) -> CodeFilter<'a> {
        CodeFilter {
            source: text,
            pulldown: pulldown_cmark::Parser::new(text),
        }
    }
}

type KeyedChunk<'a> = (String, CodeChunk<'a>);
impl<'a> Iterator for CodeFilter<'a> {
    type Item = KeyedChunk<'a>;

    fn next(&mut self) -> Option<KeyedChunk<'a>> {
        loop {
            let event = self.pulldown.next()?;
            if let Event::Start(Tag::CodeBlock(c)) = event {
                let language = String::from(c);
                let offset = self.pulldown.get_offset();
                let mut end = offset;
                let mut event;
                loop {
                    event = self.pulldown.next()?;
                    match event {
                        Event::Text(_) => end = self.pulldown.get_offset(),
                        _ => break,
                    }
                }
                if let Event::End(Tag::CodeBlock(_)) = event {
                    let (body, first, key) = extract_key_info(&self.source[offset..end]);
                    let code = to_chunklets(body);
                    return Some((
                        key,
                        CodeChunk {
                            code,
                            first,
                            language,
                            offset,
                        },
                    ));
                } else {
                    panic!(
                        "Unexpected (non-Text) pulldown event between\
                         Start(CodeBlock) and End(CodeBlock): {:?}",
                        event
                    );
                }
            }
        }
    }
}

fn extract_key_info(text: &str) -> (&str, bool, String) {
    lazy_static! {
        static ref TITLE: Regex = Regex::new(r"(?s)^\s*⟨(.*?)⟩(\+?)≡[ \t\r]*").unwrap();
    }
    let mut text = text;
    let mut first = false;
    let mut key = String::from("");
    if let Some(title) = TITLE.captures(text) {
        let raw_key = title.get(1).unwrap();
        key = normalize_whitespace(raw_key.as_str());

        let continuation = title.get(2).unwrap();
        first = continuation.as_str() != "+";

        let next_byte = title.get(0).unwrap().end();
        text = &text[next_byte..];
        if !text.is_empty() && text.as_bytes()[0] == b'\n' {
            text = &text[1..];
        }
    }
    (text, first, key)
}

pub fn normalize_whitespace(text: &str) -> String {
    lazy_static! {
        static ref WHITESPACE: Regex = Regex::new(r"\s+").unwrap();
    }
    String::from(WHITESPACE.replace_all(text.trim(), " "))
}

pub fn to_chunklets(text: &str) -> Chunklets {
    lazy_static! {
        static ref SECTION: Regex = Regex::new(r"(?s)⟨.*?⟩").unwrap();
    }

    let mut code = Chunklets::new();
    let mut current = 0;
    for section_name in SECTION.find_iter(text) {
        if current < section_name.start() {
            code.push(Chunklet::Text(&text[current..section_name.start()]));
        }
        code.push(Chunklet::SectionName(section_name.as_str()));
        current = section_name.end();
    }
    if current < text.len() {
        code.push(Chunklet::Text(&text[current..]));
    }
    code
}

use std::collections::HashMap;

type DraftWeb<'a> = HashMap<String, Vec<CodeChunk<'a>>>;

#[derive(Debug)]
pub struct Draft<'a> {
    web: DraftWeb<'a>,
}

impl<'a> Draft<'a> {
    pub fn new(text: &'a str) -> Draft<'a> {
        let parser = CodeFilter::new(text);
        let mut draft = Draft {
            web: DraftWeb::default(),
        };
        for (key, chunk) in parser {
            if chunk.language == "rust" {
                //FIXME
                let mut chunks = draft.web.entry(key).or_insert_with(|| vec![]);
                chunks.push(chunk);
            }
        }
        draft
    }

    pub fn text_of(&self, key: &str) -> Option<String> {
        let chunks = self.web.get(key)?;
        let mut text = String::new();
        for chunk in chunks {
            for chunklet in &chunk.code {
                match *chunklet {
                    Chunklet::Text(t) => text.push_str(t),
                    Chunklet::SectionName(name) => { //FIXME: eliminate intermediate string result
                        if let Some(t) = self.text_of(&chunklet.key()) {
                            text.push_str("\n// ");
                            text.push_str(name);
                            text.push_str("\n");
                            text.push_str(t.as_str());
                        }
                    }
                }
            }
        }
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /*
    #[test]
    fn test_code_filter() {
        let mut stuff = String::new();
        for (language, chunk) in CodeFilter::new(source()) {
            stuff.push_str(&format!("```{}\n{}```\n", language, chunk.code));
        }
        assert_eq!(stuff, expected());
    }
*/
    #[test]
    fn test_draft_web_rust() {
        let src = source();
        let web = Draft::new(&src);
        let rust = web.text_of("").unwrap();
        assert_eq!(rust, just_rust());
    }

    /*
    fn expected() -> String {
        format!(
            r"```
Cargo? What cargo?
```
```rust
{}```
",
            just_rust()
        )
    }
*/

    fn just_rust() -> &'static str {
        r#"fn a () { "bee" }

println!("{}", a());
"#
    }
    fn source() -> &'static str {
        r#"
# heading
text is *text*

###### ⟨subhead⟩

## Head sub

And a paragraph
gggg

```
Cargo? What cargo?
```

```rust
fn a () { "bee" }

println!("{}", a());
```
"#
    }
}
