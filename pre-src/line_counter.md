Find Line Numbers Given Character Offsets
=========================================

The job of a `LineCounter` is to return the line number of a given character
offset `n`, via the `line_of()` method.  We define the line of `n` as the number
of newline characters in `&text[0..n]`. (So the first line of the file is line 0.)

We count the terminating newline as part of each line, and don't require a
newline at the very end of the text.

For efficiency and simplicity, the outputs of `line_of()` are non-decreasing:
once `line_of()` has returned `n`, it will never return an `m < n`. In other
other words, for a given `LineCounter` `lc`, `lc.line_of(k) == n` and the first
character of line `n` is `j`, then `lc.line_of(i)` will return the wrong answer
for `i < j`.  (In optimized builds we just return the wrong answer if `x < n`; in
non-optimized builds we'll use `debug_assert` to panic instead.)

Here are tests that say those things:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    fn lc(text: &str) -> LineCounter { LineCounter::new(text) }

    #[test] fn lc_first_is_0()  { // The first line number is 0
        assert_eq!(lc("abc\nx").line_of(2), 0);
    }
    #[test] fn lc_incl_newline() { // A line includes its terminating newline character
        assert_eq!(lc("a\n").line_of(1), 0);
        // ... even if it's the first character of the text:
        assert_eq!(lc("\na\n").line_of(0), 0);
    }
    #[test] fn lc_begins_immediately() {
        // Next line begins immediately after the newline of the previous line
        assert_eq!(lc("a\nb").line_of(2), 1);
    }
    #[test] fn lc_no_last_newline() { // We don't require a newline at the end of text:
        assert_eq!(lc("ab").line_of(2), 0);
    }
    #[test] fn lc_virtual_last() {
        // We act as if offsets beyond the end of the text were in one long line
        assert_eq!(lc("ab").line_of(9), 0);
        let abc = "a\nb\nc";
        assert_eq!(lc(abc).line_of(abc.len()+1), 2);
        assert_eq!(lc(abc).line_of(abc.len()+100), 2);
        let abcn = "a\nb\nc\n";
        assert_eq!(lc(abc).line_of(abcn.len()+1), 2);
        assert_eq!(lc(abc).line_of(abcn.len()+100), 2);
    }
    #[test] fn lc_same_line_is_ok() {
        let mut c = lc("a\nb\nc\n");
        assert_eq!(c.line_of(3), 1);
        assert_eq!(c.line_of(2), 1);
        let mut c = lc("\n\n\n");
        assert_eq!(c.line_of(1), 1);
        assert_eq!(c.line_of(1), 1);
    }
    #[cfg(debug_assertions)]
    #[should_panic]
    #[test] fn lc_previous_line_panics() { // in test mode
        let mut c = lc("a\nb\nc\n");
        assert_eq!(c.line_of(3), 1);
        assert_eq!(c.line_of(2), 1);
        assert_eq!(c.line_of(0), 1);
    }
    // And a column test
    #[test] fn lc_column() {
        let mut c = lc("abc\ndef");
        assert_eq!(c.line_and_column(0), (0,0));
        assert_eq!(c.line_and_column(2), (0,2));
        assert_eq!(c.line_and_column(5), (1,1));
    }
}
```

The `memchr` crate gives us a fast interator over the postions of newlines in
the Markdown text.  It will be useful to pretend that there is an infinite
supply of newlines, infinitely far from the end of our text. (Where "infinitely
far" means "at position `usize::max_value()`".)

```rust
use memchr::Memchr;

struct Newlines<'a>(Memchr<'a>);
impl<'a> Newlines<'a> {
    fn new(text: &'a str) -> Newlines<'a> {
        Newlines(Memchr::new(b'\n', text.as_bytes()))
    }
    fn next(&mut self) -> usize {
        match self.0.next() {
            Some(n) => n,
            None => usize::max_value(),
        }
    }
}
```
So if a series of `next()` calls on the inner `Memchr` returns

```rust,ignore
    Some(i), Some(j), Some(k), None, None, None ...
```

then the equivalent series of `next()` calls on our `Newlines` object returns

```rust,ignore
    i, j, k, usize::max_value(), usize::max_value(), usize::max_value(), ...
```

We'll be using `Newlines` to find the next newline at or after a given offset,
incrementing the line number each time we call `next()`.  We'll also need fields
to hold information on the current line: the line number, the postion of the
line's (real or virtual) newline, and the postion of the first character of the
line. We initialize things as they would be after a call to `line_of(0)`.

```rust
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Line {pub start: usize, pub end: usize, pub number: usize}

pub (crate) struct LineCounter<'a> {
    newlines: Newlines<'a>,
    number: usize,
    start: usize,
    end: usize,
}

impl<'a> LineCounter<'a> {
    pub fn new(text: &'a str) -> LineCounter<'a> {
        let mut newlines = Newlines::new(text);
        let end = newlines.next();
        LineCounter{newlines, number: 0, start: 0, end}
    }
    pub fn line_of(&mut self, offset: usize) -> usize {
        self.setup(offset);
        self.number
    }
    pub fn line_and_column(&mut self, offset: usize) -> (usize, usize) {
        self.setup(offset);
        (self.number, offset - self.start)
    }
    ⟨Define `fn setup`⟩
}
```

Each line counter maintains these invariants:

- `end` is the end of the current line: the position of the first
  newline at or after the last `offset` argument to `setup()` (or simply the
  first newline, for the line counter value returned by `new()`);
- `start` is the first character of the previous line.
- `number` is the count of newlines whose postion is strictly less than
  `end`.

Calling `newlines.next()` gives us the offset of the next newline in `text`. So
to find the line number of the `offset` parameter, we keep calling
`newline.next()` until it returns a number at least as great as `offset`.

```rust
⟨Define `fn setup`⟩≡
    pub fn setup(&mut self, offset: usize) {
        debug_assert!(self.start <= offset);
        while self.end < offset {
            self.number += 1;
            self.start = self.end+1;
            self.end = self.newlines.next();
        }
    }
```

And here are a few tests of the column function:

```rust

