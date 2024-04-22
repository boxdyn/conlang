//! Parses [tokens](cl_token::token) into an [AST](cl_ast)
//!
//! For the full grammar, see [grammar.ebnf][1]
//!
//! [1]: https://github.com/boxdyn/conlang/src/branch/main/grammar.ebnf
#![warn(clippy::all)]
#![feature(decl_macro)]

pub use parser::Parser;

use cl_structures::span::*;
use cl_token::*;

pub mod error;

pub mod parser;

pub mod inliner;
