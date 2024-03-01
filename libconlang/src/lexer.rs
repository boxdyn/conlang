//! Converts a text file into tokens
use cl_token::*;
use cl_structures::span::Loc;
use std::{
    iter::Peekable,
    str::{Chars, FromStr},
};
use unicode_xid::UnicodeXID;

pub mod lexer_iter {
    //! Iterator over a [`Lexer`], returning [`LResult<Token>`]s
    use super::{
        error::{LResult, Reason},
        Lexer, Token,
    };

    /// Iterator over a [`Lexer`], returning [`LResult<Token>`]s
    pub struct LexerIter<'t> {
        lexer: Lexer<'t>,
    }
    impl<'t> Iterator for LexerIter<'t> {
        type Item = LResult<Token>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.lexer.scan() {
                Ok(v) => Some(Ok(v)),
                Err(e) => {
                    if e.reason == Reason::EndOfFile {
                        None
                    } else {
                        Some(Err(e))
                    }
                }
            }
        }
    }
    impl<'t> IntoIterator for Lexer<'t> {
        type Item = LResult<Token>;
        type IntoIter = LexerIter<'t>;
        fn into_iter(self) -> Self::IntoIter {
            LexerIter { lexer: self }
        }
    }
}

/// The Lexer iterates over the characters in a body of text, searching for [Tokens](Token).
///
/// # Examples
/// ```rust
/// # use conlang::lexer::Lexer;
/// // Read in your code from somewhere
/// let some_code = "
/// fn main () {
///     // TODO: code goes here!
/// }
/// ";
/// // Create a lexer over your code
/// let mut lexer = Lexer::new(some_code);
/// // Scan for a single token
/// let first_token = lexer.scan().unwrap();
/// println!("{first_token:?}");
/// // Loop over all the rest of the tokens
/// for token in lexer {
/// #   let token: Result<_,()> = Ok(token.unwrap());
///     match token {
///         Ok(token) => println!("{token:?}"),
///         Err(e) => eprintln!("{e:?}"),
///     }
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Lexer<'t> {
    iter: Peekable<Chars<'t>>,
    start: usize,
    start_loc: (u32, u32),
    current: usize,
    current_loc: (u32, u32),
}

