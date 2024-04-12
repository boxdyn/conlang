//! A [Token] contains a single unit of lexical information, and an optional bit of [TokenData]
use super::{TokenData, TokenKind};

/// Contains a single unit of lexical information,
/// and an optional bit of [TokenData]
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    ty: TokenKind,
    data: TokenData,
    line: u32,
    col: u32,
}
impl Token {
    /// Creates a new [Token] out of a [TokenKind], [TokenData], line, and column.
    pub fn new(ty: TokenKind, data: impl Into<TokenData>, line: u32, col: u32) -> Self {
        Self { ty, data: data.into(), line, col }
    }
    /// Casts this token to a new [TokenKind]
    pub fn cast(self, ty: TokenKind) -> Self {
        Self { ty, ..self }
    }
    /// Returns the [TokenKind] of this token
    pub fn ty(&self) -> TokenKind {
        self.ty
    }
    /// Returns a reference to this token's [TokenData]
    pub fn data(&self) -> &TokenData {
        &self.data
    }
    /// Converts this token into its inner [TokenData]
    pub fn into_data(self) -> TokenData {
        self.data
    }
    /// Returns the line where this token originated
    pub fn line(&self) -> u32 {
        self.line
    }
    /// Returns the column where this token originated
    pub fn col(&self) -> u32 {
        self.col
    }
}
