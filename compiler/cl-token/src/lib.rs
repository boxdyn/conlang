//! # Token
//!
//! Stores a component of a file as a [Lexeme], [TKind], and [struct@Span]
#![warn(clippy::all)]
//! The Token defines an interface between lexer and parser

use cl_structures::span::Span;

/// A unit of lexical information produced by a Lexer
#[derive(Clone, Debug)]
pub struct Token {
    pub lexeme: Lexeme,
    pub kind: TKind,
    pub span: Span,
}

impl Token {
    /// Extracts the [`kind`](Token::kind) field of this Token
    pub const fn kind(&self) -> TKind {
        self.kind
    }
}

/// The (possibly pre-processed) lexical information, in the form of a [String], [u128], or [char]
#[derive(Clone, Debug)]
pub enum Lexeme {
    String(String),
    Integer(u128, u32),
    Char(char),
}

impl Lexeme {
    pub fn string(self) -> Option<String> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
    pub fn str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
    pub const fn int(&self) -> Option<u128> {
        match self {
            Self::Integer(i, _) => Some(*i),
            _ => None,
        }
    }
    pub const fn char(&self) -> Option<char> {
        match self {
            Self::Char(c) => Some(*c),
            _ => None,
        }
    }
}

impl std::fmt::Display for Lexeme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(v) => v.fmt(f),
            Self::Integer(v, _) => v.fmt(f),
            Self::Char(v) => v.fmt(f),
        }
    }
}

/// The lexical classification of a [Token].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TKind {
    /// Line or block comment
    Comment,
    /// Inner doc comment: //!.*
    InDoc,
    /// Outer doc comment ///.*
    OutDoc,

    As,
    Break,
    Catch,
    Const,
    Continue,
    Defer,
    Do,
    Else,
    Enum,
    False,
    Fn,
    For,
    If,
    Impl,
    In,
    Let,
    Loop,
    Macro,
    Match,
    Mod,
    Mut,
    Pub,
    Return,
    Static,
    Struct,
    True,
    Try,
    Type,
    Underscore,
    Use,
    While,

    Identifier, // or Keyword
    Character,
    String,
    /// `0(x[0-9A-Fa-f]* | d[0-9]* | o[0-7]* | b[0-1]*) | [1-9][0-9]*`
    Integer,
    /// {
    LCurly,
    /// }
    RCurly,
    /// [
    LBrack,
    /// ]
    RBrack,
    /// (
    LParen,
    /// )
    RParen,
    /// &
    Amp,
    /// &&
    AmpAmp,
    /// &=
    AmpEq,
    /// ->
    Arrow,
    /// @
    At,
    /// \
    Backslash,
    /// !
    Bang,
    /// !!
    BangBang,
    /// !=
    BangEq,
    /// |
    Bar,
    /// ||
    BarBar,
    /// |=
    BarEq,
    /// :
    Colon,
    /// ::
    ColonColon,
    /// ,
    Comma,
    /// $
    Dollar,
    /// .
    Dot,
    /// ..
    DotDot,
    /// ...
    DotDotDot,
    /// ..=
    DotDotEq,
    /// =
    Eq,
    /// ==
    EqEq,
    /// =>
    FatArrow,
    /// \`
    Grave,
    /// >
    Gt,
    /// >=
    GtEq,
    /// >>
    GtGt,
    /// >>=
    GtGtEq,
    /// #
    Hash,
    /// #!
    HashBang,
    /// <
    Lt,
    /// <=
    LtEq,
    /// <<
    LtLt,
    /// <<=
    LtLtEq,
    /// -
    Minus,
    /// -=
    MinusEq,
    /// +
    Plus,
    /// +=
    PlusEq,
    /// ?
    Question,
    /// %
    Rem,
    /// %=
    RemEq,
    /// ;
    Semi,
    /// /
    Slash,
    /// /=
    SlashEq,
    /// *
    Star,
    /// *=
    StarEq,
    /// ~
    Tilde,
    /// ^
    Xor,
    /// ^=
    XorEq,
    /// ^^
    XorXor,
}

impl TKind {
    pub const fn flip(self) -> Self {
        match self {
            Self::LCurly => Self::RCurly,
            Self::RCurly => Self::LCurly,
            Self::LBrack => Self::RBrack,
            Self::RBrack => Self::LBrack,
            Self::LParen => Self::RParen,
            Self::RParen => Self::LParen,
            Self::Gt => Self::Lt,
            Self::GtGt => Self::LtLt,
            Self::LtLt => Self::GtGt,
            Self::Lt => Self::Gt,
            _ => self,
        }
    }

    /// Splits a single [TKind] into two, if possible, or returns the original.
    pub const fn split(self) -> Result<(Self, Self), Self> {
        Ok(match self {
            Self::AmpAmp => (Self::Amp, Self::Amp),
            Self::AmpEq => (Self::Amp, Self::Eq),
            Self::Arrow => (Self::Minus, Self::Gt),
            Self::BangBang => (Self::Bang, Self::Bang),
            Self::BangEq => (Self::Bang, Self::Eq),
            Self::BarBar => (Self::Bar, Self::Bar),
            Self::BarEq => (Self::Bar, Self::Eq),
            Self::ColonColon => (Self::Colon, Self::Colon),
            Self::DotDot => (Self::Dot, Self::Dot),
            Self::DotDotEq => (Self::DotDot, Self::Eq),
            Self::EqEq => (Self::Eq, Self::Eq),
            Self::FatArrow => (Self::Eq, Self::Gt),
            Self::GtEq => (Self::Gt, Self::Eq),
            Self::GtGt => (Self::Gt, Self::Gt),
            Self::GtGtEq => (Self::Gt, Self::GtEq),
            Self::HashBang => (Self::Hash, Self::Bang),
            Self::LtEq => (Self::Lt, Self::Eq),
            Self::LtLt => (Self::Lt, Self::Lt),
            Self::LtLtEq => (Self::Lt, Self::LtEq),
            Self::MinusEq => (Self::Minus, Self::Eq),
            Self::PlusEq => (Self::Plus, Self::Eq),
            Self::RemEq => (Self::Rem, Self::Eq),
            Self::SlashEq => (Self::Slash, Self::Eq),
            Self::StarEq => (Self::Star, Self::Eq),
            Self::XorEq => (Self::Xor, Self::Eq),
            Self::XorXor => (Self::Xor, Self::Xor),
            _ => return Err(self),
        })
    }
}
