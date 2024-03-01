use std::{
    fmt::{Result as FmtResult, Write as FmtWrite},
    io::{Result as IoResult, Write as IoWrite},
};

/// Trait which adds a function to [fmt Writers](FmtWrite) to turn them into [Prettifier]
pub trait FmtPretty: FmtWrite {
    /// Indents code according to the number of matched curly braces
    fn pretty(self) -> Prettifier<'static, Self>
    where Self: Sized {
        Prettifier::new(self)
    }
}
/// Trait which adds a function to [io Writers](IoWrite) to turn them into [Prettifier]
pub trait IoPretty: IoWrite {
    /// Indents code according to the number of matched curly braces
    fn pretty(self) -> Prettifier<'static, Self>
    where Self: Sized;
}
impl<W: FmtWrite> FmtPretty for W {}
impl<W: IoWrite> IoPretty for W {
    fn pretty(self) -> Prettifier<'static, Self> {
        Prettifier::new(self)
    }
}

/// Intercepts calls to either [std::io::Write] or [std::fmt::Write],
/// and inserts indentation between matched parentheses
pub struct Prettifier<'i, T: ?Sized> {
    level: isize,
    indent: &'i str,
    writer: T,
}

impl<'i, W> Prettifier<'i, W> {
    pub fn new(writer: W) -> Self {
        Self { level: 0, indent: "    ", writer }
    }
    pub fn with_indent(indent: &'i str, writer: W) -> Self {
        Self { level: 0, indent, writer }
    }
}

impl<'i, W: FmtWrite> Prettifier<'i, W> {
    #[inline]
    fn fmt_write_indentation(&mut self) -> FmtResult {
        let Self { level, indent, writer } = self;
        for _ in 0..*level {
            writer.write_str(indent)?;
        }
        Ok(())
    }
}
impl<'i, W: IoWrite> Prettifier<'i, W> {
    pub fn io_write_indentation(&mut self) -> IoResult<usize> {
        let Self { level, indent, writer } = self;
        let mut count = 0;
        for _ in 0..*level {
            count += writer.write(indent.as_bytes())?;
        }
        Ok(count)
    }
}

impl<'i, W: FmtWrite> FmtWrite for Prettifier<'i, W> {
    fn write_str(&mut self, s: &str) -> FmtResult {
        for s in s.split_inclusive(['{', '}']) {
            match s.as_bytes().last() {
                Some(b'{') => self.level += 1,
                Some(b'}') => self.level -= 1,
                _ => (),
            }
            for s in s.split_inclusive('\n') {
                self.writer.write_str(s)?;
                if let Some(b'\n') = s.as_bytes().last() {
                    self.fmt_write_indentation()?;
                }
            }
        }
        Ok(())
    }
}

impl<'i, W: IoWrite> IoWrite for Prettifier<'i, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut size = 0;
        for buf in buf.split_inclusive(|b| b"{}".contains(b)) {
            match buf.last() {
                Some(b'{') => self.level += 1,
                Some(b'}') => self.level -= 1,
                _ => (),
            }
            for buf in buf.split_inclusive(|b| b'\n' == *b) {
                size += self.writer.write(buf)?;
                if let Some(b'\n') = buf.last() {
                    self.io_write_indentation()?;
                }
            }
        }
        Ok(size)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}
