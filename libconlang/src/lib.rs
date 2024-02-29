//! Conlang is an expression-based programming language with similarities to Rust and Python
#![warn(clippy::all)]
#![feature(decl_macro)]

pub mod common;

pub mod token;

pub mod ast;

pub mod lexer;

pub mod parser;

pub mod resolver;

#[cfg(test)]
mod tests;
