//! Converts a text file into tokens
#![warn(clippy::all)]
#![feature(decl_macro)]
use cl_structures::span::Loc;
use cl_token::{TokenKind as Kind, *};
use std::{
    iter::Peekable,
    str::{CharIndices, FromStr},
};
use unicode_ident::*;

#[cfg(test)]
mod tests;

pub mod lexer_iter {
    //! Iterator over a [`Lexer`], returning [`LResult<Token>`]s
    use super::{
        Lexer, Token,
        error::{LResult, Reason},
    };

    /// Iterator over a [`Lexer`], returning [`LResult<Token>`]s
    pub struct LexerIter<'t> {
        lexer: Lexer<'t>,
    }
    impl Iterator for LexerIter<'_> {
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
/// # use cl_lexer::Lexer;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Read in your code from somewhere
/// let some_code = "
/// fn main () {
///     // TODO: code goes here!
/// }
/// ";
/// // Create a lexer over your code
/// let mut lexer = Lexer::new(some_code);
/// // Scan for a single token
/// let first_token = lexer.scan()?;
/// println!("{first_token:?}");
/// // Loop over all the rest of the tokens
/// for token in lexer {
/// #   let token: Result<_,()> = Ok(token?);
///     match token {
///         Ok(token) => println!("{token:?}"),
///         Err(e) => eprintln!("{e:?}"),
///     }
/// }
/// # Ok(()) }
/// ```
#[derive(Clone, Debug)]
pub struct Lexer<'t> {
    /// The source text
    text: &'t str,
    /// A peekable iterator over the source text
    iter: Peekable<CharIndices<'t>>,
    /// The end of the current token
    head: usize,
    /// The (line, col) end of the current token
    head_loc: (u32, u32),
    /// The start of the current token
    tail: usize,
    /// The (line, col) start of the current token
    tail_loc: (u32, u32),
}

impl<'t> Lexer<'t> {
    /// Creates a new [Lexer] over a [str]
    pub fn new(text: &'t str) -> Self {
        Self {
            text,
            iter: text.char_indices().peekable(),
            head: 0,
            head_loc: (1, 1),
            tail: 0,
            tail_loc: (1, 1),
        }
    }

    /// Returns the current line
    pub fn line(&self) -> u32 {
        self.tail_loc.0
    }

    /// Returns the current column
    pub fn col(&self) -> u32 {
        self.tail_loc.1
    }

