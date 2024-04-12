//! Additional data stored within a [Token](super::Token),
//! external to its [TokenKind](super::token_type::TokenKind)
/// Additional data stored within a [Token](super::Token),
/// external to its [TokenKind](super::token_type::TokenKind)
#[derive(Clone, Debug, PartialEq)]
pub enum TokenData {
    /// [Token](super::Token) contains an [identifier](str)
    Identifier(Box<str>),
    /// [Token](super::Token) contains a [String]
    String(String),
    /// [Token](super::Token) contains a [character](char)
    Character(char),
    /// [Token](super::Token) contains an [integer](u128)
    Integer(u128),
    /// [Token](super::Token) contains a [float](f64)
    Float(f64),
    /// [Token](super::Token) contains no additional data
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
/// Implements [From] for an enum
macro from($($value:ident: $src:ty => $dst:expr),*$(,)?) {
    $(impl From<$src> for TokenData {
        fn from($value: $src) -> Self { $dst }
    })*
}
impl std::fmt::Display for TokenData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenData::Identifier(v) => v.fmt(f),
            TokenData::String(v) => write!(f, "\"{v}\""),
            TokenData::Character(v) => write!(f, "'{v}'"),
            TokenData::Integer(v) => v.fmt(f),
            TokenData::Float(v) => v.fmt(f),
            TokenData::None => "None".fmt(f),
        }
    }
}
