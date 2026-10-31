//! Represents a [block of code](Function) which lives inside the Interpreter

use crate::{env::Location, typeinfo::Type};

use super::{Callable, ConValue, Environment, IResult};
use cl_ast::{
    At, Bind, BindOp, DefaultTypes, Expr, Op, Pat, PatOp,
    types::Symbol,
    visit::{Visit, Walk},
};
use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    rc::Rc,
};

pub type CallableFn =
    Box<dyn Fn(Clunktion, &mut Environment, Vec<ConValue>) -> IResult<ConValue> + 'static>;

/// The inner portion of a [Clunktion]
///
/// This holds the information required to call a [CallableFn]:
/// - the [CallableFn] itself
/// - the [Symbol]s it captures
/// - the [Location] of those captures, once known
pub struct Clunk {
    pub ast: At<Expr>,
    pub expr: CallableFn,
    pub captures: BTreeSet<Symbol>,
    pub upvars: RefCell<(HashMap<Symbol, Location>, usize, Option<Type>)>,
}

#[derive(Clone)]
pub struct Clunktion(Rc<Clunk>);

impl Clunktion {
    pub fn new(ast: At<Expr>, expr: CallableFn) -> Self {
        let mut captures = Default::default();
        Captor::new(&mut captures).visit(&ast);

        Self(Rc::new(Clunk {
            expr,
            ast,
            captures,
            upvars: Default::default(),
        }))
    }

    pub fn inner(&self) -> &Clunk {
        &self.0
    }

    pub fn mark(&self, env: &Environment) -> IResult<()> {
        if let Ok(upvars) = self.0.upvars.try_borrow_mut().as_deref_mut() {
            if upvars.2.is_none() {
                upvars.2 = env.get_direct_ancestor();
            }
            self.capture_inner(env, upvars)?;
        }
        Ok(())
    }

    pub fn capture(&self, env: &Environment) -> IResult<()> {
        if let Ok(upvars) = self.0.upvars.try_borrow_mut().as_deref_mut() {
            self.capture_inner(env, upvars)?;
        }
        Ok(())
    }

    fn capture_inner(
        &self,
        env: &Environment,
        upvars: &mut (HashMap<Symbol, Location>, usize, Option<Type>),
    ) -> IResult<()> {
        let (self_upvars, depth, _module) = upvars;
        if self_upvars.len() >= self.0.captures.len() {
            return Ok(());
        }
        let frame_top = env
            .get_frame(*depth + 1)
            .map(|frame| frame.base)
            .unwrap_or_else(|| env.stack_depth());
        for &name in self.0.captures.iter() {
            let Ok(id) = env.id_of(name) else {
                continue;
            };
            if matches!(id, Location::Stack(id) if id as usize >= frame_top) {
                continue;
            }
            self_upvars.entry(name).or_insert(id);
        }
        Ok(())
    }
}
impl Callable for Clunktion {
    fn call(&self, env: &mut Environment, args: Vec<ConValue>) -> IResult<ConValue> {
        self.capture(env)?;
        (self.0.expr)(self.clone(), env, args)
    }

    fn name(&self) -> Option<Symbol> {
        None
    }
}

impl std::fmt::Debug for Clunktion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Clunktion")
            .field("ast", &self.0.ast)
            .field("captures", &self.0.captures)
            .field("upvars", &self.0.upvars)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for Clunktion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.ast.fmt(f)
    }
}

