//! A lobster
use std::{iter::Peekable, ops::Range, str::CharIndices};
use unicode_ident::{is_xid_continue, is_xid_start};

use cl_structures::span::Span;
use cl_token::*;

pub use cl_structures::intern::interned::Symbol;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LexError {
    pub pos: Span,
    pub res: LexFailure,
}

impl std::error::Error for LexError {}
impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { pos, res } = self;
        write!(f, "{pos}: {res}")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LexFailure {
    /// Reached end of file
    EOF,
    UnexpectedEOF,
    Unexpected(char),
    UnterminatedBlockComment,
    UnterminatedCharacter,
    UnterminatedString,
    UnterminatedUnicodeEscape,
    InvalidUnicodeEscape(u32),
    InvalidDigitForBase(char, u32),
    IntegerOverflow,
}
use LexFailure::*;
pub use LexFailure::{EOF, UnexpectedEOF};

impl std::fmt::Display for LexFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EOF => "EOF".fmt(f),
            Self::UnexpectedEOF => "Unexpected EOF".fmt(f),
            Self::Unexpected(c) => write!(f, "Character {c:?}"),
            Self::UnterminatedBlockComment => "Unterminated Block Comment".fmt(f),
            Self::UnterminatedCharacter => "Unterminated Character".fmt(f),
            Self::UnterminatedString => "Unterminated String".fmt(f),
            Self::UnterminatedUnicodeEscape => "Unterminated Unicode Escape".fmt(f),
            Self::InvalidUnicodeEscape(hex) => {
                write!(f, "'\\u{{{hex:x}}}' is not a valid UTF-8 codepoint")
            }
            Self::InvalidDigitForBase(digit, base) => {
                write!(f, "Invalid digit {digit} for base {base}")
            }
            Self::IntegerOverflow => "Integer literal does not fit in 128 bits".fmt(f),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Lexer<'t> {
    path: Symbol,
    /// The source text
    text: &'t str,
    /// A peekable iterator over the source text
    iter: Peekable<CharIndices<'t>>,
    /// The start of the current token
    head: u32,
    /// The end of the current token
    tail: u32,
}

impl<'t> Lexer<'t> {
    /// Constructs a new Lexer from some text
    pub fn new(path: Symbol, text: &'t str) -> Self {
        let iter = text.char_indices().peekable();
        Self { path, text, iter, head: 0, tail: 0 }
    }

    /// Gets the [Span] of the current token.
    ///
    /// When called from outside [Lexer::scan], this will return
    /// a zero-sized span marking the current lexer location.
    pub const fn span(&self) -> Span {
        Span(self.path, self.head, self.tail)
    }

    /// Peeks the next character without advancing the lexer
    pub fn peek(&mut self) -> Option<char> {
        self.iter.peek().map(|&(_, c)| c)
    }

    /// Advances the tail to the current character index
    fn advance_tail(&mut self) {
        match self.iter.peek() {
            Some(&(idx, _)) => self.tail = idx as u32,
            None => self.tail = self.text.len() as _,
        }
    }

    /// Takes the last character
    fn take(&mut self) -> Option<char> {
        let (_, c) = self.iter.next()?;
        self.advance_tail();
        Some(c)
    }

    fn next_if(&mut self, expected: char) -> Option<char> {
        let (_, c) = self.iter.next_if(|&(_, c)| c == expected)?;
        self.advance_tail();
        Some(c)
    }

    /// Consumes the last-peeked character, advancing the tail
    fn consume(&mut self) -> &mut Self {
        self.iter.next();
        self.advance_tail();
        self
    }

    /// Produces a [`LexError`] at the start of the current token
    const fn error(&self, res: LexFailure) -> LexError {
        LexError { pos: self.span(), res }
    }

    /// Gets the Lexer's current &[str] lexeme and [Span]
    fn as_str(&self) -> (&'t str, Span) {
        let span = self.span();
        (&self.text[Range::from(span)], span)
    }

    /// Produces a Token
    fn produce(&mut self, kind: TKind) -> Token {
        self.advance_tail();
        let (lexeme, span) = self.as_str();
        self.head = self.tail;
        Token { lexeme: Lexeme::String(lexeme.to_owned()), kind, span }
    }

    fn produce_with_lexeme(&mut self, kind: TKind, lexeme: Lexeme) -> Token {
        self.advance_tail();
        let span = self.span();
        self.head = self.tail;
        Token { lexeme, kind, span }
    }

    /// Consumes 0 or more whitespace
    fn skip_whitespace(&mut self) -> &mut Self {
        while self.peek().is_some_and(char::is_whitespace) {
            let _ = self.consume();
        }
        self
    }

    const fn start_token(&mut self) -> &mut Self {
        self.head = self.tail;
        self
    }

