//! [String] tools for Conlang
//#![warn(clippy::all)]
#![feature(decl_macro, const_trait_impl)]

impl<T: Iterator> ConstrTools for T {}
pub trait ConstrTools {
    /// Unescapes string escape sequences
    fn unescape(self) -> UnescapeString<Self>
    where Self: Iterator<Item = char> + Sized {
        UnescapeString::new(self)
    }
    /// Parse an integer
    fn parse_int<O>(self) -> ParseInt<Self, O>
    where Self: Iterator<Item = char> + Sized {
        ParseInt::new(self)
    }
}

pub use unescape_string::UnescapeString;
pub mod unescape_string {
    //! TODO: Write the module-level documentation
    pub struct UnescapeString<I: Iterator<Item = char>> {
        inner: I,
    }

    impl<I: Iterator<Item = char>> Iterator for UnescapeString<I> {
        type Item = I::Item;
        fn next(&mut self) -> Option<Self::Item> {
            self.unescape()
        }
    }

    impl<I: Iterator<Item = char>> UnescapeString<I> {
        pub fn new(inner: I) -> Self {
            Self { inner }
        }
        /// Consumes an escape sequence. See the [module level documentation](self).
        pub fn unescape(&mut self) -> Option<char> {
            match self.inner.next()? {
                '\\' => (),
                other => return Some(other),
            }
            Some(match self.inner.next()? {
                'a' => '\x07',
                'b' => '\x08',
                'f' => '\x0c',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                'x' => self.hex_digits::<2>()?,
                'u' => self.unicode()?,
                '0' => '\0',
                byte => byte,
            })
        }
        fn unicode(&mut self) -> Option<char> {
            let mut out = 0;
            let Some('{') = self.inner.next() else {
                return None;
            };
            for c in self.inner.by_ref() {
                match c {
                    '}' => return char::from_u32(out),
                    _ => out = (out << 4) + super::base::<16>(c)? as u32,
                }
            }
            None
        }
        fn hex_digits<const DIGITS: u32>(&mut self) -> Option<char> {
            let mut out = 0;
            for _ in 0..DIGITS {
                out = (out << 4) + self.hex_digit()? as u32;
            }
            char::from_u32(out)
        }
        fn hex_digit(&mut self) -> Option<u8> {
            super::base::<16>(self.inner.next()?)
        }
    }
}
pub use parse_int::ParseInt;
pub mod parse_int {
    use std::marker::PhantomData;

    pub struct ParseInt<I: Iterator<Item = char>, O> {
        inner: I,
        _data: PhantomData<O>,
    }
    impl<I: Iterator<Item = char>, O> ParseInt<I, O> {
        pub fn new(inner: I) -> Self {
            Self { inner, _data: Default::default() }
        }
        fn digit<const B: u8>(&mut self) -> Option<u8> {
            let next = loop {
                match self.inner.next()? {
                    '_' => continue,
                    c => break c,
                }
            };
            super::base::<B>(next)
        }
    }
    parse_int_impl!(u8, i8, u16, i16, u32, i32, u64, i64, u128, i128);
    macro parse_int_impl($($T:ty),*$(,)?) {$(
        impl<I: Iterator<Item = char>> ParseInt<I, $T> {
            fn digits<const B: u8>(&mut self, init: Option<u8>) -> Option<$T> {
                let mut out = match init {
                    Some(digit) => digit,
                    None => self.digit::<B>()?,
                } as $T;
                while let Some(digit) = self.digit::<B>() {
                    out = out.checked_mul(B as $T)?.checked_add(digit as $T)?
                }
                Some(out)
            }
            fn base(&mut self) -> Option<$T> {
                match self.inner.next()? {
                    'b' => self.digits::<2>(None),
                    'd' => self.digits::<10>(None),
                    'o' => self.digits::<8>(None),
                    'x' => self.digits::<16>(None),
                    c => self.digits::<10>(Some(super::base::<10>(c)?)),
                }
            }
        }
        impl<I: Iterator<Item = char>> Iterator for ParseInt<I, $T> {
            type Item = $T;
            fn next(&mut self) -> Option<Self::Item> {
                match self.digit::<10>()? {
                    0 => self.base(),
                    c if (0..=9).contains(&c) => self.digits::<10>(Some(c)),
                    _ => None,
                }
            }
        }
    )*}
}

