//! A small pseudo-multiline editing library
// #![allow(unused)]

pub mod error {
    /// Result type for Repline
    pub type ReplResult<T> = std::result::Result<T, Error>;
    /// Borrowed error (does not implement [Error](std::error::Error)!)
    #[derive(Debug)]
    pub enum Error {
        /// User broke with Ctrl+C
        CtrlC(String),
        /// User broke with Ctrl+D
        CtrlD(String),
        /// Invalid unicode codepoint
        BadUnicode(u32),
        /// Error came from [std::io]
        IoFailure(std::io::Error),
        /// End of input
        EndOfInput,
    }

    impl std::error::Error for Error {}
    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::CtrlC(_) => write!(f, "Ctrl+C"),
                Error::CtrlD(_) => write!(f, "Ctrl+D"),
                Error::BadUnicode(u) => write!(f, "0x{u:x} is not a valid unicode codepoint"),
                Error::IoFailure(s) => write!(f, "{s}"),
                Error::EndOfInput => write!(f, "End of input"),
            }
        }
    }
    impl From<std::io::Error> for Error {
        fn from(value: std::io::Error) -> Self {
            Self::IoFailure(value)
        }
    }
}

pub mod ignore {
    //! Does nothing, universally.
    //!
    //! Introduces the [Ignore] trait, and its singular function, [ignore](Ignore::ignore),
    //! which does nothing.
    impl<T> Ignore for T {}
    /// Does nothing
    ///
    /// # Examples
    /// ```rust
    /// #![deny(unused_must_use)]
    /// # use cl_frontend::repline::ignore::Ignore;
    /// ().ignore();
    /// Err::<(), &str>("Foo").ignore();
    /// Some("Bar").ignore();
    /// 42.ignore();
    ///
    /// #[must_use]
    /// fn the_meaning() -> usize {
    ///     42
    /// }
    /// the_meaning().ignore();
    /// ```
    pub trait Ignore {
        /// Does nothing
        fn ignore(&self) {}
    }
}

pub mod chars {
    //! Converts an <code>[Iterator]<Item = [u8]></code> into an
    //! <code>[Iterator]<Item = [char]></code>

    use super::error::*;

    /// Converts an <code>[Iterator]<Item = [u8]></code> into an
    /// <code>[Iterator]<Item = [char]></code>
    #[derive(Clone, Debug)]
    pub struct Chars<I: Iterator<Item = u8>>(pub I);
    impl<I: Iterator<Item = u8>> Chars<I> {
        pub fn new(bytes: I) -> Self {
            Self(bytes)
        }
    }
    impl<I: Iterator<Item = u8>> Iterator for Chars<I> {
        type Item = ReplResult<char>;
        fn next(&mut self) -> Option<Self::Item> {
            let Self(bytes) = self;
            let start = bytes.next()? as u32;
            let (mut out, count) = match start {
                start if start & 0x80 == 0x00 => (start, 0), // ASCII valid range
                start if start & 0xe0 == 0xc0 => (start & 0x1f, 1), // 1 continuation byte
                start if start & 0xf0 == 0xe0 => (start & 0x0f, 2), // 2 continuation bytes
                start if start & 0xf8 == 0xf0 => (start & 0x07, 3), // 3 continuation bytes
                _ => return None,
            };
            for _ in 0..count {
                let cont = bytes.next()? as u32;
                if cont & 0xc0 != 0x80 {
                    return None;
                }
                out = out << 6 | (cont & 0x3f);
            }
            Some(char::from_u32(out).ok_or(Error::BadUnicode(out)))
        }
    }
}

pub mod flatten {
    //! Flattens an [Iterator] returning [`Result<T, E>`](Result) or [`Option<T>`](Option)
    //! into a *non-[FusedIterator](std::iter::FusedIterator)* over `T`

