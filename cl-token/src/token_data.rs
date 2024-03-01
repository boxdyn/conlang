//! Additional data stored within a [Token](super::Token),
//! external to its [Type](super::token_type::Type)
/// Additional data stored within a [Token](super::Token),
/// external to its [Type](super::token_type::Type)
#[derive(Clone, Debug, PartialEq)]
pub enum Data {
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
    $(impl From<$src> for Data {
        fn from($value: $src) -> Self { $dst }
    })*
}
impl std::fmt::Display for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Data::Identifier(v) => v.fmt(f),
            Data::String(v) => write!(f, "\"{v}\""),
            Data::Character(v) => write!(f, "'{v}'"),
            Data::Integer(v) => v.fmt(f),
            Data::Float(v) => v.fmt(f),
            Data::None => "None".fmt(f),
        }
    }
}
