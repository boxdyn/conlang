//! Walks a Conlang AST, interpreting it as a program.
#![warn(clippy::all)]
#![feature(decl_macro)]
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
    /// Returns the common name of this identifier.
    fn name(&self) -> Sym;
}

pub mod convalue;

pub mod interpret;

pub mod function;

pub mod constructor {
    use cl_ast::types::Symbol as Sym;

    use crate::{
        Callable,
        convalue::ConValue,
        env::Environment,
        error::{Error, IResult},
    };

    #[derive(Clone, Copy, Debug)]
    pub struct Constructor {
        pub name: Sym,
        pub arity: u32,
    }

    impl Callable for Constructor {
        fn call(&self, _env: &mut Environment, args: &[ConValue]) -> IResult<ConValue> {
            let &Self { name, arity } = self;
            if arity as usize == args.len() {
                Ok(ConValue::TupleStruct(name, Box::new(args.into())))
            } else {
                Err(Error::ArgNumber(arity as usize, args.len()))
            }
        }

        fn name(&self) -> cl_ast::types::Symbol {
            "tuple-constructor".into()
        }
    }
}

pub mod builtin;

pub mod pattern {}

pub mod env;

pub mod modules {}

pub mod error;

#[cfg(test)]
mod tests;
