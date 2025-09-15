//! Represents a block of code which lives inside the Interpreter

use collect_upvars::collect_upvars;

use crate::error::ErrorKind;

use super::{Callable, ConValue, Environment, Error, IResult, Interpret, pattern};
use cl_ast::{Function as FnDecl, Sym};
use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    rc::Rc,
};

pub mod collect_upvars;

type Upvars = HashMap<Sym, ConValue>;

/// Represents a block of code which persists inside the Interpreter
#[derive(Clone, Debug)]
pub struct Function {
    /// Stores the contents of the function declaration
    decl: Rc<FnDecl>,
    /// Stores data from the enclosing scopes
    upvars: RefCell<Upvars>,
}

impl Function {
    pub fn new(decl: &FnDecl) -> Self {
        // let upvars = collect_upvars(decl, env);
        Self { decl: decl.clone().into(), upvars: Default::default() }
    }
    pub fn decl(&self) -> &FnDecl {
        &self.decl
    }
    pub fn upvars(&self) -> Ref<'_, Upvars> {
        self.upvars.borrow()
    }
    pub fn lift_upvars(&self, env: &Environment) {
        let upvars = collect_upvars(&self.decl, env);
        if let Ok(mut self_upvars) = self.upvars.try_borrow_mut() {
            *self_upvars = upvars;
        }
    }
}

impl Callable for Function {
    fn name(&self) -> Sym {
        let FnDecl { name, .. } = *self.decl;
        name
    }
    fn call(&self, env: &mut Environment, args: &[ConValue]) -> IResult<ConValue> {
        let FnDecl { name, gens: _, bind, body, sign: _ } = &*self.decl;

        // Check arg mapping
        let Some(body) = body else {
            return Err(Error::NotDefined(*name));
        };

        let upvars = self.upvars.take();
        let mut env = env.with_frame("upvars", upvars);

        // TODO: completely refactor data storage
        let mut frame = env.frame("fn args");
        for (name, value) in pattern::substitution(&frame, bind, ConValue::Tuple(args.into()))? {
            frame.insert(name, value);
        }
        let res = body.interpret(&mut frame);
        drop(frame);
        if let Some(upvars) = env.pop_values() {
            self.upvars.replace(upvars);
        }
        match res {
            Err(Error { kind: ErrorKind::Return(value), .. }) => Ok(value),
            Err(Error { kind: ErrorKind::Break(value), .. }) => Err(Error::BadBreak(value)),
            other => other,
        }
    }
}
