//! Represents a block of code which lives inside the Interpreter

use crate::error::ErrorKind;

use super::{Callable, ConValue, Environment, Error, IResult, Interpret};
use cl_ast::{Bind, types::Symbol as Sym};
use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    rc::Rc,
};

type Upvars = HashMap<Sym, ConValue>;

/// Represents a block of code which persists inside the Interpreter
#[derive(Clone, Debug)]
pub struct Function {
    /// Stores the contents of the function declaration
    decl: Rc<Bind>,
    /// Stores data from the enclosing scopes
    upvars: RefCell<Upvars>,
}

impl Function {
    pub fn new(decl: &Bind) -> Self {
        // let upvars = collect_upvars(decl, env);
        Self { decl: decl.clone().into(), upvars: Default::default() }
    }
    pub fn decl(&self) -> &Bind {
        &self.decl
    }
    pub fn upvars(&self) -> Ref<'_, Upvars> {
        self.upvars.borrow()
    }
    pub fn lift_upvars(&self, env: &Environment) {
        // TODO: collect upvars externally. We should know them here.
        let upvars = Default::default();
        if let Ok(mut self_upvars) = self.upvars.try_borrow_mut() {
            *self_upvars = upvars;
        }
    }
}

impl Callable for Function {
    fn name(&self) -> Sym {
        todo!()
    }
    fn call(&self, env: &mut Environment, args: &[ConValue]) -> IResult<ConValue> {
        todo!()
    }
}
