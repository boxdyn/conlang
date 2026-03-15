//! Walks a Conlang AST, interpreting it as a program.
#![warn(clippy::all)]
#![feature(decl_macro, rev_into_inner, string_into_chars)]
#![expect(unused, reason = "Work in progress")]

use cl_ast::types::Symbol as Sym;
use convalue::ConValue;
use env::Environment;
use error::{Error, ErrorKind, IResult};
use interpret::Interpret;

/// Callable types can be called from within a Conlang program
pub trait Callable {
    /// Calls this [Callable] in the provided [Environment], with [ConValue] args  \
    /// The Callable is responsible for checking the argument count and validating types
    fn call(&self, interpreter: &mut Environment, args: &[ConValue]) -> IResult<ConValue>;
    /// Returns the common name of this callable, if it has one.
    fn name(&self) -> Option<Sym>;
}

pub mod typeinfo;

pub mod place;

pub mod convalue;

pub mod interpret;

pub mod function;

pub mod builtin;

pub mod pattern {}

pub mod env;

pub mod modules {}

pub mod error;

#[cfg(test)]
mod tests;
