//! Represents a block of code which lives inside the Interpreter

use super::{Callable, ConValue, Environment, Error, IResult, Interpret};
use cl_ast::{Function as FnDecl, Param, Sym};
use std::rc::Rc;
/// Represents a block of code which persists inside the Interpreter
#[derive(Clone, Debug)]
pub struct Function {
    /// Stores the contents of the function declaration
    decl: Rc<FnDecl>,
    // /// Stores the enclosing scope of the function
    // env: Box<Environment>,
}

impl Function {
    pub fn new(decl: &FnDecl) -> Self {
        Self { decl: decl.clone().into() }
    }
    pub fn decl(&self) -> &FnDecl {
        &self.decl
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
        // TODO: completely refactor data storage
        let mut frame = env.frame("fn args");
        for (Param { mutability: _, name }, value) in bind.iter().zip(args) {
            frame.insert(*name, Some(value.clone()));
        }
        match body.interpret(&mut frame) {
            Err(Error::Return(value)) => Ok(value),
            Err(Error::Break(value)) => Err(Error::BadBreak(value)),
            result => result,
        }
    }
}