impl<'t> Lexer<'t> {
    /// Creates a new [Lexer] over a [str]
    pub fn new(text: &'t str) -> Self {
        Self {
            iter: text.chars().peekable(),
            start: 0,
            start_loc: (1, 1),
            current: 0,
            current_loc: (1, 1),
        }
    }
    /// Scans through the text, searching for the next [Token]
    pub fn scan(&mut self) -> LResult<Token> {
        match self.skip_whitespace().peek()? {
            '{' => self.consume()?.produce(Type::LCurly, ()),
            '}' => self.consume()?.produce(Type::RCurly, ()),
            '[' => self.consume()?.produce(Type::LBrack, ()),
            ']' => self.consume()?.produce(Type::RBrack, ()),
            '(' => self.consume()?.produce(Type::LParen, ()),
            ')' => self.consume()?.produce(Type::RParen, ()),
            '&' => self.consume()?.amp(),
            '@' => self.consume()?.produce(Type::At, ()),
            '\\' => self.consume()?.produce(Type::Backslash, ()),
            '!' => self.consume()?.bang(),
            '|' => self.consume()?.bar(),
            ':' => self.consume()?.colon(),
            ',' => self.consume()?.produce(Type::Comma, ()),
            '.' => self.consume()?.dot(),
            '=' => self.consume()?.equal(),
            '`' => self.consume()?.produce(Type::Grave, ()),
            '>' => self.consume()?.greater(),
            '#' => self.consume()?.hash(),
            '<' => self.consume()?.less(),
            '-' => self.consume()?.minus(),
            '+' => self.consume()?.plus(),
            '?' => self.consume()?.produce(Type::Question, ()),
            '%' => self.consume()?.rem(),
            ';' => self.consume()?.produce(Type::Semi, ()),
            '/' => self.consume()?.slash(),
            '*' => self.consume()?.star(),
            '~' => self.consume()?.produce(Type::Tilde, ()),
            '^' => self.consume()?.xor(),
            '0' => self.consume()?.int_with_base(),
            '1'..='9' => self.digits::<10>(),
            '"' => self.consume()?.string(),
            '\'' => self.consume()?.character(),
            '_' => self.identifier(),
            i if i.is_xid_start() => self.identifier(),
            e => {
                let err = Err(Error::unexpected_char(e, self.line(), self.col()));
                let _ = self.consume();
                err
            }
        }
    }
    /// Returns the current line
    pub fn line(&self) -> u32 {
        self.start_loc.0
    }
    /// Returns the current column
    pub fn col(&self) -> u32 {
        self.start_loc.1
    }
    fn next(&mut self) -> LResult<char> {
        let out = self.peek();
        self.consume()?;
        out
    }
    fn peek(&mut self) -> LResult<char> {
        self.iter
            .peek()
            .copied()
            .ok_or(Error::end_of_file(self.line(), self.col()))
    }
    fn produce(&mut self, ty: Type, data: impl Into<Data>) -> LResult<Token> {
        let loc = self.start_loc;
        self.start_loc = self.current_loc;
        self.start = self.current;
        Ok(Token::new(ty, data, loc.0, loc.1))
    }
    fn skip_whitespace(&mut self) -> &mut Self {
        while let Ok(c) = self.peek() {
            if !c.is_whitespace() {
                break;
            }
            let _ = self.consume();
        }
        self.start = self.current;
        self.start_loc = self.current_loc;
        self
    }
    fn consume(&mut self) -> LResult<&mut Self> {
        self.current += 1;
        match self.iter.next() {
            Some('\n') => {
                let (line, col) = &mut self.current_loc;
                *line += 1;
                *col = 1;
            }
            Some(_) => self.current_loc.1 += 1,
            None => Err(Error::end_of_file(self.line(), self.col()))?,
        }
        Ok(self)
    }
}
/// Digraphs and trigraphs
impl<'t> Lexer<'t> {
    fn amp(&mut self) -> LResult<Token> {
        match self.peek() {
            Ok('&') => self.consume()?.produce(Type::AmpAmp, ()),
            Ok('=') => self.consume()?.produce(Type::AmpEq, ()),
            _ => self.produce(Type::Amp, ()),
        }
    }
    fn bang(&mut self) -> LResult<Token> {
        match self.peek() {
            Ok('!') => self.consume()?.produce(Type::BangBang, ()),
            Ok('=') => self.consume()?.produce(Type::BangEq, ()),
            _ => self.produce(Type::Bang, ()),
        }
    }
    fn bar(&mut self) -> LResult<Token> {
        match self.peek() {
            Ok('|') => self.consume()?.produce(Type::BarBar, ()),
            Ok('=') => self.consume()?.produce(Type::BarEq, ()),
            _ => self.produce(Type::Bar, ()),
        }
    }
    fn colon(&mut self) -> LResult<Token> {
        match self.peek() {
            Ok(':') => self.consume()?.produce(Type::ColonColon, ()),
            _ => self.produce(Type::Colon, ()),
        }
    }
    fn dot(&mut self) -> LResult<Token> {
        match self.peek() {
            Ok('.') => {
                if let Ok('=') = self.consume()?.peek() {
                    self.consume()?.produce(Type::DotDotEq, ())
                } else {
                    self.produce(Type::DotDot, ())
                }
            }
            _ => self.produce(Type::Dot, ()),
        }
    }
    fn equal(&mut self) -> LResult<Token> {
        match self.peek() {
            Ok('=') => self.consume()?.produce(Type::EqEq, ()),
            Ok('>') => self.consume()?.produce(Type::FatArrow, ()),
            _ => self.produce(Type::Eq, ()),
        }
    }
    fn greater(&mut self) -> LResult<Token> {
        match self.peek() {
            Ok('=') => self.consume()?.produce(Type::GtEq, ()),
            Ok('>') => {
                if let Ok('=') = self.consume()?.peek() {
                    self.consume()?.produce(Type::GtGtEq, ())
                } else {
                    self.produce(Type::GtGt, ())
                }
            }
            _ => self.produce(Type::Gt, ()),
        }
    }
    fn hash(&mut self) -> LResult<Token> {
        match self.peek() {
            Ok('!') => self.consume()?.produce(Type::HashBang, ()),
            _ => self.produce(Type::Hash, ()),
        }
    }
    fn less(&mut self) -> LResult<Token> {
        match self.peek() {
            Ok('=') => self.consume()?.produce(Type::LtEq, ()),
            Ok('<') => {
                if let Ok('=') = self.consume()?.peek() {
                    self.consume()?.produce(Type::LtLtEq, ())
                } else {
                    self.produce(Type::LtLt, ())
                }
            }
            _ => self.produce(Type::Lt, ()),
        }
    }
    fn minus(&mut self) -> LResult<Token> {
        match self.peek() {
            Ok('=') => self.consume()?.produce(Type::MinusEq, ()),
            Ok('>') => self.consume()?.produce(Type::Arrow, ()),
            _ => self.produce(Type::Minus, ()),
        }
    }
    fn plus(&mut self) -> LResult<Token> {
        match self.peek() {
            Ok('=') => self.consume()?.produce(Type::PlusEq, ()),
            _ => self.produce(Type::Plus, ()),
        }
    }
    fn rem(&mut self) -> LResult<Token> {
        match self.peek() {
            Ok('=') => self.consume()?.produce(Type::RemEq, ()),
            _ => self.produce(Type::Rem, ()),
        }
    }
    fn slash(&mut self) -> LResult<Token> {
        match self.peek() {
            Ok('=') => self.consume()?.produce(Type::SlashEq, ()),
            Ok('/') => self.consume()?.line_comment(),
            Ok('*') => self.consume()?.block_comment(),
            _ => self.produce(Type::Slash, ()),
        }
    }
    fn star(&mut self) -> LResult<Token> {
        match self.peek() {
            Ok('=') => self.consume()?.produce(Type::StarEq, ()),
            _ => self.produce(Type::Star, ()),
        }
    }
    fn xor(&mut self) -> LResult<Token> {
        match self.peek() {
            Ok('=') => self.consume()?.produce(Type::XorEq, ()),
            Ok('^') => self.consume()?.produce(Type::XorXor, ()),
            _ => self.produce(Type::Xor, ()),
        }
    }
}
/// Comments
impl<'t> Lexer<'t> {
    fn line_comment(&mut self) -> LResult<Token> {
        while Ok('\n') != self.peek() {
            self.consume()?;
        }
        self.produce(Type::Comment, ())
    }
    fn block_comment(&mut self) -> LResult<Token> {
        while let Ok(c) = self.next() {
            if '*' == c && Ok('/') == self.next() {
                break;
            }
        }
        self.produce(Type::Comment, ())
    }
}
/// Identifiers
impl<'t> Lexer<'t> {
    fn identifier(&mut self) -> LResult<Token> {
        let mut out = String::from(self.xid_start()?);
        while let Ok(c) = self.xid_continue() {
            out.push(c)
        }
        if let Ok(keyword) = Keyword::from_str(&out) {
            self.produce(Type::Keyword(keyword), ())
        } else {
            self.produce(Type::Identifier, Data::Identifier(out.into()))
        }
    }
    fn xid_start(&mut self) -> LResult<char> {
        match self.peek()? {
            xid if xid == '_' || xid.is_xid_start() => {
                self.consume()?;
                Ok(xid)
            }
            bad => Err(Error::not_identifier(bad, self.line(), self.col())),
        }
    }
    fn xid_continue(&mut self) -> LResult<char> {
        match self.peek()? {
            xid if xid.is_xid_continue() => {
                self.consume()?;
                Ok(xid)
            }
            bad => Err(Error::not_identifier(bad, self.line(), self.col())),
        }
    }
}
/// Integers
impl<'t> Lexer<'t> {
    fn int_with_base(&mut self) -> LResult<Token> {
        match self.peek() {
            Ok('x') => self.consume()?.digits::<16>(),
            Ok('d') => self.consume()?.digits::<10>(),
            Ok('o') => self.consume()?.digits::<8>(),
            Ok('b') => self.consume()?.digits::<2>(),
            Ok('0'..='9') => self.digits::<10>(),
            _ => self.produce(Type::Integer, 0),
        }
    }
    fn digits<const B: u32>(&mut self) -> LResult<Token> {
        let mut value = self.digit::<B>()? as u128;
        while let Ok(true) = self.peek().as_ref().map(char::is_ascii_alphanumeric) {
            value = value * B as u128 + self.digit::<B>()? as u128;
        }
        self.produce(Type::Integer, value)
    }
    fn digit<const B: u32>(&mut self) -> LResult<u32> {
        let digit = self.peek()?;
        self.consume()?;
        digit
            .to_digit(B)
            .ok_or(Error::invalid_digit(digit, self.line(), self.col()))
    }
}
/// Strings and characters
impl<'t> Lexer<'t> {
    fn string(&mut self) -> LResult<Token> {
        let mut value = String::new();
        while '"'
            != self
                .peek()
                .map_err(|e| e.mask_reason(Reason::UnmatchedDelimiters('"')))?
        {
            value.push(self.unescape()?)
        }
        self.consume()?.produce(Type::String, value)
    }
    fn character(&mut self) -> LResult<Token> {
        let out = self.unescape()?;
        match self.peek()? {
            '\'' => self.consume()?.produce(Type::Character, out),
            _ => Err(Error::unmatched_delimiters('\'', self.line(), self.col())),
        }
    }
    /// Unescape a single character
    fn unescape(&mut self) -> LResult<char> {
        match self.next() {
            Ok('\\') => (),
            other => return other,
        }
        Ok(match self.next()? {
            'a' => '\x07',
            'b' => '\x08',
            'f' => '\x0c',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'x' => self.hex_escape()?,
            'u' => self.unicode_escape()?,
            '0' => '\0',
            chr => chr,
        })
    }
    /// unescape a single 2-digit hex escape
    fn hex_escape(&mut self) -> LResult<char> {
        let out = (self.digit::<16>()? << 4) + self.digit::<16>()?;
        char::from_u32(out).ok_or(Error::bad_unicode(out, self.line(), self.col()))
    }
    /// unescape a single \u{} unicode escape
    fn unicode_escape(&mut self) -> LResult<char> {
        let mut out = 0;
        let Ok('{') = self.peek() else {
            return Err(Error::invalid_escape('u', self.line(), self.col()));
        };
        self.consume()?;
        while let Ok(c) = self.peek() {
            match c {
                '}' => {
                    self.consume()?;
                    return char::from_u32(out).ok_or(Error::bad_unicode(
                        out,
                        self.line(),
                        self.col(),
                    ));
                }
                _ => out = (out << 4) + self.digit::<16>()?,
            }
        }
        Err(Error::invalid_escape('u', self.line(), self.col()))
    }
}

