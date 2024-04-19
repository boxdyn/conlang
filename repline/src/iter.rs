//! Shmancy iterator adapters

pub use chars::Chars;
pub use flatten::Flatten;

pub mod chars {
    //! Converts an <code>[Iterator]<Item = [u8]></code> into an
    //! <code>[Iterator]<Item = [Result]<[char], [BadUnicode]>></code>

    /// Invalid unicode codepoint found when iterating over [Chars]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct BadUnicode(pub u32);
    impl std::error::Error for BadUnicode {}
    impl std::fmt::Display for BadUnicode {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self(code) = self;
            write!(f, "Bad unicode: {code}")
        }
    }

    /// Converts an <code>[Iterator]<Item = [u8]></code> into an
    /// <code>[Iterator]<Item = [char]></code>
    #[derive(Clone, Debug)]
    pub struct Chars<I: Iterator<Item = u8>>(pub I);
    impl<I: Iterator<Item = u8>> Iterator for Chars<I> {
        type Item = Result<char, BadUnicode>;
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
            Some(char::from_u32(out).ok_or(BadUnicode(out)))
        }
    }
}
pub mod flatten {
    //! Flattens an [Iterator] returning [`Result<T, E>`](Result) or [`Option<T>`](Option)
    //! into a *non-[FusedIterator](std::iter::FusedIterator)* over `T`

    /// Flattens an [Iterator] returning [`Result<T, E>`](Result) or [`Option<T>`](Option)
    /// into a *non-[FusedIterator](std::iter::FusedIterator)* over `T`
    #[derive(Clone, Debug)]
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
