//! Stores a [Token's](super::Token) lexical information
use std::{fmt::Display, str::FromStr};

/// Stores a [Token's](super::Token) lexical information
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// Invalid sequence
    Invalid,
    /// Any kind of comment
    Comment,
    /// Any tokenizable literal (See [TokenData](super::TokenData))
    Literal,
    /// A non-keyword identifier
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
    Loop,
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
    /// Delimiter or punctuation
    Punct(Punct),
}

/// An operator character (delimiter, punctuation)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Punct {
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
            TokenKind::Literal => "literal".fmt(f),
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
            TokenKind::Loop => "loop".fmt(f),
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

            TokenKind::Punct(op) => op.fmt(f),
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
            "loop" => Self::Loop,
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

impl Display for Punct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Punct::LCurly => "{".fmt(f),
            Punct::RCurly => "}".fmt(f),
            Punct::LBrack => "[".fmt(f),
            Punct::RBrack => "]".fmt(f),
            Punct::LParen => "(".fmt(f),
            Punct::RParen => ")".fmt(f),
            Punct::Amp => "&".fmt(f),
            Punct::AmpAmp => "&&".fmt(f),
            Punct::AmpEq => "&=".fmt(f),
            Punct::Arrow => "->".fmt(f),
            Punct::At => "@".fmt(f),
            Punct::Backslash => "\\".fmt(f),
            Punct::Bang => "!".fmt(f),
            Punct::BangBang => "!!".fmt(f),
            Punct::BangEq => "!=".fmt(f),
            Punct::Bar => "|".fmt(f),
            Punct::BarBar => "||".fmt(f),
            Punct::BarEq => "|=".fmt(f),
            Punct::Colon => ":".fmt(f),
            Punct::ColonColon => "::".fmt(f),
            Punct::Comma => ",".fmt(f),
            Punct::Dot => ".".fmt(f),
            Punct::DotDot => "..".fmt(f),
            Punct::DotDotEq => "..=".fmt(f),
            Punct::Eq => "=".fmt(f),
            Punct::EqEq => "==".fmt(f),
            Punct::FatArrow => "=>".fmt(f),
            Punct::Grave => "`".fmt(f),
            Punct::Gt => ">".fmt(f),
            Punct::GtEq => ">=".fmt(f),
            Punct::GtGt => ">>".fmt(f),
            Punct::GtGtEq => ">>=".fmt(f),
            Punct::Hash => "#".fmt(f),
            Punct::HashBang => "#!".fmt(f),
            Punct::Lt => "<".fmt(f),
            Punct::LtEq => "<=".fmt(f),
            Punct::LtLt => "<<".fmt(f),
            Punct::LtLtEq => "<<=".fmt(f),
            Punct::Minus => "-".fmt(f),
            Punct::MinusEq => "-=".fmt(f),
            Punct::Plus => "+".fmt(f),
            Punct::PlusEq => "+=".fmt(f),
            Punct::Question => "?".fmt(f),
            Punct::Rem => "%".fmt(f),
            Punct::RemEq => "%=".fmt(f),
            Punct::Semi => ";".fmt(f),
            Punct::Slash => "/".fmt(f),
            Punct::SlashEq => "/=".fmt(f),
            Punct::Star => "*".fmt(f),
            Punct::StarEq => "*=".fmt(f),
            Punct::Tilde => "~".fmt(f),
            Punct::Xor => "^".fmt(f),
            Punct::XorEq => "^=".fmt(f),
            Punct::XorXor => "^^".fmt(f),
        }
    }
}
