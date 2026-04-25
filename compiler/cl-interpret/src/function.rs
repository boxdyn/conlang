//! Represents a block of code which lives inside the Interpreter

use crate::{
    error::ErrorKind,
    interpret::{Match, MatchEnv},
};

use super::{Callable, ConValue, Environment, Error, IResult, Interpret};
use cl_ast::{At, Bind, BindOp, Expr, Op, Pat, PatOp, types::Symbol as Sym};
use cl_structures::{intern::interned::Interned, span::Span};
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
    decl: Rc<(At<Pat>, At<Expr>)>,
    /// Stores data from the enclosing scopes
    upvars: RefCell<Upvars>,
}

impl Function {
    pub fn new(decl: &Bind) -> Self {
        // let upvars = collect_upvars(decl, env);
        if let Bind(BindOp::Fn, _, pat, exprs) = decl
            && let [body] = exprs.as_slice()
        {
            Self { decl: (pat.clone(), body.clone()).into(), upvars: Default::default() }
        } else {
            unimplemented!()
        }
    }
    pub fn decl(&self) -> &(At<Pat>, At<Expr>) {
        &self.decl
    }
    pub fn span(&self) -> Span {
        self.decl.1.1
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
    fn name(&self) -> Option<Sym> {
        fn get_name(pat: &Pat) -> Option<Sym> {
            match pat {
                Pat::Name(name) => Some(*name),
                Pat::Op(PatOp::Tuple | PatOp::Slice | PatOp::Fn, _) => None,
                Pat::Op(_op, pats) => pats.iter().find_map(|At(pat, ..)| get_name(pat)),
                _ => None,
            }
        }
        get_name(self.decl.0.value())
    }
    fn call(&self, env: &mut Environment, args: &[ConValue]) -> IResult<ConValue> {
        let args = ConValue::Tuple(args.into());
        let (pat, body) = self.decl();

        let mut bindings = HashMap::new();
        pat.matches(args, &mut MatchEnv::new(env, &mut bindings))?;

        let mut scope = env.with_frame("args", bindings);
        let mut scope = scope.frame(
            self.name().map(|name| name.to_ref()).unwrap_or("closure"),
            Some(self.span()),
        );
        match body.interpret(&mut scope) {
            Err(Error { kind: ErrorKind::Panic(e, depth), span }) => {
                println!("{depth:>4}: {pat} at {}", span.unwrap_or(self.span()));
                Err(Error { kind: ErrorKind::Panic(e, depth + 1), span: None })
            }
            Err(Error { kind: ErrorKind::Return(value), .. }) => Ok(value),
            other => other,
        }
    }
}

impl std::fmt::Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (pat, At(expr, ..)) = self.decl();
        write!(f, "fn {pat}")?;
        match expr {
            Expr::Op(Op::Block, ..) => write!(f, " {expr}"),
            _ => write!(f, " = {expr}"),
        }
    }
}