    /// Flattens an [Iterator] returning [`Result<T, E>`](Result) or [`Option<T>`](Option)
    /// into a *non-[FusedIterator](std::iter::FusedIterator)* over `T`
    pub struct Flatten<T, I: Iterator<Item = T>>(pub I);
    impl<T, E, I: Iterator<Item = Result<T, E>>> Iterator for Flatten<Result<T, E>, I> {
        type Item = T;
        fn next(&mut self) -> Option<Self::Item> {
            self.0.next()?.ok()
        }
    }
    impl<T, I: Iterator<Item = Option<T>>> Iterator for Flatten<Option<T>, I> {
        type Item = T;
        fn next(&mut self) -> Option<Self::Item> {
            self.0.next()?
        }
    }
}

pub mod raw {
    //! Sets the terminal to [`raw`] mode for the duration of the returned object's lifetime.

    /// Sets the terminal to raw mode for the duration of the returned object's lifetime.
    pub fn raw() -> impl Drop {
        Raw::default()
    }
    struct Raw();
    impl Default for Raw {
        fn default() -> Self {
            std::thread::yield_now();
            crossterm::terminal::enable_raw_mode()
                .expect("should be able to transition into raw mode");
            Raw()
        }
    }
    impl Drop for Raw {
        fn drop(&mut self) {
            crossterm::terminal::disable_raw_mode()
                .expect("should be able to transition out of raw mode");
            // std::thread::yield_now();
        }
    }
}

mod out {
    #![allow(unused)]
    use std::io::{Result, Write};

    /// A [Writer](Write) that flushes after every wipe
    #[derive(Clone, Debug)]
    pub(super) struct EagerWriter<W: Write> {
        out: W,
    }
    impl<W: Write> EagerWriter<W> {
        pub fn new(writer: W) -> Self {
            Self { out: writer }
        }
    }
    impl<W: Write> Write for EagerWriter<W> {
        fn write(&mut self, buf: &[u8]) -> Result<usize> {
            let out = self.out.write(buf)?;
            self.out.flush()?;
            Ok(out)
        }
        fn flush(&mut self) -> Result<()> {
            self.out.flush()
        }
    }
}

use self::{chars::Chars, editor::Editor, error::*, flatten::Flatten, ignore::Ignore, raw::raw};
use std::{
    collections::VecDeque,
    io::{stdout, Bytes, Read, Result, Write},
};

pub struct Repline<'a, R: Read> {
    input: Chars<Flatten<Result<u8>, Bytes<R>>>,

    history: VecDeque<String>, // previous lines
    hindex: usize,             // current index into the history buffer

    ed: Editor<'a>, // the current line buffer
}

