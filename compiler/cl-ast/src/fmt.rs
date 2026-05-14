//! The Conlang format extensions

use std::fmt::{Display, Write};

/// The default indentation string. Defaults to four spaces.
const INDENTATION: &str = {
    match option_env!("CONLANG_INDENT") {
        Some(indent) => indent,
        None => "    ",
    }
};

impl<W: Write + ?Sized> FmtAdapter for W {}
pub trait FmtAdapter: Write {
    /// Indents by one level.
    fn indent(&mut self) -> Indent<'_, Self> {
        Indent::new(self, INDENTATION)
    }

    /// Pastes `indent` after each newline.
    fn indent_with<I: Display>(&mut self, indent: I) -> Indent<'_, Self, I> {
        Indent::new(self, indent)
    }

    /// Delimits a section with `open` and `close`.
    fn delimit<O: Display, E: Display>(&mut self, open: O, close: E) -> Delimit<'_, Self, E> {
        Delimit::new(self, open, close)
    }

    /// Delimits a section with `open` and `close`, raising the indent level within.
    fn delimit_indented<O: Display, E: Display>(
        &mut self,
        open: O,
        close: E,
    ) -> DelimitIndent<'_, Self, E> {
        DelimitIndent::new(self, open, close)
    }

    /// Formats bracketed lists of the kind (Item (Comma Item)*)?
    #[inline]
    fn list<Iter: IntoIterator<Item: Display>, Sep: Display>(
        &mut self,
        items: Iter,
        sep: Sep,
    ) -> std::fmt::Result {
        self.list_wrap("", items, sep, "")
    }

    /// Wraps a list in `open` and `close`.
    /// This differs from [`FmtAdapter::delimit`] because it prints nothing
    /// if the list is empty.
    fn list_wrap<Iter: IntoIterator<Item: Display>, Sep: Display, O: Display, E: Display>(
        &mut self,
        open: O,
        items: Iter,
        sep: Sep,
        close: E,
    ) -> std::fmt::Result {
        let mut iter = items.into_iter();
        let Some(item) = iter.next() else {
            return Ok(());
        };

        write!(self, "{open}{item}")?;
        for item in iter {
            write!(self, "{sep}{item}")?;
        }
        write!(self, "{close}")
    }
}

/// Pads text with leading indentation after every newline
pub struct Indent<'f, F: Write + ?Sized, I: Display = &'static str> {
    indent: I,
    needs_indent: bool,
    f: &'f mut F,
}

impl<'f, F: Write + ?Sized, I: Display> Indent<'f, F, I> {
    pub const fn new(f: &'f mut F, indent: I) -> Self {
        Indent { f, needs_indent: false, indent }
    }

    /// Gets mutable access to the inner [Write]-adapter
    pub const fn inner(&mut self) -> &mut F {
        self.f
    }
}

impl<F: Write + ?Sized, I: Display> Write for Indent<'_, F, I> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        for s in s.split_inclusive('\n') {
            if self.needs_indent {
                write!(self.f, "{}", self.indent)?;
            }
            self.needs_indent = s.ends_with('\n');
            self.f.write_str(s)?;
        }
        Ok(())
    }
    fn write_char(&mut self, c: char) -> std::fmt::Result {
        if self.needs_indent {
            write!(self.f, "{}", self.indent)?;
        }
        self.needs_indent = c == '\n';
        self.f.write_char(c)
    }
}

/// Prints delimiters around anything formatted with this. Implies [Indent]
pub struct Delimit<'f, F: Write + ?Sized, E: Display = &'static str> {
    /// The formatter
    pub f: &'f mut F,
    close: E,
}

impl<F: Write + ?Sized, E: Display> Delimit<'_, F, E> {
    /// Gets mutable access to the inner [Write]-adapter
    pub const fn inner(&mut self) -> &mut F {
        self.f
    }
}

impl<'f, F: Write + ?Sized, E: Display> Delimit<'f, F, E> {
    pub fn new<O: Display>(f: &'f mut F, open: O, close: E) -> Self {
        let _ = write!(f, "{open}");
        Self { f, close }
    }
}

impl<F: Write + ?Sized, E: Display> Drop for Delimit<'_, F, E> {
    fn drop(&mut self) {
        let Self { f, close, .. } = self;
        let _ = write!(f, "{close}");
    }
}

impl<F: Write + ?Sized, E: Display> Write for Delimit<'_, F, E> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.f.write_str(s)
    }
}

/// Prints delimiters around anything formatted with this. Implies [Indent]
pub struct DelimitIndent<'f, F: Write + ?Sized, E: Display = &'static str> {
    f: Indent<'f, F>,
    close: E,
}

impl<F: Write + ?Sized, E: Display> DelimitIndent<'_, F, E> {
    /// Gets mutable access to the inner [Write]-adapter
    pub const fn inner(&mut self) -> &mut F {
        self.f.inner()
    }
}

impl<'f, F: Write + ?Sized, E: Display> DelimitIndent<'f, F, E> {
    pub fn new<O: Display>(f: &'f mut F, open: O, close: E) -> Self {
        let mut f = f.indent();
        let _ = write!(f, "{open}");
        Self { f, close }
    }
}

impl<F: Write + ?Sized, E: Display> Drop for DelimitIndent<'_, F, E> {
    fn drop(&mut self) {
        let Self { f: Indent { f, .. }, close, .. } = self;
        let _ = write!(f, "{close}");
    }
}

impl<F: Write + ?Sized, E: Display> Write for DelimitIndent<'_, F, E> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.f.write_str(s)
    }
}
