//! Walks a Conlang AST, interpreting it as a program.
#![warn(clippy::all)]
#![feature(decl_macro)]

use cl_ast::Sym;
use convalue::ConValue;
use env::Environment;
use error::{Error, IResult};
use interpret::Interpret;

/// Callable types can be called from within a Conlang program
pub trait Callable: std::fmt::Debug {
    /// Calls this [Callable] in the provided [Environment], with [ConValue] args  \
    /// The Callable is responsible for checking the argument count and validating types
    fn call(&self, interpreter: &mut Environment, args: &[ConValue]) -> IResult<ConValue>;
    /// Returns the common name of this identifier.
    fn name(&self) -> Sym;
}

/// [BuiltIn]s are [Callable]s with bespoke definitions
pub trait BuiltIn: std::fmt::Debug + Callable {
    fn description(&self) -> &str;
}

pub mod convalue;

pub mod interpret;

pub mod function;

pub mod builtin;

pub mod env;

pub mod error;

#[cfg(test)]
mod tests;
