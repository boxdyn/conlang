//! # Universally useful structures
//! - [Span](struct@span::Span): Stores a start and end position in a named stream
//! - [DroplessInterner][di] & [StringInterner][si]: Provides stable, unique allocations
//! - [Stack](stack::Stack): Contiguous collections with constant capacity
//! - [IndexMap][im]: A map from [map indices][mi] to values
//!
//! [di]: intern::dropless_interner::DroplessInterner
//! [si]: intern::string_interner::StringInterner
//! [im]: index_map::IndexMap
//! [mi]: index_map::MapIndex
#![warn(clippy::all)]
#![feature(dropck_eyepatch, decl_macro)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod intern;

pub mod span;

pub mod tree;

pub mod list;

pub mod stack;

pub mod index_map;

pub use cl_arena::dropless_arena;

pub use cl_arena::typed_arena;
