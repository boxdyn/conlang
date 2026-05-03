//! Houses the Conlang [Parser].
//!
//! Conlang uses a pair of mutually recursive Pratt [Parse]rs
//! to parse its [`Expr`ession][Expr] and [`Pat`tern][Pat] sub-languages.
//!
//! These parsers are implemented in [`expr`] and [`pat`], respectively.
//!
//! The [Parser] (or "parser context" if you're pedantic) keeps track of
//! the last-peeked [Token], the [Span] information,
//! and whether or not the last-consumed [Token] is allowed to stand in
//! for a semicolon (`;`) in Conlang's `do` expressions.
//!
//! [Expr]: cl_ast::ast::Expr
//! [Pat]: cl_ast::ast::Pat
//! [Token]: cl_token::Token
//! [Span]: cl_structures::span::Span

pub mod inliner;
mod parser;
pub use parser::*;