    /// Scans forward until it finds the next Token in the input
    pub fn scan(&mut self) -> Result<Token, LexError> {
        use TKind::*;
        // !"#%&'()*+,-./:;<=>?@[\\]^`{|}~
        let tok = match self
            .skip_whitespace()
            .start_token()
            .peek()
            .ok_or_else(|| self.error(EOF))?
        {
            '!' => Bang,
            '"' => return self.string(),
            '#' => Hash,
            '$' => Dollar,
            '%' => Rem,
            '&' => Amp,
            '\'' => return self.character(),
            '(' => LParen,
            ')' => RParen,
            '*' => Star,
            '+' => Plus,
            ',' => return self.consume().trailing(Comma),
            '-' => Minus,
            '.' => Dot,
            '/' => Slash,
            '0' => Integer,
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
            c => Err(self.error(Unexpected(c)))?,
        };

        // Handle digraphs
        let tok = match (tok, self.consume().peek()) {
            (Integer, Some('b')) => return self.consume().digits::<2>(),
            (Integer, Some('d')) => return self.consume().digits::<10>(),
            (Integer, Some('o')) => return self.consume().digits::<8>(),
            (Integer, Some('x')) => return self.consume().digits::<16>(),
            (Integer, Some('~')) => return self.consume().digits::<36>(),
            (Integer, _) => return self.digits::<10>(),
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
            (Slash, Some('*')) => return Ok(self.block_comment()?.produce(Comment)),
            (Slash, Some('=')) => SlashEq,
            (Slash, Some('/')) => return self.line_comment(),
            (Star, Some('=')) => StarEq,
            (Xor, Some('=')) => XorEq,
            (Xor, Some('^')) => XorXor,
            _ => return Ok(self.produce(tok)),
        };

        // Handle trigraphs
        let tok = match (tok, self.consume().peek()) {
            (HashBang, Some('/')) => return self.line_comment(),
            (DotDot, Some('.')) => DotDotDot,
            (DotDot, Some('=')) => DotDotEq,
            (GtGt, Some('=')) => GtGtEq,
            (LtLt, Some('=')) => LtLtEq,
            _ => return Ok(self.produce(tok)),
        };

        Ok(self.consume().produce(tok))
    }

    /// Elides the trailing [Token] `kind` when it comes before a list terminator.
    pub fn trailing(&mut self, kind: TKind) -> Result<Token, LexError> {
        Ok(match self.skip_whitespace().peek() {
            // Some(')') => self.consume().produce(TKind::RParen), // maybe.
            // Some(']') => self.consume().produce(TKind::RBrack),
            Some('}') => self.consume().produce(TKind::RCurly),
            _ => self.produce(kind),
        })
    }

    /// Consumes characters until the lexer reaches a newline `'\n'`
    pub fn line_comment(&mut self) -> Result<Token, LexError> {
        let kind = match self.consume().peek() {
            Some('/') => TKind::OutDoc,
            Some('!') => TKind::InDoc,
            _ => TKind::Comment,
        };
        while self.consume().peek().is_some_and(|c| c != '\n') {}
        let (lexeme, _) = self.as_str();
        let lexeme = lexeme
            .strip_prefix("///")
            .or_else(|| lexeme.strip_prefix("//!"))
            .map(|lexeme| lexeme.strip_prefix(" ").unwrap_or(lexeme))
            .unwrap_or(lexeme);

        Ok(self.produce_with_lexeme(kind, Lexeme::String(lexeme.into())))
    }

    /// Consumes characters until the lexer reaches the end of a *nested* block comment.
    /// This allows you to arbitrarily comment out code, even if that code has a block comment.
    pub fn block_comment(&mut self) -> Result<&mut Self, LexError> {
        self.consume();
        while let Some(c) = self.take() {
            match (c, self.peek()) {
                ('/', Some('*')) => self.block_comment()?,
                ('*', Some('/')) => return Ok(self.consume()),
                _ => continue,
            };
        }
        Err(self.error(UnterminatedBlockComment))
    }

    /// Consumes characters until it reaches a character not in [`is_xid_continue`].
    ///
    /// Always consumes the first character.
    ///
    /// Maps the result to either a [`TKind::Identifier`] or a [`TKind`] keyword.
    pub fn identifier(&mut self) -> Result<Token, LexError> {
        while self.consume().peek().is_some_and(is_xid_continue) {}
        let (lexeme, _span) = self.as_str();
        let token = self.produce(TKind::Identifier);
        Ok(Token {
            kind: match lexeme {
                "as" => TKind::As,
                "break" => TKind::Break,
                "const" => TKind::Const,
                "continue" => TKind::Continue,
                "do" => TKind::Do,
                "else" => TKind::Else,
                "enum" => TKind::Enum,
                "false" => TKind::False,
                "fn" => TKind::Fn,
                "for" => TKind::For,
                "if" => TKind::If,
                "impl" => TKind::Impl,
                "in" => TKind::In,
                "let" => TKind::Let,
                "loop" => TKind::Loop,
                "macro" => TKind::Macro,
                "match" => TKind::Match,
                "mod" => TKind::Mod,
                "mut" => TKind::Mut,
                "pub" => TKind::Pub,
                "return" => TKind::Return,
                "static" => TKind::Static,
                "struct" => TKind::Struct,
                "then" => TKind::Do,
                "true" => TKind::True,
                "type" => TKind::Type,
                "use" => TKind::Use,
                "while" => TKind::While,
                _ => token.kind,
            },
            ..token
        })
    }