    /// Returns the current token's lexeme
    fn lexeme(&mut self) -> &'t str {
        &self.text[self.tail..self.head]
    }

    /// Peeks the next character without advancing the lexer
    fn peek(&mut self) -> Option<char> {
        self.iter.peek().map(|(_, c)| *c)
    }

    /// Advances the 'tail' (current position)
    fn advance_tail(&mut self) {
        let (idx, c) = self.iter.peek().copied().unwrap_or((self.text.len(), '\0'));
        let (line, col) = &mut self.head_loc;
        let diff = idx - self.head;

        self.head = idx;
        match c {
            '\n' => {
                *line += 1;
                *col = 1;
            }
            _ => *col += diff as u32,
        }
    }

    /// Takes the last-peeked character, or the next character if none peeked.
    pub fn take(&mut self) -> Option<char> {
        let (_, c) = self.iter.next()?;
        self.advance_tail();
        Some(c)
    }

    /// Takes the next char if it matches the `expected` char
    pub fn next_if(&mut self, expected: char) -> Option<char> {
        let (_, c) = self.iter.next_if(|&(_, c)| c == expected)?;
        self.advance_tail();
        Some(c)
    }

    /// Consumes the last-peeked character, advancing the tail
    pub fn consume(&mut self) -> &mut Self {
        self.iter.next();
        self.advance_tail();
        self
    }

    /// Produces an [Error] at the start of the current token
    fn error(&self, reason: Reason) -> Error {
        Error { reason, line: self.line(), col: self.col() }
    }

    /// Produces a token with the current [lexeme](Lexer::lexeme) as its data
    fn produce(&mut self, kind: Kind) -> LResult<Token> {
        let lexeme = self.lexeme().to_owned();
        self.produce_with(kind, lexeme)
    }

    /// Produces a token with the provided `data`
    fn produce_with(&mut self, kind: Kind, data: impl Into<TokenData>) -> LResult<Token> {
        let loc = self.tail_loc;
        self.tail_loc = self.head_loc;
        self.tail = self.head;
        Ok(Token::new(kind, data, loc.0, loc.1))
    }

    /// Produces a token with no `data`
    fn produce_op(&mut self, kind: Kind) -> LResult<Token> {
        self.produce_with(kind, ())
    }

    /// Consumes 0 or more whitespace
    fn skip_whitespace(&mut self) -> &mut Self {
        while self.peek().is_some_and(char::is_whitespace) {
            let _ = self.consume();
        }
        self
    }

    /// Starts a new token
    fn start_token(&mut self) -> &mut Self {
        self.tail_loc = self.head_loc;
        self.tail = self.head;
        self
    }

    /// Scans through the text, searching for the next [Token]
    pub fn scan(&mut self) -> LResult<Token> {
        use TokenKind::*;
        // !"#%&'()*+,-./:;<=>?@[\\]^`{|}~
        let tok = match self
            .skip_whitespace()
            .start_token()
            .peek()
            .ok_or_else(|| self.error(Reason::EndOfFile))?
        {
            '!' => Bang,
            '"' => return self.string(),
            '#' => Hash,
            '%' => Rem,
            '&' => Amp,
            '\'' => return self.character(),
            '(' => LParen,
            ')' => RParen,
            '*' => Star,
            '+' => Plus,
            ',' => Comma,
            '-' => Minus,
            '.' => Dot,
            '/' => Slash,
            '0' => TokenKind::Literal,
            '1'..='9' => return self.digits::<10>(),
            ':' => Colon,
            ';' => Semi,
            '<' => Lt,
            '=' => Eq,
            '>' => Gt,
            '?' => Question,
            '@' => At,
            '[' => LBrack,
            '\\' => Backslash,
            ']' => RBrack,
            '^' => Xor,
            '`' => Grave,
            '{' => LCurly,
            '|' => Bar,
            '}' => RCurly,
            '~' => Tilde,
            '_' => return self.identifier(),
            c if is_xid_start(c) => return self.identifier(),
            e => {
                let err = Err(self.error(Reason::UnexpectedChar(e)));
                let _ = self.consume();
                err?
            }
        };

        // Handle digraphs
        let tok = match (tok, self.consume().peek()) {
            (Literal, Some('b')) => return self.consume().digits::<2>(),
            (Literal, Some('d')) => return self.consume().digits::<10>(),
            (Literal, Some('o')) => return self.consume().digits::<8>(),
            (Literal, Some('x')) => return self.consume().digits::<16>(),
            (Literal, Some('~')) => return self.consume().digits::<36>(),
            (Literal, _) => return self.digits::<10>(),
            (Amp, Some('&')) => AmpAmp,
            (Amp, Some('=')) => AmpEq,
            (Bang, Some('!')) => BangBang,
            (Bang, Some('=')) => BangEq,
            (Bar, Some('|')) => BarBar,
            (Bar, Some('=')) => BarEq,
            (Colon, Some(':')) => ColonColon,
            (Dot, Some('.')) => DotDot,
            (Eq, Some('=')) => EqEq,
            (Eq, Some('>')) => FatArrow,
            (Gt, Some('=')) => GtEq,
            (Gt, Some('>')) => GtGt,
            (Hash, Some('!')) => HashBang,
            (Lt, Some('=')) => LtEq,
            (Lt, Some('<')) => LtLt,
            (Minus, Some('=')) => MinusEq,
            (Minus, Some('>')) => Arrow,
            (Plus, Some('=')) => PlusEq,
            (Rem, Some('=')) => RemEq,
            (Slash, Some('*')) => return self.block_comment()?.produce(Kind::Comment),
            (Slash, Some('/')) => return self.line_comment(),
            (Slash, Some('=')) => SlashEq,
            (Star, Some('=')) => StarEq,
            (Xor, Some('=')) => XorEq,
            (Xor, Some('^')) => XorXor,
            _ => return self.produce_op(tok),
        };

        // Handle trigraphs
        let tok = match (tok, self.consume().peek()) {
            (HashBang, Some('/')) => return self.line_comment(),
            (DotDot, Some('=')) => DotDotEq,
            (GtGt, Some('=')) => GtGtEq,
            (LtLt, Some('=')) => LtLtEq,
            _ => return self.produce_op(tok),
        };

        self.consume().produce_op(tok)
    }
}

/// Comments
impl Lexer<'_> {
    /// Consumes until the next newline '\n', producing a [Comment](Kind::Comment)
    fn line_comment(&mut self) -> LResult<Token> {
        while self.consume().peek().is_some_and(|c| c != '\n') {}
        self.produce(Kind::Comment)
    }

    /// Consumes nested block-comments. Does not produce by itself.
    fn block_comment(&mut self) -> LResult<&mut Self> {
        self.consume();
        while let Some(c) = self.take() {
            match (c, self.peek()) {
                ('/', Some('*')) => self.block_comment()?,
                ('*', Some('/')) => return Ok(self.consume()),
                _ => continue,
            };
        }
        Err(self.error(Reason::UnmatchedDelimiters('/')))
    }
}

/// Identifiers
impl Lexer<'_> {
    /// Produces an [Identifier](Kind::Identifier) or keyword
    fn identifier(&mut self) -> LResult<Token> {
        while self.consume().peek().is_some_and(is_xid_continue) {}
        if let Ok(keyword) = Kind::from_str(self.lexeme()) {
            self.produce_with(keyword, ())
        } else {
            self.produce(Kind::Identifier)
        }
    }
}