impl<'a, R: Read> Repline<'a, R> {
    /// Constructs a [Repline] with the given [Reader](Read), color, begin, and again prompts.
    pub fn with_input(input: R, color: &'a str, begin: &'a str, again: &'a str) -> Self {
        Self {
            input: Chars(Flatten(input.bytes())),
            history: Default::default(),
            hindex: 0,
            ed: Editor::new(color, begin, again),
        }
    }
    /// Set the terminal prompt color
    pub fn set_color(&mut self, color: &'a str) {
        self.ed.color = color
    }
    /// Reads in a line, and returns it for validation
    pub fn read(&mut self) -> ReplResult<String> {
        const INDENT: &str = "    ";
        let mut stdout = stdout().lock();
        let _make_raw = raw();
        self.ed.begin_frame(&mut stdout)?;
        self.ed.render(&mut stdout)?;
        loop {
            stdout.flush()?;
            self.ed.begin_frame(&mut stdout)?;
            match self.input.next().ok_or(Error::EndOfInput)?? {
                // Ctrl+C: End of Text. Immediately exits.
                // Ctrl+D: End of Transmission. Ends the current line.
                '\x03' => {
                    self.ed.render(&mut stdout)?;
                    drop(_make_raw);
                    return Err(Error::CtrlC(self.ed.to_string()));
                }
                '\x04' => {
                    self.ed.render(&mut stdout)?; // TODO: this, better
                    drop(_make_raw);
                    return Err(Error::CtrlD(self.ed.to_string()));
                }
                // Tab: extend line by 4 spaces
                '\t' => {
                    self.ed.extend(INDENT.chars());
                }
                // ignore newlines, process line feeds. Not sure how cross-platform this is.
                '\n' => {}
                '\r' => {
                    self.ed.push('\n').ignore();
                    self.ed.render(&mut stdout)?;
                    return Ok(self.ed.to_string());
                }
                // Escape sequence
                '\x1b' => self.escape()?,
                // backspace
                '\x08' | '\x7f' => {
                    let ed = &mut self.ed;
                    if ed.ends_with(INDENT.chars()) {
                        for _ in 0..INDENT.len() {
                            ed.pop().ignore();
                        }
                    } else {
                        self.ed.pop().ignore();
                    }
                }
                c if c.is_ascii_control() => {
                    eprint!("\\x{:02x}", c as u32);
                }
                c => {
                    self.ed.push(c).unwrap();
                }
            }
            self.ed.render(&mut stdout)?;
        }
    }
    /// Handle ANSI Escape
    fn escape(&mut self) -> ReplResult<()> {
        match self.input.next().ok_or(Error::EndOfInput)?? {
            '[' => self.csi()?,
            'O' => todo!("Process alternate character mode"),
            other => self.ed.extend(['\x1b', other]),
        }
        Ok(())
    }
    /// Handle ANSI Control Sequence Introducer
    fn csi(&mut self) -> ReplResult<()> {
        match self.input.next().ok_or(Error::EndOfInput)?? {
            'A' => {
                self.hindex = self.hindex.saturating_sub(1);
                self.restore_history();
            }
            'B' => {
                self.hindex = self
                    .hindex
                    .saturating_add(1)
                    .min(self.history.len().saturating_sub(1));
                self.restore_history();
            }
            'C' => self.ed.cursor_back(1).ignore(),
            'D' => self.ed.cursor_forward(1).ignore(),
            '3' => {
                if let '~' = self.input.next().ok_or(Error::EndOfInput)?? {
                    self.ed.delete().ignore()
                }
            }
            other => {
                eprintln!("{other}");
            }
        }
        Ok(())
    }
    /// Restores the currently selected history
    pub fn restore_history(&mut self) {
        let Self { history, hindex, ed, .. } = self;
        if !(0..history.len()).contains(hindex) {
            return;
        };
        ed.clear();
        ed.extend(
            history
                .get(*hindex)
                .expect("history should contain index")
                .trim_end_matches(|c: char| c != '\n' && c.is_whitespace())
                .chars(),
        );
    }

    /// Append line to history and clear it
    pub fn accept(&mut self) {
        self.history_append(self.ed.iter().collect());
        self.ed.clear();
        self.hindex = self.history.len();
    }
    /// Append line to history
    pub fn history_append(&mut self, buf: String) {
        if !self.history.contains(&buf) {
            self.history.push_back(buf)
        }
        while self.history.len() > 20 {
            self.history.pop_front();
        }
    }
    /// Clear the line
    pub fn deny(&mut self) {
        self.ed.clear()
    }
}

impl<'a> Repline<'a, std::io::Stdin> {
    pub fn new(color: &'a str, begin: &'a str, again: &'a str) -> Self {
        Self::with_input(std::io::stdin(), color, begin, again)
    }
}

pub mod editor {
    use std::{collections::VecDeque, fmt::Display, io::Write};

    use super::error::{Error, ReplResult};

    #[derive(Debug)]
    pub struct Editor<'a> {
        head: VecDeque<char>,
        tail: VecDeque<char>,