    /// Eagerly parses a character literal starting at the current lexer position.
    pub fn character(&mut self) -> Result<Token, LexError> {
        let c = match self.consume().take() {
            Some('\\') => self.escape()?,
            Some(c) => c,
            None => '\0',
        };
        if self.take().is_some_and(|c| c == '\'') {
            Ok(self.produce_with_lexeme(TKind::Character, Lexeme::Char(c)))
        } else {
            Err(self.error(UnterminatedCharacter))
        }
    }

    // Eagerly parses a string literal starting at the current lexer position.
    pub fn string(&mut self) -> Result<Token, LexError> {
        let mut lexeme = String::new();
        self.consume();
        loop {
            lexeme.push(match self.take() {
                None => Err(self.error(UnterminatedString))?,
                Some('\\') => self.escape()?,
                Some('"') => break,
                Some(c) => c,
            });
        }
        lexeme.shrink_to_fit();
        Ok(self.produce_with_lexeme(TKind::String, Lexeme::String(lexeme)))
    }

    /// Parses a single escape sequence into its resulting char value.
    pub fn escape(&mut self) -> Result<char, LexError> {
        Ok(
            match self.take().ok_or_else(|| self.error(UnexpectedEOF))? {
                ' ' => '\u{a0}', // Non-breaking space
                '0' => '\0',     // C0 Null Character
                'a' => '\x07',   // C0 Acknowledge
                'b' => '\x08',   // C0 Bell
                'e' => '\x1b',   // C0 Escape
                'f' => '\x0c',   // Form Feed
                'n' => '\n',     // New Line
                'r' => '\r',     // Carriage Return
                't' => '\t',     // Tab
                'u' => self.unicode_escape()?,
                'x' => self.hex_escape()?,
                c => c,
            },
        )
    }

    /// Parses two hex-digits and constructs a [char] out of them.
    pub fn hex_escape(&mut self) -> Result<char, LexError> {
        let out = (self.digit::<16>()? << 4) + self.digit::<16>()?;
        char::from_u32(out).ok_or_else(|| self.error(InvalidUnicodeEscape(out)))
    }

    /// Parses a sequence of `{}`-bracketed hex-digits and constructs a [char] out of them.
    pub fn unicode_escape(&mut self) -> Result<char, LexError> {
        self.next_if('{')
            .ok_or_else(|| self.error(UnterminatedUnicodeEscape))?;
        let mut out = 0;
        while let Some(c) = self.take() {
            if c == '}' {
                return char::from_u32(out).ok_or_else(|| self.error(InvalidUnicodeEscape(out)));
            }
            out = out.saturating_mul(16).saturating_add(
                c.to_digit(16)
                    .ok_or_else(|| self.error(InvalidDigitForBase(c, 16)))?,
            );
        }
        Err(self.error(UnterminatedUnicodeEscape))
    }

    /// Parses a sequence of digits (and underscores) in base `BASE`, where 2 <= `BASE` <= 36.
    ///
    /// If the sequence of digits exceeds the bounds of a [u128], the resulting number will wrap
    /// around 2^128.
    pub fn digits<const BASE: u32>(&mut self) -> Result<Token, LexError> {
        let mut int: u128 = 0;
        while let Some(c) = self.peek() {
            int = match c.to_digit(BASE).ok_or(c) {
                Err('_') => int,
                Ok(c) => int
                    .checked_mul(BASE as _)
                    .and_then(|int| int.checked_add(c as _))
                    .ok_or_else(|| self.error(IntegerOverflow))?,
                _ => break,
            };
            self.consume();
        }

        Ok(self.produce_with_lexeme(TKind::Integer, Lexeme::Integer(int, BASE)))
    }

    /// Parses a single digit in base `BASE` as a u32, where 2 <= `BASE` <= 36.
    pub fn digit<const BASE: u32>(&mut self) -> Result<u32, LexError> {
        let digit = self.take().ok_or_else(|| self.error(UnexpectedEOF))?;
        if let Some(digit) = digit.to_digit(BASE) {
            Ok(digit)
        } else {
            Err(self.error(InvalidDigitForBase(digit, BASE)))
        }
    }
}
