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

            TokenKind::LCurly => "left curly".fmt(f),
            TokenKind::RCurly => "right curly".fmt(f),
            TokenKind::LBrack => "left brack".fmt(f),
            TokenKind::RBrack => "right brack".fmt(f),
            TokenKind::LParen => "left paren".fmt(f),
            TokenKind::RParen => "right paren".fmt(f),
            TokenKind::Amp => "and".fmt(f),
            TokenKind::AmpAmp => "and-and".fmt(f),
            TokenKind::AmpEq => "and-assign".fmt(f),
            TokenKind::Arrow => "arrow".fmt(f),
            TokenKind::At => "at".fmt(f),
            TokenKind::Backslash => "backslash".fmt(f),
            TokenKind::Bang => "bang".fmt(f),
            TokenKind::BangBang => "not-not".fmt(f),
            TokenKind::BangEq => "not equal to".fmt(f),
            TokenKind::Bar => "or".fmt(f),
            TokenKind::BarBar => "or-or".fmt(f),
            TokenKind::BarEq => "or-assign".fmt(f),
            TokenKind::Colon => "colon".fmt(f),
            TokenKind::ColonColon => "path separator".fmt(f),
            TokenKind::Comma => "comma".fmt(f),
            TokenKind::Dot => "dot".fmt(f),
            TokenKind::DotDot => "exclusive range".fmt(f),
            TokenKind::DotDotEq => "inclusive range".fmt(f),
            TokenKind::Eq => "assign".fmt(f),
            TokenKind::EqEq => "equal to".fmt(f),
            TokenKind::FatArrow => "fat arrow".fmt(f),
            TokenKind::Grave => "grave".fmt(f),
            TokenKind::Gt => "greater than".fmt(f),
            TokenKind::GtEq => "greater than or equal to".fmt(f),
            TokenKind::GtGt => "shift right".fmt(f),
            TokenKind::GtGtEq => "shift right-assign".fmt(f),
            TokenKind::Hash => "hash".fmt(f),
            TokenKind::HashBang => "shebang".fmt(f),
            TokenKind::Lt => "less than".fmt(f),
            TokenKind::LtEq => "less than or equal to".fmt(f),
            TokenKind::LtLt => "shift left".fmt(f),
            TokenKind::LtLtEq => "shift left-assign".fmt(f),
            TokenKind::Minus => "sub".fmt(f),
            TokenKind::MinusEq => "sub-assign".fmt(f),
            TokenKind::Plus => "add".fmt(f),
            TokenKind::PlusEq => "add-assign".fmt(f),
            TokenKind::Question => "huh?".fmt(f),
            TokenKind::Rem => "rem".fmt(f),
            TokenKind::RemEq => "rem-assign".fmt(f),
            TokenKind::Semi => "ignore".fmt(f),
            TokenKind::Slash => "div".fmt(f),
            TokenKind::SlashEq => "div-assign".fmt(f),
            TokenKind::Star => "star".fmt(f),
            TokenKind::StarEq => "star-assign".fmt(f),
            TokenKind::Tilde => "tilde".fmt(f),
            TokenKind::Xor => "xor".fmt(f),
            TokenKind::XorEq => "xor-assign".fmt(f),
            TokenKind::XorXor => "cat-ears".fmt(f),
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
