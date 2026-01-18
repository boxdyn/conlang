//! A work-in-progress tree walk interpreter for Conlang
//!
//! Currently, major parts of the interpreter are not yet implemented, and major parts will never be
//! implemented in its current form. Namely, since no [ConValue] has a stable location, it's
//! meaningless to get a pointer to one, and would be undefined behavior to dereference a pointer to
//! one in any situation.
#![expect(unused, reason = "Work in progress")]

use super::*;
use cl_ast::*;

macro trace($($t:tt)*) {{
    #[cfg(debug_assertions)]
    if std::env::var("CONLANG_TRACE").is_ok() {
        eprintln!($($t)*)
    }
}}

/// A work-in-progress tree walk interpreter for Conlang
pub trait Interpret {
    /// Interprets this thing in the given [`Environment`].
    ///
    /// Everything returns a value!™
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue>;
}

impl Interpret for Expr<DefaultTypes> {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        todo!()
    }
}

impl Interpret for Pat<DefaultTypes> {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        todo!()
    }
}

impl Interpret for Bind<DefaultTypes> {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        todo!()
    }
}
