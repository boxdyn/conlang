use crate::{
    Callable,
    convalue::ConValue,
    env::Environment,
    error::{Error, ErrorKind, IResult},
    function::collect_upvars::CollectUpvars,
    interpret::Interpret,
    pattern,
};
use cl_ast::{Sym, ast_visitor::Visit};
use std::{collections::HashMap, fmt::Display};

/// Represents an ad-hoc anonymous function
/// which captures surrounding state by COPY
#[derive(Clone, Debug)]
pub struct Closure {
    decl: cl_ast::Closure,
    lift: HashMap<Sym, ConValue>,
}

impl Closure {
    const NAME: &'static str = "{closure}";
}

impl Closure {
    pub fn new(env: &mut Environment, decl: &cl_ast::Closure) -> Self {
        let lift = CollectUpvars::new(env).visit(decl).finish_copied();
        Self { decl: decl.clone(), lift }
    }
}

impl Display for Closure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { decl, lift: _ } = self;
        write!(f, "{decl}")
    }
}

impl Callable for Closure {
    fn call(&self, env: &mut Environment, args: &[ConValue]) -> IResult<ConValue> {
        let Self { decl, lift } = self;
        let mut env = env.frame(Self::NAME);

        // place lifts in scope
        for (name, value) in lift.clone() {
            env.insert(name, value);
        }

        let mut env = env.frame("args");

        for (name, value) in pattern::substitution(&env, &decl.arg, ConValue::Tuple(args.into()))? {
            env.insert(name, value);
        }

        let res = decl.body.interpret(&mut env);
        drop(env);

        match res {
            Err(Error { kind: ErrorKind::Return(value), .. }) => Ok(value),
            Err(Error { kind: ErrorKind::Break(value), .. }) => Err(Error::BadBreak(value)),
            other => other,
        }
    }

    fn name(&self) -> cl_ast::Sym {
        Self::NAME.into()
    }
}
