Extract Code Blocks from Markdown
=================================

The job of CodeFilter is to return, for each code block in a Markdown file, the
code block's (possibly empty) info string, the line number on which the code
block begins, and the code block itself (as a single `str`).

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
crate), which supports the [Commonmark specification](http://spec.commonmark.org/)
and is now the parser of choice for `rustdoc`.

```rust
extern crate pulldown_cmark;
```

Pulldown's parser iterator returns a stream of events, of which the relevant
ones for us are the `Start(CodeBlock())` and `End(CodeBlock())` events, and the
`Text()` events between them.

```rust
use self::pulldown_cmark::{Event, Parser, Tag};
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
        let text = "lorem ipsum\n```rust\nlet lorem = ip.sum();\n\n```\n";
        let expected = Info{
            start: text.find("let lorem").unwrap(),
            end: text.find("```\n").unwrap(),
            value: "let lorem = ip.sum();\n\n",
        };
        let mut pulldown = Parser::new(text);
        ⟨Find `start` and `end` as calculated by `pulldown`⟩;
        let actual = Info{start, end, value: &text[start..end]};
        assert_eq!(actual, expected);
    }

    ⟨Other tests⟩
}
```

To find the start of the block, we scan for the first `Start(CodeBlock(_))`
returned by `next()`. At that point we know that `get_offset()` returns the
position of the first byte beyond the line with the code fence — that is, the
first byte of the code block proper.  Then we scan through the `Text` events,
setting `end` to the end of each `Text` event in turn. When we're done, `end`
will be the end of the last `Text`, which is the end of the entire code block.

```rust
⟨Find `start` and `end` as calculated by `pulldown`⟩≡
    let mut start = 0;
    let mut end = 0;
    while let Some(event) = pulldown.next() {
        match event {
            Start(CodeBlock(_)) => start = pulldown.get_offset(),
            Text(_) => end = pulldown.get_offset(),
            _ => (),
        }
    }
```

## Extracting code blocks

We're going to build an iterator to pull out code blocks with info strings from
a text string `text` (usually the entire contents of a Markdown file).  Each
item returned will be a tuple of a `String` (the info string), a `usize` (the
line number where the code block begins) and an immutable `&str` which points to
the substring of `text` containing the code block proper.

```rust
type InfoAndRawCode<'a> = (String, usize, &'a str);
```

Because `pulldown`'s parser returns each code block as a series of `Text` events
rather than as one large text event, we'll use the strings embedded in pulldown
events only for the info string.  For the code block proper, we'll use
`get_offset()`, as above, to find the start and end of the code block, returning
a slice of `text` (whose lifetime must therefore be the same as the lifetime of
`text`).  We'll also use a `LineCounter` struct (created in the **Calculating
line numbers** section).

```rust
pub struct CodeFilter<'a> {
    text: &'a str,
    pulldown: Parser<'a>,
    lc: LineCounter<'a>,
}
```

```rust
impl<'a> CodeFilter<'a> {
    pub fn new(text: &'a str) -> CodeFilter<'a> {
        CodeFilter {text, pulldown: Parser::new(text), lc: LineCounter::new(text)}
    }
}
```

We can't just `map()` and `filter()` the output of the pulldown parser, since
the `InfoAndRawCode` items we output may depend on an arbitrary number of
pulldown events.  Nevertheless, the logic is straightforward, not too different
from our test above.

```rust
impl<'a> Iterator for CodeFilter<'a> {
    type Item = InfoAndRawCode<'a>;
    
