use super::pat::Prec as PatPrec;
use cl_lexer::LexError;
use cl_structures::span::Span;
use cl_token::TKind;
use std::{error::Error, fmt::Display};

/// All the ways a [Parser](crate::parser::Parser) can fail
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// Reached the expected end of input.
    EOF(Span),
    /// *Unexpectedly* reached end of input.
    UnexpectedEOF(Span),
    /// Did not reach end of enput when expected.
    ExpectedEOF(TKind, Span),
    /// The [`Lexer`](cl_lexer::Lexer) didn't like that.
    FromLexer(LexError),
    /// Expected [`TKind`] `0`, got [`TKind`] `1` at [`Span`]
    Expected(TKind, TKind, Span),
    /// Tried to parse a literal, but got [`TKind`] instead, at [`Span`]
    NotLiteral(TKind, Span),
    /// Tried to parse a use item, but got [`TKind`] instead, at [`Span`]
    NotUse(TKind, Span),
    /// Tried to parse a pattern, but got [`TKind`] at [`PatPrec`] instead, at [`Span`]
    NotPattern(TKind, PatPrec, Span),
    /// Tried to parse a bind item, but got [`TKind`] instead, at [`Span`]
    NotBind(TKind, Span),
    /// Tried to parse a prefix expression, but got [`TKind`] instead, at [`Span`]
    NotPrefix(TKind, Span),
    /// Tried to parse an infix expression, but got [`TKind`] instead, at [`Span`]
    NotInfix(TKind, Span),
    /// Tried to parse a postfix expression, but got [`TKind`] instead, at [`Span`]
    NotPostfix(TKind, Span),
}

pub use ParseError::EOF;

impl Error for ParseError {}
impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EOF(loc) => write!(f, "{loc}: Reached end of input."),
            Self::UnexpectedEOF(loc) => write!(f, "{loc}: Unexpected end of input."),
            Self::ExpectedEOF(tk, loc) => write!(f, "{loc}: Expected end of input, got {tk:?}."),
            Self::FromLexer(e) => e.fmt(f),
            Self::Expected(e, tk, loc) => write!(f, "{loc}: Expected {e:?}, got {tk:?}."),
            Self::NotLiteral(tk, loc) => write!(f, "{loc}: {tk:?} is not valid in a literal."),
            Self::NotUse(tk, loc) => write!(f, "{loc}: {tk:?} is no use!"),
            Self::NotPattern(tk, prec, loc) => {
                write!(f, "{loc}: {tk:?} is not valid in a {prec:?} pattern.")
            }
            Self::NotBind(bind, loc) => {
                write!(f, "{loc}: {bind:?} is not valid in a bind expression.")
            }
            Self::NotPrefix(tk, loc) => write!(f, "{loc}: {tk:?} is not a prefix operator."),
            Self::NotInfix(tk, loc) => write!(f, "{loc}: {tk:?} is not a infix operator."),
            Self::NotPostfix(tk, loc) => write!(f, "{loc}: {tk:?} is not a postfix operator."),
        }
    }
}

pub type PResult<T> = Result<T, ParseError>;

pub trait PResultExt<T> {
    fn no_eof(self) -> PResult<T>;
    fn allow_eof(self) -> PResult<Option<T>>;
    fn is_eof(&self) -> bool;
}

impl<T> PResultExt<T> for PResult<T> {
    fn no_eof(self) -> Self {
        match self {
            Err(ParseError::EOF(span)) => Err(ParseError::UnexpectedEOF(span)),
            other => other,
        }
    }
    fn allow_eof(self) -> PResult<Option<T>> {
        match self {
            Ok(t) => Ok(Some(t)),
            Err(ParseError::EOF(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }
    fn is_eof(&self) -> bool {
        matches!(self, Err(ParseError::EOF(_)))
    }
}

/// Opens a scope where [`ParseError::EOF`] is unexpected (See [`PResultExt::no_eof`])
pub fn no_eof<T>(f: impl FnOnce() -> PResult<T>) -> PResult<T> {
    f().no_eof()
}
