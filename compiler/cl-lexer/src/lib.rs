//! A lobster
//!
//! The [Lexer] takes an input text and carves it up into [Tokens](cl_token::Token)
//! which hold a [Lexeme](cl_token::Lexeme), [Token Kind](cl_token::TKind), and
//! [Span](cl_structures::span::Span).
//! 
//! You can create a new [Lexer] with [Lexer::new], and 

pub mod error;

pub mod lexer;

pub use crate::{
    error::{EOF, LexError, LexFailure},
    lexer::{Lexer, Symbol},
};
