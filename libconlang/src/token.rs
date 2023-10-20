//! Stores a component of a file as a type and span
use std::ops::Range;

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
    // Delimiters
    LCurly,
    RCurly,
    LBrack,
    RBrack,
    LParen,
    RParen,
    // Compound punctuation
    Lsh,
    Rsh,
    AmpAmp,
    BarBar,
    NotNot,
    CatEar,
    EqEq,
    GtEq,
    LtEq,
    NotEq,
    StarEq,
    DivEq,
    RemEq,
    AddEq,
    SubEq,
    AndEq,
    OrEq,
    XorEq,
    LshEq,
    RshEq,
    Arrow,
    FatArrow,
    // Simple punctuation
    Semi,
    Dot,
    Star,
    Div,
    Plus,
    Minus,
    Rem,
    Bang,
    Eq,
    Lt,
    Gt,
    Amp,
    Bar,
    Xor,
    Hash,
    At,
    Colon,
    Backslash,
    Question,
    Comma,
    Tilde,
    Grave,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Token {
    ty: Type,
    pub head: usize,
    pub tail: usize,
    line: u32,
    col: u32,
}
impl Token {
    pub fn new(ty: Type, head: usize, tail: usize, line: u32, col: u32) -> Self {
        Self { ty, head, tail, line, col }
    }
    /// Cast this [Token] to a new [Type]
    pub fn cast(self, ty: Type) -> Self {
        Self { ty, ..self }
    }
    /// Hack to work around the current [lexer's design limitations](crate::lexer)
    pub fn rebound(self, head: usize, tail: usize) -> Self {
        Self { head, tail, ..self }
    }
    /// Gets the line from this token
    pub fn line(&self) -> u32 {
        self.line
    }
    /// Gets the column from this token
    pub fn col(&self) -> u32 {
        self.col
    }
    pub fn is_empty(&self) -> bool {
        self.tail == self.head
    }
    /// Gets the length of the token, in bytes
    pub fn len(&self) -> usize {
        self.tail - self.head
    }
    /// Gets the [Type] of the token
    pub fn ty(&self) -> Type {
        self.ty
    }
    /// Gets the exclusive range of the token
    pub fn range(&self) -> Range<usize> {
        self.head..self.tail
    }
}

impl std::ops::Index<&Token> for str {
    type Output = str;
    fn index(&self, index: &Token) -> &Self::Output {
        &self[index.range()]
    }
}
