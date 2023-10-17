//! Conlang is an expression-based programming language with similarities to Rust
#![warn(clippy::all)]
#![feature(decl_macro)]
pub mod token;

pub mod ast;

pub mod lexer;

pub mod parser;

pub mod pretty_printer;

pub mod interpreter {
    //! Interprets an AST as a program
}

#[cfg(test)]
mod tests;
