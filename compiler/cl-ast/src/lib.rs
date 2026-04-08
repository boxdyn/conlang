//! # The Abstract Syntax Tree
//! Contains definitions of Conlang AST Nodes.
//!
//! All AST nodes are parameterized by an implementation of the
//! [AstTypes] trait. Implementers of the [AstTypes] trait are
//! responsible for
//!
//! # Notable nodes
//! - [Expr] Expressions
//!   - [Bind], [Use], and [Make] expressions
//!   - [Op] operators
//! - [Pat]: Pattern matching operators
#![warn(clippy::all)]

pub use ast::*;

pub mod ast;
pub mod desugar;
pub mod fmt;
