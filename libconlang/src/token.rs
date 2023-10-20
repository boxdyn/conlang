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