/// Integers
impl Lexer<'_> {
    /// Produces a [Literal](Kind::Literal) with an integer or float value.
    fn digits<const B: u32>(&mut self) -> LResult<Token> {
        let mut value = 0;
        while let Some(true) = self.peek().as_ref().map(char::is_ascii_alphanumeric) {
            value = value * B as u128 + self.digit::<B>()? as u128;
        }
        // TODO: find a better way to handle floats in the tokenizer
        match self.peek() {
            Some('.') => {
                // FIXME: hack: 0.. is not [0.0, '.']
                if let Some('.') = self.clone().consume().take() {
                    return self.produce_with(Kind::Literal, value);
                }
                let mut float = format!("{value}.");
                self.consume();
                while let Some(true) = self.peek().as_ref().map(char::is_ascii_digit) {
                    float.push(self.iter.next().map(|(_, c)| c).unwrap_or_default());
                }
                let float = f64::from_str(&float).expect("must be parsable as float");
                self.produce_with(Kind::Literal, float)
            }
            _ => self.produce_with(Kind::Literal, value),
        }
    }

    /// Consumes a single digit of base [B](Lexer::digit)
    fn digit<const B: u32>(&mut self) -> LResult<u32> {
        let digit = self.take().ok_or_else(|| self.error(Reason::EndOfFile))?;
        digit
            .to_digit(B)
            .ok_or_else(|| self.error(Reason::InvalidDigit(digit)))
    }
}

/// Strings and characters
impl Lexer<'_> {
    /// Produces a [Literal](Kind::Literal) with a pre-escaped [String]
    pub fn string(&mut self) -> Result<Token, Error> {
        let mut lexeme = String::new();
        self.consume();
        loop {
            lexeme.push(match self.take() {
                None => Err(self.error(Reason::UnmatchedDelimiters('"')))?,
                Some('\\') => self.unescape()?,
                Some('"') => break,
                Some(c) => c,
            })
        }
        lexeme.shrink_to_fit();
        self.produce_with(Kind::Literal, lexeme)
    }

    /// Produces a [Literal](Kind::Literal) with a pre-escaped [char]
    fn character(&mut self) -> Result<Token, Error> {
        let c = match self.consume().take() {
            Some('\\') => self.unescape()?,
            Some(c) => c,
            None => '\0',
        };
        if self.take().is_some_and(|c| c == '\'') {
            self.produce_with(Kind::Literal, c)
        } else {
            Err(self.error(Reason::UnmatchedDelimiters('\'')))
        }
    }

    /// Unescapes a single character
    #[rustfmt::skip]
    fn unescape(&mut self) -> LResult<char> {
        Ok(match self.take().ok_or_else(|| self.error(Reason::EndOfFile))? {
            ' ' => '\u{a0}',
            '0' => '\0',
            'a' => '\x07',
            'b' => '\x08',
            'e' => '\x1b',
            'f' => '\x0c',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'u' => self.unicode_escape()?,
            'x' => self.hex_escape()?,
            chr => chr,
        })
    }
    /// Unescapes a single 2-digit hex escape
    fn hex_escape(&mut self) -> LResult<char> {
        let out = (self.digit::<16>()? << 4) + self.digit::<16>()?;
        char::from_u32(out).ok_or_else(|| self.error(Reason::BadUnicode(out)))
    }

    /// Unescapes a single \u{} unicode escape
    pub fn unicode_escape(&mut self) -> Result<char, Error> {
        self.next_if('{')
            .ok_or_else(|| self.error(Reason::InvalidEscape('u')))?;
        let mut out = 0;
        while let Some(c) = self.take() {
            if c == '}' {
                return char::from_u32(out).ok_or_else(|| self.error(Reason::BadUnicode(out)));
            }
            out = out * 16
                + c.to_digit(16)
                    .ok_or_else(|| self.error(Reason::InvalidDigit(c)))?;
        }
        Err(self.error(Reason::UnmatchedDelimiters('}')))
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
        /// Found a character that doesn't belong to any [TokenKind](cl_token::TokenKind)
        UnexpectedChar(char),
        /// Found a character that's not valid in an escape sequence while looking for an escape
        /// sequence
        UnknownEscape(char),
        /// Escape sequence contains invalid hexadecimal digit or unmatched braces
        InvalidEscape(char),
        /// Character is not a valid digit in the requested base
        InvalidDigit(char),
        /// Unicode escape does not map to a valid unicode code-point
        BadUnicode(u32),
        /// Reached end of input
        EndOfFile,
    }
    impl Error {
        /// Returns the [Reason] for this error
        pub fn reason(&self) -> &Reason {
            &self.reason
        }
        /// Returns the (line, col) where the error happened
        pub fn location(&self) -> (u32, u32) {
            (self.line, self.col)
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
                Reason::UnmatchedDelimiters(c) => write! {f, "Unmatched `{c:?}` in input"},
                Reason::UnexpectedChar(c) => write!(f, "Character `{c:?}` not expected"),
                Reason::UnknownEscape(c) => write!(f, "`\\{c}` is not a known escape sequence"),
                Reason::InvalidEscape(c) => write!(f, "Escape sequence `\\{c}`... is malformed"),
                Reason::InvalidDigit(c) => write!(f, "`{c:?}` is not a valid digit"),
                Reason::BadUnicode(c) => write!(f, "`\\u{{{c:x}}}` is not valid unicode"),
                Reason::EndOfFile => write!(f, "Reached end of input"),
            }
        }
    }
}
