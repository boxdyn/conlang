//! Stores a component of a file as a type and span

mod token_type;
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

#[derive(Clone, Debug, PartialEq)]
pub enum TokenData {
    Identifier(Box<str>),
    String(String),
    Character(char),
    Integer(u128),
    Float(f64),
    None,
}
from! {
    value: &str => Self::Identifier(value.into()),
    value: String => Self::String(value),
    value: u128 => Self::Integer(value),
    value: f64 => Self::Float(value),
    value: char => Self::Character(value),
    _v:    () => Self::None,
}
macro from($($value:ident: $src:ty => $dst:expr),*$(,)?) {
    $(impl From<$src> for TokenData {
        fn from($value: $src) -> Self { $dst }
    })*
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    ty: Type,
    data: TokenData,
    line: u32,
    col: u32,
}
impl Token {
    /// Creates a new [Token] out of a [Type], [TokenData], line, and column.
    pub fn new(ty: Type, data: impl Into<TokenData>, line: u32, col: u32) -> Self {
        Self { ty, data: data.into(), line, col }
    }
    /// Casts this token to a new [Type]
    pub fn cast(self, ty: Type) -> Self {
        Self { ty, ..self }
    }
    /// Gets the [Type] of this token
    pub fn ty(&self) -> Type {
        self.ty
    }
    /// Gets the [TokenData] of this token
    pub fn data(&self) -> &TokenData {
        &self.data
    }
    pub fn into_data(self) -> TokenData {
        self.data
    }
    pub fn line(&self) -> u32 {
        self.line
    }
    pub fn col(&self) -> u32 {
        self.col
    }
}
