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
    As,       // as
    Break,    // "break"
    Cl,       // "cl"
    Const,    // "const"
    Continue, // "continue"
    Else,     // "else"
    Enum,     // "enum"
    False,    // "false"
    Fn,       // "fn"
    For,      // "for"
    If,       // "if"
    Impl,     // "impl"
    In,       // "in"
    Let,      // "let"
    Loop,     // "loop"
    Match,    // "match"
    Mod,      // "mod"
    Mut,      // "mut"
    Pub,      // "pub"
    Return,   // "return"
    SelfTy,   // "Self"
    Static,   // "static"
    Struct,   // "struct"
    Super,    // "super"
    True,     // "true"
    Type,     // "type"
    Use,      // "use"
    While,    // "while"
    // Delimiter or punctuation
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

            TokenKind::As => "as".fmt(f),
            TokenKind::Break => "break".fmt(f),
            TokenKind::Cl => "cl".fmt(f),
            TokenKind::Const => "const".fmt(f),
            TokenKind::Continue => "continue".fmt(f),
            TokenKind::Else => "else".fmt(f),
            TokenKind::Enum => "enum".fmt(f),
            TokenKind::False => "false".fmt(f),
            TokenKind::Fn => "fn".fmt(f),
            TokenKind::For => "for".fmt(f),
            TokenKind::If => "if".fmt(f),
            TokenKind::Impl => "impl".fmt(f),
            TokenKind::In => "in".fmt(f),
            TokenKind::Let => "let".fmt(f),
            TokenKind::Loop => "loop".fmt(f),
            TokenKind::Match => "match".fmt(f),
            TokenKind::Mod => "mod".fmt(f),
            TokenKind::Mut => "mut".fmt(f),
            TokenKind::Pub => "pub".fmt(f),
            TokenKind::Return => "return".fmt(f),
            TokenKind::SelfTy => "Self".fmt(f),
            TokenKind::Static => "static".fmt(f),
            TokenKind::Struct => "struct".fmt(f),
            TokenKind::Super => "super".fmt(f),
            TokenKind::True => "true".fmt(f),
            TokenKind::Type => "type".fmt(f),
            TokenKind::Use => "use".fmt(f),
            TokenKind::While => "while".fmt(f),

            TokenKind::LCurly => "{".fmt(f),
            TokenKind::RCurly => "}".fmt(f),
            TokenKind::LBrack => "[".fmt(f),
            TokenKind::RBrack => "]".fmt(f),
            TokenKind::LParen => "(".fmt(f),
            TokenKind::RParen => ")".fmt(f),
            TokenKind::Amp => "&".fmt(f),
            TokenKind::AmpAmp => "&&".fmt(f),
            TokenKind::AmpEq => "&=".fmt(f),
            TokenKind::Arrow => "->".fmt(f),
            TokenKind::At => "@".fmt(f),
            TokenKind::Backslash => "\\".fmt(f),
            TokenKind::Bang => "!".fmt(f),
            TokenKind::BangBang => "!!".fmt(f),
            TokenKind::BangEq => "!=".fmt(f),
            TokenKind::Bar => "|".fmt(f),
            TokenKind::BarBar => "||".fmt(f),
            TokenKind::BarEq => "|=".fmt(f),
            TokenKind::Colon => ":".fmt(f),
            TokenKind::ColonColon => "::".fmt(f),
            TokenKind::Comma => ",".fmt(f),
            TokenKind::Dot => ".".fmt(f),
            TokenKind::DotDot => "..".fmt(f),
            TokenKind::DotDotEq => "..=".fmt(f),
            TokenKind::Eq => "=".fmt(f),
            TokenKind::EqEq => "==".fmt(f),
            TokenKind::FatArrow => "=>".fmt(f),
            TokenKind::Grave => "`".fmt(f),
            TokenKind::Gt => ">".fmt(f),
            TokenKind::GtEq => ">=".fmt(f),
            TokenKind::GtGt => ">>".fmt(f),
            TokenKind::GtGtEq => ">>=".fmt(f),
            TokenKind::Hash => "#".fmt(f),
            TokenKind::HashBang => "#!".fmt(f),
            TokenKind::Lt => "<".fmt(f),
            TokenKind::LtEq => "<=".fmt(f),
            TokenKind::LtLt => "<<".fmt(f),
            TokenKind::LtLtEq => "<<=".fmt(f),
            TokenKind::Minus => "-".fmt(f),
            TokenKind::MinusEq => "-=".fmt(f),
            TokenKind::Plus => "+".fmt(f),
            TokenKind::PlusEq => "+=".fmt(f),
            TokenKind::Question => "?".fmt(f),
            TokenKind::Rem => "%".fmt(f),
            TokenKind::RemEq => "%=".fmt(f),
            TokenKind::Semi => ";".fmt(f),
            TokenKind::Slash => "/".fmt(f),
            TokenKind::SlashEq => "/=".fmt(f),
            TokenKind::Star => "*".fmt(f),
            TokenKind::StarEq => "*=".fmt(f),
            TokenKind::Tilde => "~".fmt(f),
            TokenKind::Xor => "^".fmt(f),
            TokenKind::XorEq => "^=".fmt(f),
            TokenKind::XorXor => "^^".fmt(f),
        }
    }
}
impl FromStr for TokenKind {
    /// [FromStr] can only fail when an identifier isn't a keyword
    type Err = ();
    /// Parses a string s to return a Keyword
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "as" => Self::As,
            "break" => Self::Break,
            "cl" => Self::Cl,
            "const" => Self::Const,
            "continue" => Self::Continue,
            "else" => Self::Else,
            "enum" => Self::Enum,
            "false" => Self::False,
            "fn" => Self::Fn,
            "for" => Self::For,
            "if" => Self::If,
            "impl" => Self::Impl,
            "in" => Self::In,
            "let" => Self::Let,
            "loop" => Self::Loop,
            "match" => Self::Match,
            "mod" => Self::Mod,
            "mut" => Self::Mut,
            "pub" => Self::Pub,
            "return" => Self::Return,
            "Self" => Self::SelfTy,
            "static" => Self::Static,
            "struct" => Self::Struct,
            "super" => Self::Super,
            "true" => Self::True,
            "type" => Self::Type,
            "use" => Self::Use,
            "while" => Self::While,
            _ => Err(())?,
        })
    }
}