/// Converts a single char [0-9A-Za-z] to their [base B](base::<B>) equivalent.
///
/// # May Panic
/// Panics in debug mode when B > 36
pub const fn base<const B: u8>(c: char) -> Option<u8> {
    // TODO: Wait for a way to limit const generics at compile time
    debug_assert!(B <= 36);
    // Can't use Ord::min in const context yet :(
    // This function also relies on wrapping arithmetic
    macro wrap ($c:ident - $b:literal $(+ $ten:literal)? $(< $B:ident.min($min:literal))?) {
        $c.wrapping_sub($b)$(.wrapping_add($ten))? $(< if $B < $min {$B} else {$min})?
    }
    let c = c as u8;
    match c {
        c if wrap!(c - b'0' < B.min(10)) => Some(wrap!(c - b'0')),
        _ if B <= 10 => None, // cuts base<1..=10> to 4 instructions on x86 :^)
        c if wrap!(c - b'A' + 10 < B.min(36)) => Some(wrap!(c - b'A' + 10)),
        c if wrap!(c - b'a' + 10 < B.min(36)) => Some(wrap!(c - b'a' + 10)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod unescape_string {
        use super::*;
        test_unescape! {
            empty = ["" => ""];
            n_newline = ["\\n" => "\n", "This is a\\ntest" => "This is a\ntest"];
            a_bell = ["\\a" => "\x07", "Ring the \\abell" => "Ring the \x07bell"];
            b_backspace = ["\\b" => "\x08"];
            f_feed = ["\\f" => "\x0c"];
            r_return = ["\\r" => "\r"];
            t_tab = ["\\t" => "\t"];
            _0_nul = ["\\0" => "\0"];
            x_hex = [
                "\\x41\\x41\\x41\\x41" => "AAAA",
                "\x00" => "\0",
                "\\x7f" => "\x7f",
                "\\x80" => "\u{80}",
                "\\xD0" => "\u{D0}",
            ];
            u_unicode = [
                "\\u{41}" => "A",
                "\\u{1f988}" => "🦈",
            ];
        }
        macro test_unescape ($($f:ident = [$($test:expr => $expect:expr),*$(,)?];)*) {$(
            #[test] fn $f () {
                $(assert_eq!($test.chars().unescape().collect::<String>(), dbg!($expect));)*
            }
        )*}
    }
    mod parse_int {
        use super::*;
        #[test]
        #[should_panic]
        fn base_37_panics() {
            base::<37>('a');
        }
        test_parse! {
            parse_u8: u8 = [
                "0xc5" => Some(0xc5),
                "0xc_____________________5" => Some(0xc5),
                "0x7d" => Some(0x7d),
                "0b10" => Some(0b10),
                "0o10" => Some(0o10),
                "0x10" => Some(0x10),
                "0d10" => Some(10),
                "10" => Some(10),
            ];
            parse_u16: u16 = [
                "0xc5c5" => Some(0xc5c5),
                "0x1234" => Some(0x1234),
                "0x5678" => Some(0x5678),
                "0x9abc" => Some(0x9abc),
                "0xdef0" => Some(0xdef0),
                "0xg" => None,
                "0b10" => Some(0b10),
                "0o10" => Some(0o10),
                "0x10" => Some(0x10),
                "0d10" => Some(10),
                "10" => Some(10),
            ];
            parse_u32: u32 = [
                "0xc5c5c5c5" => Some(0xc5c5c5c5),
                "0xc5_c5_c5_c5" => Some(0xc5c5c5c5),
                "1_234_567____" => Some(1234567),
                "4294967295" => Some(4294967295),
                "4294967296" => None,
                "🦈" => None,
            ];
            parse_u64: u64 = [
                "0xffffffffffffffff" => Some(0xffffffffffffffff),
                "0x10000000000000000" => None,
                "0xc5c5c5c5c5c5c5c5" => Some(0xc5c5c5c5c5c5c5c5),
                "0x123456789abcdef0" => Some(1311768467463790320),
                "0x123456789abcdefg" => Some(81985529216486895),
                "0d1234567890" => Some(1234567890),
                "0o12345670" => Some(2739128),
                "0b10" => Some(2),
            ];
            parse_u128: u128 = [
                "0x10000000000000000" => Some(0x10000000000000000),
                "0xc5c5c5c5c5c5c5c5c5c5c5c5c5c5c5c5" => Some(0xc5c5c5c5c5c5c5c5c5c5c5c5c5c5c5c5),
                "0o77777777777777777777777777777777" => Some(0o77777777777777777777777777777777),
            ];
        }
        macro test_parse ($($f:ident : $T:ty = [$($test:expr => $expect:expr),*$(,)?];)*) {$(
        #[test] fn $f () {
            type Test = $T;
            $(assert_eq!(($test.chars().parse_int() as ParseInt<_, Test>).next(), dbg!($expect));)*
        }
    )*}
    }
}
