
use delimiters::Delimiters;
use std::fmt::Write;

impl<W: Write + ?Sized> FmtAdapter for W {}
pub trait FmtAdapter: Write {
    fn indent(&mut self) -> Indent<Self> {
        Indent { f: self }
    }
    fn delimit(&mut self, delim: Delimiters) -> Delimit<Self> {
        Delimit::new(self, delim)
    }
}

/// Pads text with leading indentation after every newline
pub struct Indent<'f, F: Write + ?Sized> {
    f: &'f mut F,
}

impl<'f, F: Write + ?Sized> Write for Indent<'f, F> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        for s in s.split_inclusive('\n') {
            self.f.write_str(s)?;
            if s.ends_with('\n') {
                self.f.write_str("    ")?;
            }
        }
        Ok(())
    }
}

/// Prints [Delimiters] around anything formatted with this. Implies [Indent]
pub struct Delimit<'f, F: Write + ?Sized> {
    f: Indent<'f, F>,
    delim: Delimiters,
}
impl<'f, F: Write + ?Sized> Delimit<'f, F> {
    pub fn new(f: &'f mut F, delim: Delimiters) -> Self {
        let mut f = f.indent();
        let _ = f.write_str(delim.open);
        Self { f, delim }
    }
}
impl<'f, F: Write + ?Sized> Drop for Delimit<'f, F> {
    fn drop(&mut self) {
        let Self { f: Indent { f, .. }, delim } = self;
        let _ = f.write_str(delim.close);
    }
}

impl<'f, F: Write + ?Sized> Write for Delimit<'f, F> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.f.write_str(s)
    }
}

pub mod delimiters {
    #![allow(dead_code)]
    #[derive(Clone, Copy, Debug)]
    pub struct Delimiters {
        pub open: &'static str,
        pub close: &'static str,
    }
    /// Delimits with braces decorated with spaces  `" {\n"`, ..., `"\n}"`
    pub const SPACED_BRACES: Delimiters = Delimiters { open: " {\n", close: "\n}" };
    /// Delimits with braces on separate lines `{\n`, ..., `\n}`
    pub const BRACES: Delimiters = Delimiters { open: "{\n", close: "\n}" };
    /// Delimits with parentheses on separate lines `{\n`, ..., `\n}`
    pub const PARENS: Delimiters = Delimiters { open: "(\n", close: "\n)" };
    /// Delimits with square brackets on separate lines `{\n`, ..., `\n}`
    pub const SQUARE: Delimiters = Delimiters { open: "[\n", close: "\n]" };
    /// Delimits with braces on the same line `{ `, ..., ` }`
    pub const INLINE_BRACES: Delimiters = Delimiters { open: "{ ", close: " }" };
    /// Delimits with parentheses on the same line `( `, ..., ` )`
    pub const INLINE_PARENS: Delimiters = Delimiters { open: "(", close: ")" };
    /// Delimits with square brackets on the same line `[ `, ..., ` ]`
    pub const INLINE_SQUARE: Delimiters = Delimiters { open: "[", close: "]" };
}
