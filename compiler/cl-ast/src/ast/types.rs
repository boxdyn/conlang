//! The default (as-parsed) implementation of the AST's customization points.

use std::fmt::Display;

use crate::{ast::AstTypes, fmt::FmtAdapter};

use cl_structures::{intern::interned::Interned, span::Span};

/// The types emitted by the parser
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DefaultTypes;

impl AstTypes for DefaultTypes {
    type Annotation = Span;
    type Literal = Literal;
    type MacroId = Symbol;
    type Symbol = Symbol;
    type Path = Path;
}

impl std::fmt::Display for DefaultTypes {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

/// An interned symbol (i.e. a name)
pub type Symbol = Interned<'static, str>;

/// A qualified identifier
///
/// TODO: qualify identifier
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path {
    // TODO: Identifier interning
    pub parts: Vec<Symbol>,
    // TODO: generic parameters
}

impl Path {
    /// Returns the "root path": `::`
    pub fn root() -> Path {
        Path { parts: vec!["".into()] }
    }
    /// Returns the last path segment
    pub fn name(&self) -> Option<Symbol> {
        match self.parts.as_slice() {
            [] => None,
            [.., name] => Some(*name),
        }
    }
}

impl From<&str> for Path {
    fn from(value: &str) -> Self {
        Self { parts: vec![value.into()] }
    }
}

impl From<Symbol> for Path {
    fn from(value: Symbol) -> Self {
        Self { parts: vec![value] }
    }
}

impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { parts } = self;
        match parts.as_slice() {
            [Interned("", ..)] => f.write_str("::"),
            parts => f.list(parts, "::"),
        }
    }
}

/// A literal value (boolean, character, integer, string)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Literal {
    /// A boolean literal: true | false
    Bool(bool),
    /// A character literal: 'a', '\u{1f988}'
    Char(char),
    /// An integer literal: 0, 123, 0x10
    Int(u128, u32),
    /// A string literal:
    Str(String),
}

impl Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool(v) => v.fmt(f),
            Self::Char(c) => write!(f, "'{}'", c.escape_debug()),
            Self::Int(i, 2) => write!(f, "0b{i:b}"),
            Self::Int(i, 8) => write!(f, "0o{i:o}"),
            Self::Int(i, 16) => write!(f, "0x{i:x}"),
            Self::Int(i, _) => i.fmt(f),
            Self::Str(s) => write!(f, "\"{}\"", s.escape_debug()),
        }
    }
}
