//! Stores a [Token's](super::Token) lexical information
use std::{fmt::Display, str::FromStr};

/// Stores a [Token's](super::Token) lexical information
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // Invalid syntax
    Invalid,
    // Any kind of comment
    Comment,
    // A non-keyword identifier
    Identifier,
    // A keyword
    Break,
    Cl,
    Const,
    Continue,
    Else,
    Enum,
    False,
    For,
    Fn,
    If,
    Impl,
    In,
    Let,
    Mod,
    Mut,
    Pub,
    Return,
    SelfKw,
    SelfTy,
    Static,
    Struct,
    Super,
    True,
    Type,
    While,
    // Literals
    Integer,
    Float,
    String,
    Character,
    // Delimiters and punctuation
    Op(Op),
}

/// An operator character (delimiter, punctuation)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Op {
    LCurly,     // {
    RCurly,     // }
    LBrack,     // [
    RBrack,     // ]
    LParen,     // (
    RParen,     // )
    Amp,        // &
    AmpAmp,     // &&
    AmpEq,      // &=
    Arrow,      // ->
    At,         // @
    Backslash,  // \
    Bang,       // !
    BangBang,   // !!
    BangEq,     // !=
    Bar,        // |
    BarBar,     // ||
    BarEq,      // |=
    Colon,      // :
    ColonColon, // ::
    Comma,      // ,
    Dot,        // .
    DotDot,     // ..
    DotDotEq,   // ..=
    Eq,         // =
    EqEq,       // ==
    FatArrow,   // =>
    Grave,      // `
    Gt,         // >
    GtEq,       // >=
    GtGt,       // >>
    GtGtEq,     // >>=
    Hash,       // #
    HashBang,   // #!
    Lt,         // <
    LtEq,       // <=
    LtLt,       // <<
    LtLtEq,     // <<=
    Minus,      // -
    MinusEq,    // -=
    Plus,       // +
    PlusEq,     // +=
    Question,   // ?
    Rem,        // %
    RemEq,      // %=
    Semi,       // ;
    Slash,      // /
    SlashEq,    // /=
    Star,       // *
    StarEq,     // *=
    Tilde,      // ~
    Xor,        // ^
    XorEq,      // ^=
    XorXor,     // ^^
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Invalid => "invalid".fmt(f),
            TokenKind::Comment => "comment".fmt(f),
            TokenKind::Identifier => "identifier".fmt(f),

            TokenKind::Break => "break".fmt(f),
            TokenKind::Cl => "cl".fmt(f),
            TokenKind::Const => "const".fmt(f),
            TokenKind::Continue => "continue".fmt(f),
            TokenKind::Else => "else".fmt(f),
            TokenKind::Enum => "enum".fmt(f),
            TokenKind::False => "false".fmt(f),
            TokenKind::For => "for".fmt(f),
            TokenKind::Fn => "fn".fmt(f),
            TokenKind::If => "if".fmt(f),
            TokenKind::Impl => "impl".fmt(f),
            TokenKind::In => "in".fmt(f),
            TokenKind::Let => "let".fmt(f),
            TokenKind::Mod => "mod".fmt(f),
            TokenKind::Mut => "mut".fmt(f),
            TokenKind::Pub => "pub".fmt(f),
            TokenKind::Return => "return".fmt(f),
            TokenKind::SelfKw => "self".fmt(f),
            TokenKind::SelfTy => "Self".fmt(f),
            TokenKind::Static => "static".fmt(f),
            TokenKind::Struct => "struct".fmt(f),
            TokenKind::Super => "super".fmt(f),
            TokenKind::True => "true".fmt(f),
            TokenKind::Type => "type".fmt(f),
            TokenKind::While => "while".fmt(f),

            TokenKind::Integer => "integer literal".fmt(f),
            TokenKind::Float => "float literal".fmt(f),
            TokenKind::String => "string literal".fmt(f),
            TokenKind::Character => "char literal".fmt(f),

            TokenKind::Op(op) => op.fmt(f),
        }
    }
}
impl FromStr for TokenKind {
    /// [FromStr] can only fail when an identifier isn't a keyword
    type Err = ();
    /// Parses a string s to return a Keyword
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "break" => Self::Break,
            "cl" => Self::Cl,
            "const" => Self::Const,
            "continue" => Self::Continue,
            "else" => Self::Else,
            "enum" => Self::Enum,
            "false" => Self::False,
            "for" => Self::For,
            "fn" => Self::Fn,
            "if" => Self::If,
            "impl" => Self::Impl,
            "in" => Self::In,
            "let" => Self::Let,
            "mod" => Self::Mod,
            "mut" => Self::Mut,
            "pub" => Self::Pub,
            "return" => Self::Return,
            "self" => Self::SelfKw,
            "Self" => Self::SelfTy,
            "static" => Self::Static,
            "struct" => Self::Struct,
            "super" => Self::Super,
            "true" => Self::True,
            "type" => Self::Type,
            "while" => Self::While,
            _ => Err(())?,
        })
    }
}

