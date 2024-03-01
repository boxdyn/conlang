//! A [Token] contains a single unit of lexical information, and an optional bit of [Data]
use super::{Data, Type};

/// Contains a single unit of lexical information,
/// and an optional bit of [Data]
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    ty: Type,
    data: Data,
    line: u32,
    col: u32,
}
impl Token {
    /// Creates a new [Token] out of a [Type], [Data], line, and column.
    pub fn new(ty: Type, data: impl Into<Data>, line: u32, col: u32) -> Self {
        Self { ty, data: data.into(), line, col }
    }
    /// Casts this token to a new [Type]
    pub fn cast(self, ty: Type) -> Self {
        Self { ty, ..self }
    }
    /// Returns the [Type] of this token
    pub fn ty(&self) -> Type {
        self.ty
    }
    /// Returns a reference to this token's [Data]
    pub fn data(&self) -> &Data {
        &self.data
    }
    /// Converts this token into its inner [Data]
    pub fn into_data(self) -> Data {
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