        pub color: &'a str,
        begin: &'a str,
        again: &'a str,
    }

    impl<'a> Editor<'a> {
        pub fn new(color: &'a str, begin: &'a str, again: &'a str) -> Self {
            Self { head: Default::default(), tail: Default::default(), color, begin, again }
        }
        pub fn iter(&self) -> impl Iterator<Item = &char> {
            self.head.iter()
        }
        pub fn begin_frame<W: Write>(&self, w: &mut W) -> ReplResult<()> {
            let Self { head, .. } = self;
            match head.iter().filter(|&&c| c == '\n').count() {
                0 => write!(w, "\x1b[0G"),
                lines => write!(w, "\x1b[{}F", lines),
            }?;
            write!(w, "\x1b[0J")?;
            Ok(())
        }
        pub fn render<W: Write>(&self, w: &mut W) -> ReplResult<()> {
            let Self { head, tail, color, begin, again } = self;
            write!(w, "{color}{begin}\x1b[0m ")?;
            // draw head
            for c in head {
                match c {
                    '\n' => write!(w, "\r\n{color}{again}\x1b[0m "),
                    _ => w.write_all({ *c as u32 }.to_le_bytes().as_slice()),
                }?
            }
            // save cursor
            write!(w, "\x1b[s")?;
            // draw tail
            for c in tail {
                match c {
                    '\n' => write!(w, "\r\n{color}{again}\x1b[0m "),
                    _ => write!(w, "{c}"),
                }?
            }
            // restore cursor
            w.write_all(b"\x1b[u").map_err(Into::into)
        }
        pub fn restore(&mut self, s: &str) {
            self.clear();
            self.head.extend(s.chars())
        }
        pub fn clear(&mut self) {
            self.head.clear();
            self.tail.clear();
        }
        pub fn push(&mut self, c: char) -> ReplResult<()> {
            self.head.push_back(c);
            Ok(())
        }
        pub fn pop(&mut self) -> ReplResult<char> {
            self.head.pop_back().ok_or(Error::EndOfInput)
        }
        pub fn delete(&mut self) -> ReplResult<char> {
            self.tail.pop_front().ok_or(Error::EndOfInput)
        }
        pub fn len(&self) -> usize {
            self.head.len() + self.tail.len()
        }
        pub fn is_empty(&self) -> bool {
            self.head.is_empty() && self.tail.is_empty()
        }
        pub fn ends_with(&self, iter: impl DoubleEndedIterator<Item = char>) -> bool {
            let mut iter = iter.rev();
            let mut head = self.head.iter().rev();
            loop {
                match (iter.next(), head.next()) {
                    (None, _) => break true,
                    (Some(_), None) => break false,
                    (Some(a), Some(b)) if a != *b => break false,
                    (Some(_), Some(_)) => continue,
                }
            }
        }
        /// Moves the cursor back `steps` steps
        pub fn cursor_forward(&mut self, steps: usize) -> Option<()> {
            let Self { head, tail, .. } = self;
            for _ in 0..steps {
                tail.push_front(head.pop_back()?);
            }
            Some(())
        }
        /// Moves the cursor forward `steps` steps
        pub fn cursor_back(&mut self, steps: usize) -> Option<()> {
            let Self { head, tail, .. } = self;
            for _ in 0..steps {
                head.push_back(tail.pop_front()?);
            }
            Some(())
        }
    }

    impl<'a> Extend<char> for Editor<'a> {
        fn extend<T: IntoIterator<Item = char>>(&mut self, iter: T) {
            self.head.extend(iter)
        }
    }

    impl<'a, 'e> IntoIterator for &'e Editor<'a> {
        type Item = &'e char;
        type IntoIter = std::iter::Chain<
            std::collections::vec_deque::Iter<'e, char>,
            std::collections::vec_deque::Iter<'e, char>,
        >;
        fn into_iter(self) -> Self::IntoIter {
            self.head.iter().chain(self.tail.iter())
        }
    }
    impl<'a> Display for Editor<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            use std::fmt::Write;
            let Self { head, tail, .. } = self;
            for c in head {
                f.write_char(*c)?;
            }
            for c in tail {
                f.write_char(*c)?;
            }
            Ok(())
        }
    }
}
