//! Embed Conlang code into your Rust project!
//!
//! # This crate is experimental, and has no guarantees of stability.
#![feature(decl_macro)]
#![cfg_attr(test, feature(assert_matches))]
#![allow(unused_imports)]

pub use cl_interpret::{convalue::ConValue as Value, env::Environment};

use cl_ast::{Block, Module, ast_visitor::Fold};
use cl_interpret::{convalue::ConValue, interpret::Interpret};
use cl_lexer::Lexer;
use cl_parser::{Parser, error::Error as ParseError, inliner::ModuleInliner};
use std::{path::Path, sync::OnceLock};

/// Constructs a function which evaluates a Conlang Block
///
/// # Examples
///
/// Bind and use a variable
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use cl_embed::{conlang, Environment, Value};
///
/// let mut env = Environment::new();
///
/// // Bind a variable named `message` to "Hello, world!"
/// env.bind("message", "Hello, World!");
///
/// let print_hello = conlang!{
///     println(message);
/// };
///
/// // Run the function
/// let ret = print_hello(&mut env)?;
///
/// // `println` returns Empty
/// assert!(matches!(ret, Value::Empty));
///
/// # Ok(())
/// # }
/// ```
pub macro conlang (
    $($t:tt)*
) {{
    // Parse once
    static FN: OnceLock<Result<Block, ParseError>> = OnceLock::new();

    |env: &mut Environment| -> Result<ConValue, EvalError> {
        FN.get_or_init(|| {
            // TODO: embed the full module tree at compile time
            let path = AsRef::<Path>::as_ref(&concat!(env!("CARGO_MANIFEST_DIR"),"/../../", file!())).with_extension("");
            let mut mi = ModuleInliner::new(path);
            let code = mi.fold_block(
                Parser::new(
                    concat!(file!(), ":", line!(), ":"),
                    Lexer::new(stringify!({ $($t)* })),
                )
                .parse::<Block>()?,
            );
            if let Some((ie, pe)) = mi.into_errs() {
                for (file, err) in ie {
                    eprintln!("{}: {err}", file.display());
                }
                for (file, err) in pe {
                    eprintln!("{}: {err}", file.display());
                }
            }
            Ok(code)
        })
        .as_ref()
        .map_err(Clone::clone)?
        .interpret(env)
        .map_err(Into::into)
    }
}}

#[derive(Clone, Debug)]
pub enum EvalError {
    Parse(cl_parser::error::Error),
    Interpret(cl_interpret::error::Error),
}

impl From<cl_parser::error::Error> for EvalError {
    fn from(value: cl_parser::error::Error) -> Self {
        Self::Parse(value)
    }
}
impl From<cl_interpret::error::Error> for EvalError {
    fn from(value: cl_interpret::error::Error) -> Self {
        Self::Interpret(value)
    }
}
impl std::error::Error for EvalError {}
impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::Parse(error) => error.fmt(f),
            EvalError::Interpret(error) => error.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use super::*;

    #[test]
    fn it_works() -> Result<(), EvalError> {
        let mut env = Environment::new();

        let result = conlang! {
            fn add(left, right) -> isize {
                left + right
            }

            add(2, 2)
        }(&mut env);

        assert_matches!(result, Ok(Value::Int(4)));

        Ok(())
    }
}
