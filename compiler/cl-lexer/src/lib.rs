//! A lobster
//!
//! The [Lexer] takes an input text and carves it up into [Tokens](cl_token::Token)
//! which hold a [Lexeme](cl_token::Lexeme), [Token Kind](cl_token::TKind), and
//! [Span](struct@cl_structures::span::Span).
//!
//! You can create a new [Lexer] with [Lexer::new], and pull a token with [Lexer::scan].
//!
//! # Examples:
//!
//! ```rust
//! # use cl_lexer::LexError;
//! # fn main() -> Result<(), LexError> {
//! use cl_lexer::{Lexer, Symbol};
//! use cl_token::{Token, Lexeme, TKind};
//!
//! let sample_text = r#"
//!     "This is a string\n"
//!     '\u{1f988}'
//!     123456
//!     098765
//!     0x4141
//!     0o7777
//!     0b1100
//!     x + 1
//! "#;
//!
//! let mut lexer = Lexer::new(Symbol::from("lexer test"), sample_text);
//!
//! let a_string = lexer.scan()?;
//! assert_eq!(a_string.lexeme.str(), Some("This is a string\n"));
//! assert_eq!(a_string.kind, TKind::String);
//!
//! let a_char = lexer.scan()?;
//! assert_eq!(a_char.lexeme.char(), Some('\u{1f988}'));
//! assert_eq!(a_char.kind, TKind::Character);
//!
//! let num_123456 = lexer.scan()?;
//! assert_eq!(num_123456.lexeme.int(), Some(123456));
//! assert_eq!(num_123456.kind, TKind::Integer);
//!
//! let num_098765 = lexer.scan()?;
//! assert_eq!(num_098765.lexeme.int(), Some(98765));
//! assert_eq!(num_098765.kind, TKind::Integer);
//!
//! let num_0x4141 = lexer.scan()?;
//! assert_eq!(num_0x4141.lexeme.int(), Some(16705));
//! assert_eq!(num_0x4141.kind, TKind::Integer);
//!
//! let num_0o7777 = lexer.scan()?;
//! assert_eq!(num_0o7777.lexeme.int(), Some(4095));
//! assert_eq!(num_0o7777.kind, TKind::Integer);
//!
//! let num_0b1100 = lexer.scan()?;
//! assert_eq!(num_0b1100.lexeme.int(), Some(12));
//! assert_eq!(num_0b1100.kind, TKind::Integer);
//!
//! let id_x = lexer.scan()?;
//! assert_eq!(id_x.lexeme.str(), Some("x"));
//! assert_eq!(id_x.kind, TKind::Identifier);
//!
//! let op_plus = lexer.scan()?;
//! assert_eq!(op_plus.lexeme.str(), Some("+"));
//! assert_eq!(op_plus.kind, TKind::Plus);
//!
//! let num_1 = lexer.scan()?;
//! assert_eq!(num_1.lexeme.int(), Some(1));
//! assert_eq!(num_1.kind, TKind::Integer);
//!
//! assert!(lexer.scan().is_err());
//!
//! # Ok(())
//! # }
//! ```

pub mod error;

pub mod lexer;

pub use crate::{
    error::{EOF, LexError, LexFailure},
    lexer::{Lexer, Symbol},
};
