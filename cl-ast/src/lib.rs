//! # The Abstract Syntax Tree
//! Contains definitions of Conlang AST Nodes.
//!
//! # Notable nodes
//! - [Item] and [ItemKind]: Top-level constructs
//! - [Stmt] and [StmtKind]: Statements
//! - [Expr] and [ExprKind]: Expressions
//!   - [Assign], [Binary], and [Unary] expressions
//!   - [AssignKind], [BinaryKind], and [UnaryKind] operators
//! - [Ty] and [TyKind]: Type qualifiers
//! - [Path]: Path expressions
#![warn(clippy::all)]
#![feature(decl_macro)]

pub use ast::*;

pub mod ast;
pub mod ast_impl;
pub mod format;
