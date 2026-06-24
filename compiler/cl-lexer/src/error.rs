use cl_structures::span::Span;

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