/// Finds free variables in an expression
#[derive(Debug)]
pub(crate) struct Captor<'u> {
    free: &'u mut BTreeSet<Symbol>,
    bound: BTreeSet<Symbol>,
}
impl<'u> Captor<'u> {
    pub fn new(free: &'u mut BTreeSet<Symbol>) -> Self {
        Self { free, bound: BTreeSet::new() }
    }
    pub fn enter<'v>(&'v mut self) -> Captor<'v> {
        let Self { free: upvars, bound: unfree } = self;
        Captor { free: upvars, bound: unfree.clone() }
    }

    pub fn scope<'v, R>(&'v mut self, f: impl FnOnce(&mut Captor<'v>) -> R) -> R {
        let Self { free, bound } = self;
        f(&mut Captor { free, bound: bound.clone() })
    }

    pub fn user<'v>(&'v mut self) -> Marker<'v, 'u> {
        Marker(self)
    }

    pub fn mark(&mut self, name: Symbol) -> Result<(), !> {
        let Self { free, bound } = self;
        if !bound.contains(&name) {
            free.insert(name);
        }
        Ok(())
    }

    pub fn bind(&mut self, name: Symbol) -> Result<(), !> {
        let Self { bound, .. } = self;
        bound.insert(name);
        Ok(())
    }
}

impl Visit<'_, DefaultTypes> for Captor<'_> {
    type Error = !;

    fn visit_path(&mut self, path: &'_ cl_ast::types::Path) -> Result<(), Self::Error> {
        match path.parts.as_slice() {
            &[name, ..] => self.mark(name),
            [] => Ok(()),
        }
    }

    fn visit_pat(&mut self, item: &'_ Pat<DefaultTypes>) -> Result<(), Self::Error> {
        match item {
            Pat::Name(bound) => self.bind(*bound),
            // TODO: Proper use of `user` in record patterns
            Pat::Op(PatOp::Fn, pats) if let [args, _rety] = &pats[..] => {
                self.visit(args)
                // self.user().visit(rety)
            }
            Pat::Op(PatOp::Record, pats) => self.visit(pats),
            Pat::Op(PatOp::Typed, pats) if let [pat, ty] = &pats[..] => {
                self.user().visit(ty)?;
                self.visit(pat)
            }
            _ => item.children(self),
        }
    }

    fn visit_expr(&mut self, expr: &'_ Expr<DefaultTypes>) -> Result<(), Self::Error> {
        match expr {
            Expr::Op(Op::Block | Op::If | Op::While | Op::Loop, exprs) => {
                self.scope(|scope| exprs.visit_in(scope))
            }
            _ => expr.children(self),
        }
    }

    fn visit_bind(&mut self, item: &'_ Bind<DefaultTypes>) -> Result<(), Self::Error> {
        let Bind(op, pat, exprs) = item;
        match op {
            BindOp::Let | BindOp::Type => item.children(self),
            BindOp::Fn | BindOp::Trait if let Some(name) = pat.value().name() => {
                self.bound.insert(name);
                self.scope(|scope| {
                    scope.visit(pat)?;
                    scope.visit(exprs)
                })
            }
            BindOp::Fn | BindOp::Trait => self.scope(|scope| {
                scope.visit(pat)?;
                scope.visit(exprs)
            }),
            BindOp::Mod | BindOp::Impl => {
                self.user().visit(pat);
                self.enter().visit(exprs)
            }
            // TODO: these
            BindOp::Struct => item.children(self),
            BindOp::Enum => item.children(self),
            BindOp::For => item.children(self),
        }
    }
}

/// Forcibly `mark`s usage of any symbol
#[derive(Debug)]
pub(crate) struct Marker<'l, 'u>(&'l mut Captor<'u>);
impl Visit<'_, DefaultTypes> for Marker<'_, '_> {
    type Error = !;

    fn visit_symbol(&mut self, name: &'_ Symbol) -> Result<(), Self::Error> {
        self.0.mark(*name)
    }

    fn visit_path(&mut self, path: &'_ cl_ast::types::Path) -> Result<(), Self::Error> {
        match path.parts.as_slice() {
            &[name, ..] => self.0.mark(name),
            [] => Ok(()),
        }
    }

    fn visit_pat(&mut self, item: &'_ Pat<DefaultTypes>) -> Result<(), Self::Error> {
        match item {
            Pat::Op(PatOp::PrefixGeneric, binds) if let [.., pat] = &binds[..] => {
                self.0.scope(|scope| {
                    scope.visit(binds)?; // Pass the bound names back to the Lifter
                    scope.user().visit(pat)
                })
            }
            _ => item.children(self),
        }
    }
}
