//! Stores a [Token's](super::Token) lexical information
use std::{fmt::Display, str::FromStr};

/// Stores a [Token's](super::Token) lexical information
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    // Invalid syntax
    Invalid,
    // Any kind of comment
    Comment,
    // Any identifier
    Identifier,
    Keyword(Keyword),
    // Literals
    Integer,
    Float,
    String,
    Character,
    // Delimiters and punctuation
    LCurly,    // {
    RCurly,    // }
    LBrack,    // [
    RBrack,    // ]
    LParen,    // (
    RParen,    // )
    Amp,       // &
    AmpAmp,    // &&
    AmpEq,     // &=
    Arrow,     // ->
    At,        // @
    Backslash, // \
    Bang,      // !
    BangBang,  // !!
    BangEq,    // !=
    Bar,       // |
    BarBar,    // ||
    BarEq,     // |=
    Colon,     // :
    Comma,     // ,
    Dot,       // .
    DotDot,    // ..
    DotDotEq,  // ..=
    Eq,        // =
    EqEq,      // ==
    FatArrow,  // =>
    Grave,     // `
    Gt,        // >
    GtEq,      // >=
    GtGt,      // >>
    GtGtEq,    // >>=
    Hash,      // #
    Lt,        // <
    LtEq,      // <=
    LtLt,      // <<
    LtLtEq,    // <<=
    Minus,     // -
    MinusEq,   // -=
    Plus,      // +
    PlusEq,    // +=
    Question,  // ?
    Rem,       // %
    RemEq,     // %=
    Semi,      // ;
    Slash,     // /
    SlashEq,   // /=
    Star,      // *
    StarEq,    // *=
    Tilde,     // ~
    Xor,       // ^
    XorEq,     // ^=
    XorXor,    // ^^
}

/// Represents a reserved word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Keyword {
    Break,
    Continue,
    Else,
    False,
    For,
    Fn,
    If,
    In,
    Let,
    Return,
    True,
    While,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Invalid => "invalid".fmt(f),
            Type::Comment => "comment".fmt(f),
            Type::Identifier => "identifier".fmt(f),
            Type::Keyword(k) => k.fmt(f),
            Type::Integer => "integer literal".fmt(f),
            Type::Float => "float literal".fmt(f),
            Type::String => "string literal".fmt(f),
            Type::Character => "char literal".fmt(f),
            Type::LCurly => "left curly".fmt(f),
            Type::RCurly => "right curly".fmt(f),
            Type::LBrack => "left brack".fmt(f),
            Type::RBrack => "right brack".fmt(f),
            Type::LParen => "left paren".fmt(f),
            Type::RParen => "right paren".fmt(f),
            Type::Amp => "and".fmt(f),
            Type::AmpAmp => "and-and".fmt(f),
            Type::AmpEq => "and-assign".fmt(f),
            Type::Arrow => "arrow".fmt(f),
            Type::At => "at".fmt(f),
            Type::Backslash => "backslash".fmt(f),
            Type::Bang => "bang".fmt(f),
            Type::BangBang => "not-not".fmt(f),
            Type::BangEq => "not equal to".fmt(f),
            Type::Bar => "or".fmt(f),
            Type::BarBar => "or-or".fmt(f),
            Type::BarEq => "or-assign".fmt(f),
            Type::Colon => "colon".fmt(f),
            Type::Comma => "comma".fmt(f),
            Type::Dot => "dot".fmt(f),
            Type::DotDot => "exclusive range".fmt(f),
            Type::DotDotEq => "inclusive range".fmt(f),
            Type::Eq => "assign".fmt(f),
            Type::EqEq => "equal to".fmt(f),
            Type::FatArrow => "fat arrow".fmt(f),
            Type::Grave => "grave".fmt(f),
            Type::Gt => "greater than".fmt(f),
            Type::GtEq => "greater than or equal to".fmt(f),
            Type::GtGt => "shift right".fmt(f),
            Type::GtGtEq => "shift right-assign".fmt(f),
            Type::Hash => "hash".fmt(f),
            Type::Lt => "less than".fmt(f),
            Type::LtEq => "less than or equal to".fmt(f),
            Type::LtLt => "shift left".fmt(f),
            Type::LtLtEq => "shift left-assign".fmt(f),
            Type::Minus => "sub".fmt(f),
            Type::MinusEq => "sub-assign".fmt(f),
            Type::Plus => "add".fmt(f),
            Type::PlusEq => "add-assign".fmt(f),
            Type::Question => "huh?".fmt(f),
            Type::Rem => "rem".fmt(f),
            Type::RemEq => "rem-assign".fmt(f),
            Type::Semi => "ignore".fmt(f),
            Type::Slash => "div".fmt(f),
            Type::SlashEq => "div-assign".fmt(f),
            Type::Star => "star".fmt(f),
            Type::StarEq => "star-assign".fmt(f),
            Type::Tilde => "tilde".fmt(f),
            Type::Xor => "xor".fmt(f),
            Type::XorEq => "xor-assign".fmt(f),
            Type::XorXor => "cat-ears".fmt(f),
        }
    }
}

impl Display for Keyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Break => "break".fmt(f),
            Self::Continue => "continue".fmt(f),
            Self::Else => "else".fmt(f),
            Self::False => "false".fmt(f),
            Self::For => "for".fmt(f),
            Self::Fn => "fn".fmt(f),
            Self::If => "if".fmt(f),
            Self::In => "in".fmt(f),
            Self::Let => "let".fmt(f),
            Self::Return => "return".fmt(f),
            Self::True => "true".fmt(f),
            Self::While => "while".fmt(f),
        }
    }
}
impl FromStr for Keyword {
    /// [FromStr] can only fail when an identifier isn't a keyword
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "break" => Self::Break,
            "continue" => Self::Continue,
            "else" => Self::Else,
            "false" => Self::False,
            "for" => Self::For,
            "fn" => Self::Fn,
            "if" => Self::If,
            "in" => Self::In,
            "let" => Self::Let,
            "return" => Self::Return,
            "true" => Self::True,
            "while" => Self::While,
            _ => Err(())?,
        })
    }
}
