//! Represents a [block of code](Function) which lives inside the Interpreter

use crate::{
    error::ErrorKind,
    interpret::{Match, MatchEnv},
};

use super::{Callable, ConValue, Environment, Error, IResult, Interpret};
use cl_ast::{
    At, Bind, BindOp, DefaultTypes, Expr, Op, Pat, PatOp,
    fmt::FmtAdapter,
    types::Symbol as Sym,
    visit::{Visit, Walk},
};
use cl_structures::{intern::interned::Interned, span::Span};
use std::{
    cell::{Ref, RefCell},
    collections::{BTreeSet, HashMap},
    rc::Rc,
};

type Upvars = HashMap<Sym, usize>;

/// Represents a block of code which persists inside the Interpreter
#[derive(Clone, Debug)]
pub struct Function {
    /// Stores the contents of the function declaration
    decl: Rc<(At<Pat>, At<Expr>, BTreeSet<Sym>, RefCell<(Upvars, usize)>)>,
}

impl Function {
    pub fn new(decl: &Bind) -> Self {
        // let upvars = collect_upvars(decl, env);
        if let Bind(BindOp::Fn, pat, exprs) = decl
            && let [body] = exprs.as_slice()
        {
            let mut free = BTreeSet::new();
            Lifter::new(&mut free).visit(decl);
            Self { decl: (pat.clone(), body.clone(), free, Default::default()).into() }
        } else {
            unimplemented!()
        }
    }
    pub fn pat(&self) -> &At<Pat> {
        &self.decl.0
    }
    pub fn body(&self) -> &At<Expr> {
        &self.decl.1
    }
    pub fn span(&self) -> Span {
        self.decl.1.1
    }
    pub fn captures(&self) -> &BTreeSet<Sym> {
        &self.decl.2
    }
    pub fn upvars(&self) -> &RefCell<(Upvars, usize)> {
        &self.decl.3
    }
    pub fn lift_upvars(&self, env: &Environment) {
        // TODO: is this optional and/or necessary?
        if let Ok((self_upvars, depth)) = self.upvars().try_borrow_mut().as_deref_mut()
            && self_upvars.len() < self.captures().len() // test for missing captures
            && env.frame_depth() >= *depth
        {
            let frame_top = env
                .get_frame(*depth + 1)
                .map(|frame| frame.base)
                .unwrap_or_else(|| env.stack_depth());
            for &name in self.captures() {
                let Ok(id) = env.id_of(name) else {
                    continue;
                };
                if id >= frame_top {
                    continue;
                }
                self_upvars.entry(name).or_insert(id);
            }
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
        self.lift_upvars(env);
        let args = ConValue::Tuple(args.into());

        let pat = self.pat();
        let mut bindings = HashMap::new();
        pat.matches(args, &mut MatchEnv::new(env, &mut bindings))?;

        let mut scope = env.with_raw_frame("captures", &self.upvars().borrow().0);
        let mut scope = scope.with_frame("args", bindings);
        let mut scope = scope.frame(
            self.name().map(|name| name.to_ref()).unwrap_or("closure"),
            Some(self.span()),
        );
        match self.body().interpret(&mut scope) {
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
        let (pat, At(expr, ..), captures) = (self.pat(), self.body(), self.captures());
        if !captures.is_empty() {
            f.delimit("// captures: ", "\n").list(captures, ", ")?;
        }
        write!(f, "fn {pat}")?;
        match expr {
            Expr::Op(Op::Block, ..) => write!(f, " {expr}"),
            _ => write!(f, " = {expr}"),
        }
    }
}

#[derive(Debug)]
struct Lifter<'u> {
    free: &'u mut BTreeSet<Sym>,
    bound: BTreeSet<Sym>,
}
impl<'u> Lifter<'u> {
    pub fn new(free: &'u mut BTreeSet<Sym>) -> Self {
        Self { free, bound: BTreeSet::new() }
    }
    pub fn enter<'v>(&'v mut self) -> Lifter<'v> {
        let Self { free: upvars, bound: unfree } = self;
        Lifter { free: upvars, bound: unfree.clone() }
    }

    pub fn scope<'v, R>(&'v mut self, f: impl FnOnce(&mut Lifter<'v>) -> R) -> R {
        let Self { free, bound } = self;
        f(&mut Lifter { free, bound: bound.clone() })
    }

    pub fn user<'v>(&'v mut self) -> Marker<'v, 'u> {
        Marker(self)
    }

    pub fn mark(&mut self, name: Sym) -> Result<(), !> {
        let Self { free, bound } = self;
        if !bound.contains(&name) {
            free.insert(name);
        }
        Ok(())
    }

    pub fn bind(&mut self, name: Sym) -> Result<(), !> {
        let Self { bound, .. } = self;
        bound.insert(name);
        Ok(())
    }
}

impl Visit<'_, DefaultTypes> for Lifter<'_> {
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
            Pat::Op(PatOp::Fn, pats) if let [args, rety] = &pats[..] => {
                self.visit(args)?;
                self.user().visit(rety)
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
            BindOp::Fn if let Some(name) = pat.value().name() => {
                self.bound.insert(name);
                self.scope(|scope| {
                    scope.visit(pat)?;
                    scope.visit(exprs)
                })
            }
            BindOp::Fn => self.scope(|scope| {
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
struct Marker<'l, 'u>(&'l mut Lifter<'u>);
impl Visit<'_, DefaultTypes> for Marker<'_, '_> {
    type Error = !;

    fn visit_symbol(&mut self, name: &'_ Sym) -> Result<(), Self::Error> {
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
            Pat::Op(PatOp::PrefixGeneric, binds) if let [bound @ .., pat] = &binds[..] => {
                self.0.scope(|scope| {
                    scope.visit(binds)?; // Pass the bound names back to the Lifter
                    scope.user().visit(pat)
                })
            }
            _ => item.children(self),
        }
    }
}
