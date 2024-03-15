//! # Universally useful structures
//! - [Span](struct@span::Span): Stores a start and end [Loc](struct@span::Loc)
//! - [Loc](struct@span::Loc): Stores the index in a stream
#![warn(clippy::all)]
pub mod span;

pub mod tree;
