//! # Universally useful structures
//! - [Span](struct@span::Span): Stores a start and end [Loc](struct@span::Loc)
//! - [Loc](struct@span::Loc): Stores the index in a stream
#![warn(clippy::all)]
#![feature(inline_const, dropck_eyepatch, decl_macro)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod span;

pub mod tree;

pub mod stack;

pub mod intern_pool;
