//! Conlang is an expression-based programming language with similarities to Rust and Python
#![warn(clippy::all)]
#![feature(decl_macro)]

pub mod lexer;

pub mod resolver;

#[cfg(test)]
mod tests;
