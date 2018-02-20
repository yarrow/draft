Extract Code Blocks from Markdown
=================================

The job of CodeExtractor is to return, for each code block in a Markdown file, the
code block's (possibly empty) info string and the code block itself (as a single `str`).

Commonmark's flavor of Markdown provides for
[fenced code blocks](http://spec.commonmark.org/0.28/#fenced-code-blocks),
a block of literal text with an optional
[info string](http://spec.commonmark.org/0.28/#info-string). Such a code block
might look something like this:

````
```rust
// Some Rust code (not necessarily *legal* Rust code!)
pub mod my_module;
```
````

## Pulldown Cmark

Parsing Markdown is not trivial, so we'll use pulldown (the `pulldown_cmark`
crate), which supports the [Commonmark
specification](http://spec.commonmark.org/) and is now the parser of choice for
`rustdoc`.  Pulldown's parser iterator returns a stream of events, of which the
relevant ones for us are the `Start(CodeBlock())` and `End(CodeBlock())`
events, and the `Text()` events between them.

```rust
use pulldown_cmark::{Event, Parser, Tag};
use self::Event::{Start, End, Text};
use self::Tag::{CodeBlock};
```

Pulldown's README says:

> source-map information (the mapping between parsed blocks and offsets within
> the source text) is readily available; you basically just call `get_offset()` as
> you consume events.

I couldn't find further documentation on `get_offset()`.

Here are the properties of pulldown that we rely on.  Assume we create a parser
`p` by a statement like `let mut p = Parser::new(text)`. Then

- If `p.next()` returns an event matching `Start(CodeBlock(_))`, then  
  `              let start = p.get_offset()`  
  sets `start` to the offset of the first byte of the code
  block proper in `text`. In other words, the code block is a prefix of
  `&text[start..]`.
- after returning `Start(CodeBlock(_))`, `p.next()` will eventually return an
  `End(CodeBlock(_))`;
- between those two events `p.next()` will return only zero or more `Text`
  events;
- after `p.next()` returns a `Text` event, `&text[start..p.get_offset()]` is a
  prefix of the code block through and including `Text`.

Here's a test of those things (meant as a check that my understanding of
`get_offset()` isn't completely insane, not as a test of pulldown).  We put this
and other unit tests in the usual `tests` module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_get_offset() {
        #[derive(Debug, PartialEq)]
        struct Info<'a>{start: usize, end: usize, value: &'a str};
        let text = "lorem ipsum\n```rust\nlet lorem = ip.sum();\n\n```\nDone.";
        let mut pulldown = Parser::new(text);
        ⟨Calculate `start` and `end` via `pulldown`⟩;
        assert_eq!(start, text.find("let lorem").unwrap());
        assert_eq!(end, text.find("```\n").unwrap());
        assert_eq!(&text[start..end], "let lorem = ip.sum();\n\n");
    }

    ⟨Other tests⟩
}
```

To find the start of the block, we scan for the first `Start(CodeBlock(_))`
returned by `next()`. At that point we know that `get_offset()` returns the
position of the first byte beyond the line with the code fence — that is, the
first byte of the code block proper.  Then we scan through the `Text` events,
setting `end` to the end of each `Text` event in turn until we see an
`End(CodeBlock(_))`. At that point, `end` is the end of the last `Text` in the
block, which is the end of the entire code block.

```rust
⟨Calculate `start` and `end` via `pulldown`⟩≡
    let mut start = 0;
    let mut end = 0;
    while let Some(event) = pulldown.next() {
        match event {
            Start(CodeBlock(_)) => start = pulldown.get_offset(),
            Text(_) => end = pulldown.get_offset(),
            End(CodeBlock(_)) => break,
            _ => if start != 0 { // We're in the code block, so ...
                panic!("Unexpected Event {:?}", event);
            }
        }
    }
```

## Extracting code blocks

We're going to build an iterator to pull out code blocks with info strings from
a text string `text` (usually the entire contents of a Markdown file).  Each
item returned will be a tuple of a `String` (the info string) and an immutable
`&str` which points to the substring of `text` containing the code block proper.

```rust
type RawCode<'a> = (String, &'a str);
```

Because `pulldown`'s parser returns each code block as a series of `Text` events
rather than as one large text event, we'll use the strings embedded in pulldown
events only for the info string.  For the code block proper, we'll use
`get_offset()`, as above, to find the start and end of the code block, returning
a slice of `text` (whose lifetime must therefore be the same as the lifetime of
`text`).

```rust
pub(crate) struct CodeExtractor<'a> {
    text: &'a str,
    pulldown: Parser<'a>,
}
```

```rust
impl<'a> CodeExtractor<'a> {
    pub(crate) fn new(text: &'a str) -> CodeExtractor<'a> {
        CodeExtractor {text, pulldown: Parser::new(text)}
    }
}
```

We can't just `map()` and `filter()` the output of the pulldown parser, since
the `RawCode` items we output may depend on an arbitrary number of
pulldown events.  Nevertheless, the logic is straightforward, not too different
from our test above.

```rust
impl<'a> Iterator for CodeExtractor<'a> {
    type Item = RawCode<'a>;

    fn next(&mut self) -> Option<RawCode<'a>> {
        loop {
            let event = self.pulldown.next()?;
            if let Start(CodeBlock(info)) = event {
                let info = String::from(info);
                ⟨Find `start`, `end`, and the first non-`Text` `event`⟩;
                if let End(CodeBlock(_)) = event {
                    return Some((info, &self.text[start..end]));
                }
                else { ⟨Handle an unexpected event⟩ }
            }
        }
    }
}
```

Having seen a `Start(CodeBlock(_))` event, we know that `get_offset()` is the
index of the first byte of the code block proper.  To find the block's end, we
scan events, setting `end` to the end of each `Text` event in turn, until we
find a non-`Text` event.

```rust
⟨Find `start`, `end`, and the first non-`Text` `event`⟩≡
    let start = self.pulldown.get_offset();
    let mut end = start;
    let mut event;
    loop {
        event = self.pulldown.next()?;
        match event {
            Text(_) => end = self.pulldown.get_offset(),
            _ => break,
        }
    }
```

There seems to be no way for any `Text` events to occur between
`Start(CodeBlock(_))` and `End(CodeBlock(_))`.  Currently we panic if one does
occur; perhaps it would be better just to ignore them?

```rust
⟨Handle an unexpected event⟩≡
    panic!("Can't handle non-Text event between Start(CodeBlock) and \
            End(CodeBlock): {:?}", event)
```

Here are some tests of the above code.

```rust
⟨Other tests⟩+≡
    fn info_strings(text: &str) -> Vec<String> {
        CodeExtractor::new(text).map(|x| x.0).collect()
    }
    fn code_texts(text: &str) -> Vec<&str> {
        CodeExtractor::new(text).map(|x| x.1).collect()
    }

    #[test]
    fn test_code_filter() {
        let code = ["let a = 0;\nlet b = 1;\n", "def init(n)\n  @n = n\nend\n"];
        let markdown_string = format!(
            "##Some Rust\n```rust\n{}```\n##And Ruby\n```ruby\n{}```\n",
            code[0], code[1]
        );
        let markdown = markdown_string.as_str();
        assert_eq!(info_strings(markdown), vec!["rust".to_string(), "ruby".to_string()]);
        assert_eq!(code_texts(markdown), code);
    }

    #[test]
    fn test_degenerate_rust_block() { // Will a code block with no text panic?
        let markdown = "```rust";
        assert_eq!(info_strings(markdown), vec!["rust".to_string()]);
        assert_eq!(code_texts(markdown), vec![""]);
    }
```

