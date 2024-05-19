//! # Universally useful structures
//! - [Span](struct@span::Span): Stores a start and end [Loc](struct@span::Loc)
//! - [Loc](struct@span::Loc): Stores the index in a stream
//! - [TypedInterner][ti] & [StringInterner][si]: Provies stable, unique allocations
//! - [Stack](stack::Stack): Contiguous collections with constant capacity
//! 
//! [ti]: intern::typed_interner::TypedInterner
//! [si]: intern::string_interner::StringInterner
#![warn(clippy::all)]
#![feature(inline_const, dropck_eyepatch, decl_macro, get_many_mut)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod intern;

pub mod span;

pub mod tree;

pub mod stack;

pub mod index_map;