    fn next(&mut self) -> Option<InfoAndRawCode<'a>> {
        loop {
            let event = self.pulldown.next()?;
            if let Start(CodeBlock(info)) = event {
                let info = String::from(info);
                ⟨Find `start`, `end`, and the first non-`Text` `event`⟩;
                if let End(CodeBlock(_)) = event {
                    return Some((info, self.lc.line_of(start), &self.text[start..end]));
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
        CodeFilter::new(text).map(|x| x.0).collect()
    }
    fn line_numbers(text: &str) -> Vec<usize> {
        CodeFilter::new(text).map(|x| x.1).collect()
    }
    fn code_texts(text: &str) -> Vec<&str> {
        CodeFilter::new(text).map(|x| x.2).collect()
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
        assert_eq!(line_numbers(markdown), vec![2, 7]);
        assert_eq!(code_texts(markdown), code);
    }

    #[test]
    fn test_degenerate_rust_block() { // Mostly to check that it doesn't panic
        let markdown = "```rust"; // No newline, no actual code.
        assert_eq!(info_strings(markdown), vec!["rust".to_string()]);
        assert_eq!(line_numbers(markdown), vec![0]);
        assert_eq!(code_texts(markdown), vec![""]);
    }
```

## Calculating line numbers

Once we know the starting byte `s` of a code block, we use a `LineCounter` to
find the corresponding line number, which we define as the number of newline
characters in `&text[0..s]`. (So the first line of the file is line 0.)

The `LineCounter` method `line_of()` returns the line number corresponding to a
given offset into the Markdown text.  Since we'll use `line_of()` only to find
the starting line number of each code block, we can require that its inputs be
non-increasing.

We'll count the terminating newline as part of each line, and won't require a
newline at the very end of the text.  For efficiency's sake the `line_of()`
method is sticky — having once returned a number `n`, it never returns a lesser
number.

Here are tests that say those things:

```rust
⟨Other tests⟩≡
    fn lc(text: &str) -> LineCounter { LineCounter::new(text) }
    #[test]
    fn test_line_counter() {
        // The first line number is 0:
            assert_eq!(lc("abc\nx").line_of(2), 0);

        // A line includes its terminating newline character:
            assert_eq!(lc("a\n").line_of(1), 0);

        // ... even if it's the first character of the text:
            assert_eq!(lc("\na\n").line_of(0), 0);

        // The next line begins immediately after the newline of the previous line:
            assert_eq!(lc("a\nb").line_of(2), 1);

        // We don't require a newline at the end of text:
            assert_eq!(lc("ab").line_of(2), 0);

        // We act as if offsets beyond the end of the text were in one long line
            let abc = "a\nb\nc";
            let abcn = "a\nb\nc\n";
            assert_eq!(lc("ab").line_of(9), 0);
            assert_eq!(lc(abc).line_of(abc.len()+1), 2);
            assert_eq!(lc(abc).line_of(abcn.len()+1), 2);
            assert_eq!(lc(abc).line_of(abc.len()+100), 2);
            assert_eq!(lc(abc).line_of(abcn.len()+100), 2);

        // The `line_of()` method is sticky
            let mut c = lc("a\nb\nc\n");
            assert_eq!(c.line_of(1), 0);
            assert_eq!(c.line_of(4), 2);
            assert_eq!(c.line_of(1), 2);
    }
```

The `memchr` crate gives us a fast interator over the postions of newlines in
the the Markdown text, and we'll create a `Line` struct to hold information on
the last line found so far.  If our `Memchr` object ever runs out of newlines,
we pretend that there is a virtual newline at position `usize::MAX`.

```rust
extern crate memchr;
use self::memchr::Memchr;

struct MemchrX<'a>(Memchr<'a>);
impl<'a> MemchrX<'a> {
    fn new(text: &'a str) -> MemchrX<'a> {
        MemchrX(Memchr::new(b'\n', text.as_bytes()))
    }
    fn next(&mut self) -> usize {
        match self.0.next() {
            Some(n) => n,
            None => ::std::usize::MAX,
        }
    }
}

#[derive(Debug)]
struct Line {end: usize, number: usize}

pub struct LineCounter<'a> {
    newlines: MemchrX<'a>,
    current: Line,
}
```

We'll be using our tweaked `MemchrX` to find the next newline at or after a
given offset, incrementing the line number each time we call `next()`.  We store
the current line end so we know whether or not to call `next()` again for a
given offset, and the current line number so we can return or update it.

```rust
impl<'a> LineCounter<'a> {
    fn new(text: &'a str) -> LineCounter<'a> {
        let mut newlines = MemchrX::new(text);
        let end = newlines.next();
        LineCounter{newlines, current: Line {end, number: 0}}
    }
    fn line_of(&mut self, offset: usize) -> usize {
        ⟨Calculate and return the line number⟩
    }
}
```

Each line counter has the invariant that `current.end` is always the end of the
current line: the position of the first newline at or after the last `offset`
argument to `line_of()` (or simply the first newline, for the line counter value
returned by `new()`); and that `current.number` is the count of newlines whose
postion is strictly less than `current.end`.

Calling `newlines.next()` gives us the offset of the next newline in `text`. So
to find the line number of the `offset` parameter, we keep calling
`newline.next()` until it returns a number at least as great as `offset`.

```rust
⟨Calculate and return the line number⟩≡
    while self.current.end < offset {
        self.current.number += 1;
        self.current.end = self.newlines.next();
    }
    self.current.number
```
