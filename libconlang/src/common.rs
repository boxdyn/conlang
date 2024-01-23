//! # Universally useful structures
//! - [struct@Span]: Stores the start and end [struct@Loc] of a notable AST node
//! - [struct@Loc]: Stores the line/column of a notable AST node
#![allow(non_snake_case)]
use crate::lexer::Lexer;

/// Stores the start and end [locations](struct@Loc) within the token stream
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    pub head: Loc,
    pub tail: Loc,
}
pub fn Span(head: Loc, tail: Loc) -> Span {
    Span { head, tail }
}

/// Stores a read-only (line, column) location in a token stream
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Loc {
    line: u32,
    col: u32,
}
pub fn Loc(line: u32, col: u32) -> Loc {
    Loc { line, col }
}
impl Loc {
    pub fn line(self) -> u32 {
        self.line
    }
    pub fn col(self) -> u32 {
        self.col
    }
}

impl std::fmt::Display for Loc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Loc { line, col } = self;
        write!(f, "{line}:{col}:")
    }
}

impl<'t> From<&Lexer<'t>> for Loc {
    fn from(value: &Lexer<'t>) -> Self {
        Loc(value.line(), value.col())
    }
}