impl<'t> From<&Lexer<'t>> for Loc {
    fn from(value: &Lexer<'t>) -> Self {
        Loc(value.line(), value.col())
    }
}

use error::{Error, LResult, Reason};
pub mod error {
    //! [Error] type for the [Lexer](super::Lexer)
    use std::fmt::Display;

    /// Result type with [Err] = [Error]
    pub type LResult<T> = Result<T, Error>;
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Error {
        pub reason: Reason,
        pub line: u32,
        pub col: u32,
    }
    /// The reason for the [Error]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Reason {
        /// Found an opening delimiter of type [char], but not the expected closing delimiter
        UnmatchedDelimiters(char),
        /// Found a character that doesn't belong to any [Type](crate::token::token_type::Type)
        UnexpectedChar(char),
        /// Found a character that's not valid in identifiers while looking for an identifier
        NotIdentifier(char),
        /// Found a character that's not valid in an escape sequence while looking for an escape
        /// sequence
        UnknownEscape(char),
        /// Escape sequence contains invalid hexadecimal digit or unmatched braces
        InvalidEscape(char),
        /// Character is not a valid digit in the requested base
        InvalidDigit(char),
        /// Base conversion requested, but the base character was not in the set of known
        /// characters
        UnknownBase(char),
        /// Unicode escape does not map to a valid unicode code-point
        BadUnicode(u32),
        /// Reached end of input
        EndOfFile,
    }
    error_impl! {
        unmatched_delimiters(c: char) => Reason::UnmatchedDelimiters(c),
        unexpected_char(c: char) => Reason::UnexpectedChar(c),
        not_identifier(c: char) => Reason::NotIdentifier(c),
        unknown_escape(e: char) => Reason::UnknownEscape(e),
        invalid_escape(e: char) => Reason::InvalidEscape(e),
        invalid_digit(digit: char) => Reason::InvalidDigit(digit),
        unknown_base(base: char) => Reason::UnknownBase(base),
        bad_unicode(value: u32) => Reason::BadUnicode(value),
        end_of_file => Reason::EndOfFile,
    }
    impl Error {
        /// Changes the [Reason] of this error
        pub(super) fn mask_reason(self, reason: Reason) -> Self {
            Self { reason, ..self }
        }
        /// Returns the [Reason] for this error
        pub fn reason(&self) -> &Reason {
            &self.reason
        }
        /// Returns the (line, col) where the error happened
        pub fn location(&self) -> (u32, u32) {
            (self.line, self.col)
        }
    }
    macro error_impl ($($fn:ident$(( $($p:ident: $t:ty),* ))? => $reason:expr),*$(,)?) {
        #[allow(dead_code)]
        impl Error {
            $(pub(super) fn $fn ($($($p: $t),*,)? line: u32, col: u32) -> Self {
                Self { reason: $reason, line, col }
            })*
        }
    }
    impl std::error::Error for Error {}
    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}:{}: {}", self.line, self.col, self.reason)
        }
    }
    impl Display for Reason {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Reason::UnmatchedDelimiters(c) => write! {f, "Unmatched `{c}` in input"},
                Reason::UnexpectedChar(c) => write!(f, "Character `{c}` not expected"),
                Reason::NotIdentifier(c) => write!(f, "Character `{c}` not valid in identifiers"),
                Reason::UnknownEscape(c) => write!(f, "`\\{c}` is not a known escape sequence"),
                Reason::InvalidEscape(c) => write!(f, "Escape sequence `\\{c}`... is malformed"),
                Reason::InvalidDigit(c) => write!(f, "`{c}` is not a valid digit"),
                Reason::UnknownBase(c) => write!(f, "`0{c}`... is not a valid base"),
                Reason::BadUnicode(c) => write!(f, "`{c}` is not a valid unicode code-point"),
                Reason::EndOfFile => write!(f, "Reached end of input"),
            }
        }
    }
}
