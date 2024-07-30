//! # Token
//!
//! Stores a component of a file as a [TokenKind], some [TokenData], and a line and column number
#![warn(clippy::all)]
#![feature(decl_macro)]

pub mod token;
pub mod token_data;
pub mod token_type;

pub use token::Token;
pub use token_data::TokenData;
pub use token_type::TokenKind;