impl Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Op::LCurly => "left curly".fmt(f),
            Op::RCurly => "right curly".fmt(f),
            Op::LBrack => "left brack".fmt(f),
            Op::RBrack => "right brack".fmt(f),
            Op::LParen => "left paren".fmt(f),
            Op::RParen => "right paren".fmt(f),
            Op::Amp => "and".fmt(f),
            Op::AmpAmp => "and-and".fmt(f),
            Op::AmpEq => "and-assign".fmt(f),
            Op::Arrow => "arrow".fmt(f),
            Op::At => "at".fmt(f),
            Op::Backslash => "backslash".fmt(f),
            Op::Bang => "bang".fmt(f),
            Op::BangBang => "not-not".fmt(f),
            Op::BangEq => "not equal to".fmt(f),
            Op::Bar => "or".fmt(f),
            Op::BarBar => "or-or".fmt(f),
            Op::BarEq => "or-assign".fmt(f),
            Op::Colon => "colon".fmt(f),
            Op::ColonColon => "path separator".fmt(f),
            Op::Comma => "comma".fmt(f),
            Op::Dot => "dot".fmt(f),
            Op::DotDot => "exclusive range".fmt(f),
            Op::DotDotEq => "inclusive range".fmt(f),
            Op::Eq => "assign".fmt(f),
            Op::EqEq => "equal to".fmt(f),
            Op::FatArrow => "fat arrow".fmt(f),
            Op::Grave => "grave".fmt(f),
            Op::Gt => "greater than".fmt(f),
            Op::GtEq => "greater than or equal to".fmt(f),
            Op::GtGt => "shift right".fmt(f),
            Op::GtGtEq => "shift right-assign".fmt(f),
            Op::Hash => "hash".fmt(f),
            Op::HashBang => "shebang".fmt(f),
            Op::Lt => "less than".fmt(f),
            Op::LtEq => "less than or equal to".fmt(f),
            Op::LtLt => "shift left".fmt(f),
            Op::LtLtEq => "shift left-assign".fmt(f),
            Op::Minus => "sub".fmt(f),
            Op::MinusEq => "sub-assign".fmt(f),
            Op::Plus => "add".fmt(f),
            Op::PlusEq => "add-assign".fmt(f),
            Op::Question => "huh?".fmt(f),
            Op::Rem => "rem".fmt(f),
            Op::RemEq => "rem-assign".fmt(f),
            Op::Semi => "ignore".fmt(f),
            Op::Slash => "div".fmt(f),
            Op::SlashEq => "div-assign".fmt(f),
            Op::Star => "star".fmt(f),
            Op::StarEq => "star-assign".fmt(f),
            Op::Tilde => "tilde".fmt(f),
            Op::Xor => "xor".fmt(f),
            Op::XorEq => "xor-assign".fmt(f),
            Op::XorXor => "cat-ears".fmt(f),
        }
    }
}
