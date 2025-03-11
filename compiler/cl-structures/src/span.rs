//! - [struct@Span]: Stores the start and end [struct@Loc] of a notable AST node
//! - [struct@Loc]: Stores the line/column of a notable AST node
#![allow(non_snake_case)]

/// Stores the start and end [locations](struct@Loc) within the token stream
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    pub head: Loc,
    pub tail: Loc,
}
pub const fn Span(head: Loc, tail: Loc) -> Span {
    Span { head, tail }
}

impl Span {
    pub const fn dummy() -> Self {
        Span { head: Loc::dummy(), tail: Loc::dummy() }
    }
}

/// Stores a read-only (line, column) location in a token stream
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Loc {
    line: u32,
    col: u32,
}
pub const fn Loc(line: u32, col: u32) -> Loc {
    Loc { line, col }
}
impl Loc {
    pub const fn dummy() -> Self {
        Loc { line: 0, col: 0 }
    }
    pub const fn line(self) -> u32 {
        self.line
    }
    pub const fn col(self) -> u32 {
        self.col
    }
}

impl std::fmt::Display for Loc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Loc { line, col } = self;
        write!(f, "{line}:{col}")
    }
}
