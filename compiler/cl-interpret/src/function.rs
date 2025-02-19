//! Represents a block of code which lives inside the Interpreter

use collect_upvars::collect_upvars;

use super::{Callable, ConValue, Environment, Error, IResult, Interpret};
use cl_ast::{Function as FnDecl, Param, Sym};
use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    rc::Rc,
};

pub mod collect_upvars;

type Upvars = HashMap<Sym, Option<ConValue>>;

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
    pub fn upvars(&self) -> Ref<Upvars> {
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
        let FnDecl { name, bind, body, sign: _ } = &*self.decl;

        // Check arg mapping
        if args.len() != bind.len() {
            return Err(Error::ArgNumber { want: bind.len(), got: args.len() });
        }
        let Some(body) = body else {
            return Err(Error::NotDefined(*name));
        };

        let upvars = self.upvars.take();
        env.push_frame("upvars", upvars);

        // TODO: completely refactor data storage
        let mut frame = env.frame("fn args");
        for (Param { mutability: _, name }, value) in bind.iter().zip(args) {
            frame.insert(*name, Some(value.clone()));
        }
        let res = body.interpret(&mut frame);
        drop(frame);
        if let Some((upvars, _)) = env.pop_frame() {
            self.upvars.replace(upvars);
        }
        match res {
            Err(Error::Return(value)) => Ok(value),
            Err(Error::Break(value)) => Err(Error::BadBreak(value)),
            result => result,
        }
    }
}
