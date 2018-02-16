Find Line Numbers Given Character Offsets
=========================================

The job of a `LineCounter` is to return the line number of a given character
offset `n`, via the `line_of()` method.  We define the line of `n` as the number
of newline characters in `&text[0..n]`. (So the first line of the file is line 0.)

For efficiency and simplicity we require that our the inputs to `line_of()` be
non-decreasing.  That is, for a given `LineCounter` `lc`, once we call
`lc.line_of(n)`, in all following calls `lc.line_of(x)`, we must have `n ≤ x`.
(In optimize builds we'll just return a wrong answer if `x < n`; in
non-optimized builds we'll use `debug_assert` to panic.)

We count the terminating newline as part of each line, and don't require a
newline at the very end of the text.

Here are tests that say those things:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    fn lc(text: &str) -> LineCounter { LineCounter::new(text) }

    #[test] fn first_is_0()  { // The first line number is 0
        assert_eq!(lc("abc\nx").line_of(2), 0);
    }
    #[test] fn incl_newline() { // A line includes its terminating newline character
        assert_eq!(lc("a\n").line_of(1), 0);
        // ... even if it's the first character of the text:
        assert_eq!(lc("\na\n").line_of(0), 0);
    }
    #[test] fn begins_immediately() {
        // Next line begins immediately after the newline of the previous line
        assert_eq!(lc("a\nb").line_of(2), 1);
    }
    #[test] fn no_last_newline() { // We don't require a newline at the end of text:
        assert_eq!(lc("ab").line_of(2), 0);
    }
    #[test] fn virtual_last() {
        // We act as if offsets beyond the end of the text were in one long line
        assert_eq!(lc("ab").line_of(9), 0);
        let abc = "a\nb\nc";
        assert_eq!(lc(abc).line_of(abc.len()+1), 2);
        assert_eq!(lc(abc).line_of(abc.len()+100), 2);
        let abcn = "a\nb\nc\n";
        assert_eq!(lc(abc).line_of(abcn.len()+1), 2);
        assert_eq!(lc(abc).line_of(abcn.len()+100), 2);
    }
    #[should_panic]
    #[test] fn non_decreasing() { // The `line_of()` method is sticky
        let mut c = lc("a\nb\nc\n");
        c.line_of(0); c.line_of(4); c.line_of(0); 
    }
}
```

The `memchr` crate gives us a fast interator over the postions of newlines in
the Markdown text.  It will be useful to pretend that there is an infinite
supply of newlines, infinitely far from the end of our text. (Where "infinitely
far" means "at position `usize::MAX`".)

```rust
extern crate memchr;
use self::memchr::Memchr;

struct Newlines<'a>(Memchr<'a>);
impl<'a> Newlines<'a> {
    fn new(text: &'a str) -> Newlines<'a> {
        Newlines(Memchr::new(b'\n', text.as_bytes()))
    }
    fn next(&mut self) -> usize {
        match self.0.next() {
            Some(n) => n,
            None => ::std::usize::MAX,
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
    i, j, k, usize:MAX, usize:MAX, usize:MAX, ...
```

We'll also need a `Line` struct to hold information on the last line found so
far: `mid` records the last offset passed to `line_of()`, `end` the end of
`mid`'s line, and `number` the line number returned by `line_of()`.  We
initialize things as they would be after a call to `line_of(0)`.

```rust
#[derive(Debug)]
struct Line {mid: usize, end: usize, number: usize}

pub (crate) struct LineCounter<'a> {
    newlines: Newlines<'a>,
    current: Line,
}
```

We'll be using `Newlines` to find the next newline at or after a given offset,
incrementing the line number each time we call `next()`.  We store the current
line end so we know whether or not to call `next()` again for a given offset,
and the current line number so we can return or update it.

```rust
impl<'a> LineCounter<'a> {
    pub fn new(text: &'a str) -> LineCounter<'a> {
        let mut newlines = Newlines::new(text);
        let end = newlines.next();
        LineCounter{newlines, current: Line {mid: 0, end, number: 0}}
    }
    pub fn line_of(&mut self, offset: usize) -> usize {
        debug_assert!(self.current.mid <= offset);
        self.current.mid = offset;
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

