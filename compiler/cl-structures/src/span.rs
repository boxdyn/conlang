//! - [struct@Span]: Stores the start and end position of a notable AST node
#![allow(non_snake_case)]
use std::ops::Range;

use crate::intern::interned::Symbol;

/// Stores the start and end byte position
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub path: Symbol,
    pub head: u32,
    pub tail: u32,
}

impl std::fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { path, head, tail } = self;
        match path.0 {
            "" => write!(f, "[{head}:{tail}]"),
            _ => write!(f, "[{path}:{head}:{tail}]"),
        }
    }
}

#[expect(non_snake_case)]
/// Stores the start and end byte position
pub const fn Span(path: Symbol, head: u32, tail: u32) -> Span {
    Span { path, head, tail }
}

impl Span {
    /// Computes the [struct@Span] containing both `self` and `other`
    pub fn merge(self, other: Span) -> Span {
        if !(self.path.is_empty() || other.path.is_empty()) {
            assert_eq!(self.path, other.path, "Attempted to merge unrelated paths!")
        }
        Span { path: self.path, head: self.head.min(other.head), tail: self.tail.max(other.tail) }
    }
}

impl From<Span> for Range<usize> {
    fn from(value: Span) -> Self {
        let Span { path: _, head, tail } = value;
        (head as usize)..(tail as usize)
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { path, head, tail } = self;
        match path.0 {
            "" => write!(f, "{head}:{tail}"),
            _ => write!(f, "{path}:{head}:{tail}"),
        }
    }
}
