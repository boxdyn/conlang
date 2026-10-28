//! A Conlang interpreter which "compiles" to a closure

use crate::{
    Callable,
    convalue::{ConValue, Integer},
    env::{Environment, Location},
    error::{Error, ErrorKind, IResult},
    function::Captor,
    interpret::{bind_enum, bind_struct, cl_todo},
    place::Place,
    typeinfo::{Model, Type},
};
use cl_ast::{types::*, visit::Visit, *};
use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    iter,
    rc::Rc,
    sync::Arc,
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
impl std::fmt::Debug for Clunktion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Clunktion").field(&self.0.ast).finish()
    }
}
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

    pub fn mark(&self, env: &Environment) -> IResult<()> {
        if let Ok(upvars) = self.0.upvars.try_borrow_mut().as_deref_mut() {
            if upvars.2.is_none() {
                upvars.2 = env.get_module();
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
impl std::fmt::Display for Clunktion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.ast.fmt(f)
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

pub struct ScopeFrame {
    /// The bodies of `defer` exprs seen in this scope
    pub deferred: Vec<At<Expr>>,
    pub stack_allocations: usize,
    pub names: HashMap<Symbol, Location>,
}

pub struct Scopes {
    pub stack: Vec<ScopeFrame>,
    pub bodies: Vec<OwnClexpr>,
}

// this could likely be allocated as &'arena dyn Fn(&mut Environment)... + 'arena
pub type Clexpr = dyn Fn(&mut Environment) -> IResult<ConValue>;
pub type OwnClexpr = Box<Clexpr>;
pub type Plexpr = Box<dyn Fn(&mut Environment) -> IResult<Place> + 'static>;

pub fn compile(expr: &At<Expr>) -> IResult<OwnClexpr> {
    compile_inner(expr).map_err(|e| e.with_span(expr.1))
}

fn compile_inner(expr: &At<Expr>) -> IResult<OwnClexpr> {
    let compute: OwnClexpr = match expr.inner().value() {
        // TODO: don't clone?
        _ if let Some(place) = compile_place(expr)? => Box::new(move |e| place(e)?.get(e).cloned()),
        Expr::Omitted => Box::new(|_| Ok(ConValue::Unit)),
        Expr::Id(path) => compile_path(&path.parts, expr),
        Expr::MetId(_) => unimplemented!("Expr::MetId"),
        Expr::Lit(lit) => compile_lit(lit, expr)?,
        Expr::Use(tree) => compile_use(tree, expr)?,
        Expr::Bind(bind) => compile_bind(bind, expr)?,
        Expr::Make(make) => compile_make(make, expr)?,
        Expr::Match(mtch) => compile_match(mtch, expr)?,
        Expr::Label(label) => compile_label(label, expr)?,
        Expr::Op(op, exprs) => compile_op(*op, exprs, expr)?,
    };

    Ok(compute)
    // let span = expr.1;
    // Ok(Box::new(move |e| compute(e).map_err(|e| e.with_span(span))))
}

pub fn compile_place(value: &At<Expr>) -> IResult<Option<Plexpr>> {
    Ok(match value.value() {
        Expr::Id(Path { parts }) if let &[name] = parts.as_slice() => {
            Some(Box::new(move |e| Ok(Place::from_index(e.id_of(name)?))))
        }
        Expr::Op(op, exprs) => match (op, exprs.as_slice()) {
            (Op::Group, [e]) => compile_place(e)?,
            (Op::MetaInner | Op::MetaOuter, [_, e]) => compile_place(e)?,
            (Op::Pub, [e]) => compile_place(e)?,
            (Op::Identity, [e]) => compile_place(e)?,
            (Op::Index, [place, idx]) if let Some(place) = compile_place(place)? => {
                let index = compile(idx)?;
                Some(Box::new(move |e| match index(e)? {
                    ConValue::Int(i @ ..0) => Ok(place(e)?.index(-i as _, true)),
                    ConValue::Int(i @ 0..) => Ok(place(e)?.index(i as _, false)),
                    other => Err(Error::TypeError("{integer}", other.type_of(e))),
                }))
            }
            (Op::Const | Op::Static, _) => None,
            (Op::Dot, &[ref place, At(Expr::Lit(Literal::Int(i, _)), _)]) => {
                let Some(place) = compile_place(place)? else {
                    return Ok(None);
                };
                Some(Box::new(move |e| Ok(place(e)?.dot_idx(i as _))))
            }
            (Op::Dot, [place, At(Expr::Id(path), _)]) if path.parts.len() == 1 => {
                let Some(place) = compile_place(place)? else {
                    return Ok(None);
                };
                let name = path.parts[0];
                Some(Box::new(move |e| Ok(place(e)?.dot_sym(name))))
            }
            (Op::Deref, [ptr]) => {
                let ptr = compile(ptr)?;
                Some(Box::new(move |e| match ptr(e)? {
                    ConValue::Ref(place) => Ok(place),
                    other => Err(Error::TypeError("&", other.type_of(e))),
                }))
            }
            _ => None,
        },
        _ => None,
    })
}

fn compile_path(parts: &[Symbol], _within: &At<Expr>) -> OwnClexpr {
    match parts {
        &[name] => Box::new(move |e| e.get(name)),
        &[first, ref rest @ ..] => {
            let mut resolve: OwnClexpr = Box::new(move |e| e.get(first));
            for &name in rest {
                resolve = Box::new(move |e| match resolve(e)? {
                    ConValue::TypeInfo(ty) => e.get_impl(ty, name),
                    value => cl_todo!("{value}::{name}")?,
                })
            }
            resolve
        }
        [] => unimplemented!("Empty paths"),
    }
}

pub fn compile_lit(lit: &Literal, _within: &At<Expr>) -> IResult<OwnClexpr> {
    Ok(match lit {
        &Literal::Bool(b) => Box::new(move |_| Ok(ConValue::Bool(b))),
        &Literal::Char(c) => Box::new(move |_| Ok(ConValue::Char(c))),
        &Literal::Int(i, _) => {
            let i = i as Integer;
            Box::new(move |_| Ok(ConValue::Int(i)))
        }
        Literal::Str(s) => {
            let s = Symbol::from(s.as_str());
            Box::new(move |_| Ok(ConValue::Str(s)))
        }
    })
}

pub fn compile_use(tree: &Use, _within: &At<Expr>) -> IResult<OwnClexpr> {
    type UseTy = Box<dyn Fn(&mut Environment, Type) -> IResult<()>>;
    fn compile_use_ty(tree: &Use, _within: &At<Expr>) -> UseTy {
        fn bind(e: &mut Environment, ty: Type, from: Symbol, to: Symbol) -> IResult<()> {
            match e.get_impl_id(ty, from) {
                Some(id) => {
                    e.bind_raw(to, id);
                }
                None => {
                    let value = ty.getattr(from)?;
                    e.bind(to, value);
                }
            }
            Ok(())
        }
        match tree {
            Use::Glob => Box::new(|e, ty| {
                let impls = (e.impls.get(&ty)).ok_or_else(|| Error::TypeError("impls", ty))?;
                for (name, id) in impls.clone() {
                    e.bind_raw(name, Location::Global(id));
                }
                Ok(())
            }),
            &Use::Name(name) => Box::new(move |e, ty| bind(e, ty, name, name)),
            &Use::Alias(from, to) => Box::new(move |e, ty| bind(e, ty, from, to)),
            &Use::Path(name, ref tree) => {
                let tree = compile_use_ty(&tree, _within);
                Box::new(
                    move |e, ty| match e.get_impl(ty, name)?.dereference_in(e)? {
                        &ConValue::TypeInfo(ty) => tree(e, ty),
                        other => Err(Error::TypeError("type", other.type_of(e))),
                    },
                )
            }
            Use::Tree(items) => {
                let items: Vec<_> = items.iter().map(|u| compile_use_ty(u, _within)).collect();
                Box::new(move |e, ty| {
                    for item in items.iter() {
                        item(e, ty)?;
                    }
                    Ok(())
                })
            }
        }
    }

    Ok(match tree {
        Use::Glob => Box::new(move |_| Ok(ConValue::Unit)),
        &Use::Name(name) => Box::new(move |e| {
            let id = e.id_of(name)?;
            e.bind_raw(name, id);
            Ok(ConValue::Unit)
        }),
        &Use::Alias(from, to) => Box::new(move |e| {
            let id = e.id_of(from)?;
            e.bind_raw(to, id);
            Ok(ConValue::Unit)
        }),
        &Use::Path(name, ref tree) => {
            let tree = compile_use_ty(tree, _within);
            Box::new(move |e| match e.get(name)? {
                ConValue::TypeInfo(ty) => tree(e, ty).map(|_| ConValue::Unit),
                other => Err(Error::TypeError("type", other.type_of(e))),
            })
        }
        Use::Tree(items) => {
            let items = items
                .iter()
                .map(|tree| compile_use(tree, _within))
                .collect::<Result<Vec<_>, _>>()?;
            Box::new(move |e| {
                for item in items.iter() {
                    item(e)?;
                }
                Ok(ConValue::Unit)
            })
        }
    })
}

fn compile_bind(bind: &Bind, within: &At<Expr>) -> IResult<OwnClexpr> {
    let Bind(op, pat, exprs) = bind;
    let exprs = exprs.as_slice();
    Ok(match (op, pat, exprs) {
        (BindOp::Let, _, []) => cl_todo!("let {pat}")?,
        (BindOp::Let, _, [scrutinee]) => {
            let scrutinee = compile(scrutinee)?;
            let (it_matches, then_bind) = compile_let_pattern(pat)?;
            match compile_guard_patterns(pat)? {
                Some(try_guards) => Box::new(move |e| {
                    let value = scrutinee(e)?;
                    if it_matches(e, &value)? {
                        // TODO: Frame.forget() to merge frames
                        let mut scope = e.frame("if_guard", None, None);
                        then_bind(&mut scope, value);
                        if try_guards(&mut scope)? {
                            scope.forget();
                            return Ok(ConValue::Bool(true));
                        }
                    }
                    Ok(ConValue::Bool(false))
                }),
                None => Box::new(move |e| {
                    let value = scrutinee(e)?;
                    let success = it_matches(e, &value)?;
                    if success {
                        then_bind(e, value);
                    }
                    Ok(ConValue::Bool(success))
                }),
            }
        }
        (BindOp::Let, _, [scrutinee, default]) => {
            let (scrutinee, default) = (compile(scrutinee)?, compile(default)?);
            let (it_matches, then_bind) = compile_let_pattern(pat)?;
            match compile_guard_patterns(pat)? {
                Some(guards) => Box::new(move |e| {
                    let value = scrutinee(e)?;
                    if it_matches(e, &value)? {
                        // TODO: Frame.forget() to merge frames
                        then_bind(e, value);
                        if guards(e)? {
                            return Ok(ConValue::Unit);
                        }
                    }
                    let value = default(e)?;
                    if it_matches(e, &value)? {
                        then_bind(e, value);
                        if guards(e)? {
                            return Ok(ConValue::Unit);
                        }
                    } else {
                        Err(Error::MatchNonexhaustive(value))?
                    }

                    Err(Error::Panic("oops".into()))
                }),
                None => Box::new(move |e| {
                    let value = scrutinee(e)?;
                    if it_matches(e, &value)? {
                        then_bind(e, value);
                        return Ok(ConValue::Unit);
                    }
                    let value = default(e)?;
                    if it_matches(e, &value)? {
                        then_bind(e, value);
                        return Ok(ConValue::Unit);
                    }
                    Err(Error::MatchNonexhaustive(value))
                }),
            }
        }
        (BindOp::Type, _, []) => cl_todo!("type {pat}")?,
        (BindOp::Type, _, [alias]) => cl_todo!("type {pat} = {alias}")?,
        (BindOp::Fn, pat, [body]) => {
            // TODO: captures?
            let body_span = body.1;
            let body = match body.inner() {
                At(Expr::Op(Op::Block, b), _) if let [body] = b.as_slice() => body,
                _ => body,
            };
            let body = compile(body)?;
            let inner_pat = pat.0.inner();
            let (it_matches, then_bind) = let_inner_pattern(inner_pat)?;
            if compile_guard_patterns(pat)?.is_some() {
                cl_todo!("Guard patterns on function args")?
            }

            let body: CallableFn = Box::new(move |f, e, args| {
                let upvars = f.0.upvars.borrow();
                let mut scope = e.frame("captures", Some(body_span), upvars.2);
                drop(upvars);

                let mut scope = scope.frame("body", Some(body_span), None);
                let args = ConValue::Tuple(args);
                if !it_matches(&mut scope, &args)? {
                    Err(Error::Panic("args failed".into()))?
                }
                then_bind(&mut scope, args);

                match body(&mut scope) {
                    Err(Error { kind: ErrorKind::Panic(e, depth), span }) => {
                        println!("{depth:>4}: at {}", span.unwrap_or(body_span));
                        Err(Error { kind: ErrorKind::Panic(e, depth + 1), span: None })
                    }
                    Err(Error { kind: ErrorKind::Return(value), .. }) => Ok(value),
                    other => other,
                }
            });

            let clunk = Clunktion::new(within.clone(), body);

            match pat.0.name() {
                Some(name) => Box::new(move |e| {
                    clunk.mark(e)?;
                    e.bind(name, ConValue::Clunktion(clunk.clone()));
                    Ok(ConValue::Clunktion(clunk.clone()))
                }),
                None => Box::new(move |_| Ok(ConValue::Clunktion(clunk.clone()))),
            }
        }
        // TODO: perform a naming pass, then do this
        (BindOp::Mod, _, [body]) => compile_impl(pat, body, false)?,
        (BindOp::Impl, _, [body]) => compile_impl(pat, body, true)?,
        (BindOp::Impl, _, []) => compile_impl(pat, &Expr::Omitted.at(within.1), true)?,
        (BindOp::Struct, _, []) => {
            let pat = pat.clone();
            Box::new(move |e| match bind_struct(pat.value(), e)? {
                (Some(name), ty) => {
                    let deftype = e.def_type(name, ty.intern());
                    e.bind(name, ConValue::TypeInfo(deftype));
                    Ok(ConValue::TypeInfo(deftype))
                }
                (None, ty) => Ok(ConValue::TypeInfo(ty.intern())),
            })
        }
        (BindOp::Enum, &At(Pat::Name(name), _), []) => {
            let never = Model::Never.intern();
            Box::new(move |e| {
                e.def_type(name, never);
                e.bind(name, never);
                Ok(ConValue::TypeInfo(never))
            })
        }
        (BindOp::Enum, _, []) => {
            let pat = pat.value().clone();
            Box::new(move |e| bind_enum(&pat, e))
        }
        _ => unimplemented!("{bind}"),
    })
}

fn compile_impl(pat: &At<Pat>, body: &At<Expr>, is_impl: bool) -> IResult<OwnClexpr> {
    fn find_body(body: &At<Expr>) -> &At<Expr> {
        let At(Expr::Op(op, exprs), _) = body else {
            return body;
        };
        match (op, exprs.as_slice()) {
            (Op::Block, [body]) => body,
            (Op::Pub | Op::Group, [body]) => find_body(body),
            (Op::MetaInner | Op::MetaOuter, [_, body]) => find_body(body),
            _ => body,
        }
    }
    let body = find_body(body);
    let body = compile(body)?;
    let pat = pat.clone();
    if is_impl {
        Ok(Box::new(move |e: &mut Environment| {
            let selfty = e.kw.kw_Self;
            let (name, model) = bind_struct(pat.value(), e)?;
            let ty = model.intern(); // TODO: bad
            let mut scope = e.frame(ty.name(), Some(pat.1), Some(ty));
            let bound = scope.bind_global(selfty, ty);
            if let Some(name) = name {
                scope.bind_raw(name, bound);
            }
            body(&mut scope)?;
            if let Some(values) = scope.pop_into_globals() {
                for (name, value) in values {
                    if let Location::Global(loc) = value {
                        e.implement_raw(ty, name, loc);
                        if let Model::Enum(_, variants) = ty.to_ref() {
                            for &(_, variant) in variants {
                                e.implement_raw(variant, name, loc);
                            }
                        }
                        match e.get_id(value).expect("Just bound") {
                            ConValue::Clunktion(c) => c.clone().capture(e)?,
                            _ => {}
                        }
                    }
                }
            }
            Ok(ConValue::TypeInfo(ty))
        }))
    } else {
        Ok(Box::new(move |e| {
            let (kw_self, kw_super) = (e.kw.kw_self, e.kw.kw_super);
            let (name, model) = bind_struct(pat.value(), e)?;
            let ty = model.intern();
            let upper = e.get_module();
            let me = e
                .get(e.kw.modules_enabled)
                .and_then(|v| v.truthy(e))
                .unwrap_or(false);
            if !me {
                return body(e);
            };

            let mut scope = e.frame(ty.name(), Some(pat.1), Some(ty));
            scope.bind(kw_self, ty);
            scope.bind(kw_super, if let Some(m) = upper { m } else { ty });
            body(&mut scope)?;
            if let Some(values) = scope.pop_into_globals() {
                for (name, value) in values {
                    if let Location::Global(id) = value {
                        e.implement_raw(ty, name, id);
                    }
                }
            }
            if let Some(name) = name {
                e.bind(name, ConValue::TypeInfo(ty));
            }
            Ok(ConValue::TypeInfo(ty))
        }))
    }
}

fn compile_make(make: &Make, _within: &At<Expr>) -> IResult<OwnClexpr> {
    let Make(ty, arm_exprs) = make;
    let ty = compile(ty)?;
    let mut arms: Vec<(Symbol, OwnClexpr)> = vec![];
    for &MakeArm(name, ref value) in arm_exprs {
        arms.push(match value {
            Some(value) => (name, compile(value)?),
            None => (name, Box::new(move |e| e.get(name))),
        });
    }
    Ok(Box::new(move |e| {
        let tyinfo = match ty(e)? {
            ConValue::TypeInfo(info) => info,
            other => Err(Error::TypeError("type", other.type_of(e)))?,
        };

        let mut values = HashMap::new();
        for &(name, ref value) in arms.iter() {
            values.insert(name, value(e)?);
        }

        tyinfo.make_struct(values)
    }))
}

fn compile_match(mtch: &Match, _within: &At<Expr>) -> IResult<OwnClexpr> {
    let Match(scrutinee, match_arms) = mtch;
    let scrutinee = compile(scrutinee)?;
    let mut matchers = vec![];
    let mut then_bind = vec![];
    let mut guard_clauses = vec![];
    let mut and_run = vec![];
    for MatchArm(pat, expr) in match_arms {
        // TODO: refactor pattern matching to allow nested alts
        match pat {
            At(Pat::Op(PatOp::Alt, alts), _) => {
                for pat in alts {
                    let (matcher, binder) = compile_let_pattern(pat)?;
                    matchers.push(matcher);
                    then_bind.push(binder);
                    guard_clauses.push(compile_guard_patterns(pat)?);
                    and_run.push(compile(expr)?);
                }
            }
            pat => {
                let (matcher, binder) = compile_let_pattern(pat)?;
                matchers.push(matcher);
                then_bind.push(binder);
                guard_clauses.push(compile_guard_patterns(pat)?);
                and_run.push(compile(expr)?);
            }
        }
    }

    Ok(Box::new(move |e| {
        let scrutinee = scrutinee(e)?;

        // TODO: optimized match trees
        for (idx, matcher) in matchers.iter().enumerate() {
            let _ = matcher(e, &scrutinee)? || continue;
            let mut scope = e.frame("match-arm", None, None);
            match &guard_clauses[idx] {
                None => then_bind[idx](&mut scope, scrutinee),
                Some(guard) => {
                    // Guard patterns must be evaluated after names are bound,
                    // but doing so prevents us from matching subsequent arms.
                    // as such, we clone the scrutinee :(
                    then_bind[idx](&mut scope, scrutinee.clone());
                    if !guard(&mut scope)? {
                        continue;
                    }
                }
            }
            return and_run[idx](&mut scope);
        }
        Err(Error::MatchNonexhaustive(scrutinee))
    }))
}

fn compile_label(label: &Label, _within: &At<Expr>) -> IResult<OwnClexpr> {
    let Label(label, body) = label;
    let label = label.to_ref();
    let body = compile(body)?;
    Ok(Box::new(move |e| {
        body(e).or_else(|err| err.catch_a_break(label))
    }))
}

fn compile_op(op: Op, exprs: &[At<Expr>], _within: &At<Expr>) -> IResult<OwnClexpr> {
    Ok(match (op, exprs) {
        (Op::Do, []) => Box::new(|_| Ok(ConValue::Unit)),
        (Op::Do, [discard @ .., ret]) => {
            let discard = discard.iter().map(compile).collect::<Result<Vec<_>, _>>()?;
            let ret = compile(ret)?;
            Box::new(move |e| {
                for expr in discard.iter() {
                    expr(e)?;
                }
                ret(e)
            })
        }
        (Op::As, [value, ty]) => {
            let value = compile(value)?;
            let ty = compile(ty)?;
            Box::new(move |e| match ty(e)? {
                ConValue::TypeInfo(ty) => Ok(value(e)?.cast(&ty, e)),
                other => Err(Error::TypeError("type", other.type_of(e))),
            })
        }
        (Op::Block, []) => Box::new(|_| Ok(ConValue::Unit)),
        (Op::Block, [expr]) => {
            let body_span = expr.1;
            let body = compile(expr)?;
            Box::new(move |e| body(&mut e.frame("block", Some(body_span), None)))
        }
        (Op::Array, []) => Box::new(|_| Ok(ConValue::Array(vec![]))),
        (Op::Array, exprs) => {
            let exprs = exprs.iter().map(compile).collect::<Result<Vec<_>, _>>()?;
            Box::new(move |env| {
                let values = exprs.iter().map(|f| f(env)).collect::<Result<_, _>>()?;
                Ok(ConValue::Array(values))
            })
        }
        (Op::ArRep, [array, repeat]) => {
            let array = compile(array)?;
            let repeat = compile(repeat)?;
            Box::new(move |e| {
                let v = array(e)?;
                let rep = repeat(e)?;
                let ConValue::Int(rep) = rep else {
                    Err(Error::TypeError("int", rep.type_of(e)))?
                };
                Ok(ConValue::Array(vec![v; rep as usize]))
            })
        }
        (Op::Group, [body]) => compile(body)?,
        (Op::Tuple, []) => Box::new(|_| Ok(ConValue::Unit)),
        (Op::Tuple, exprs) => {
            let exprs = exprs.iter().map(compile).collect::<Result<Vec<_>, _>>()?;
            Box::new(move |e| {
                let values = exprs.iter().map(|v| v(e)).collect::<Result<_, _>>()?;
                Ok(ConValue::Tuple(values))
            })
        }
        // TODO: capture meta, and add it to binds
        (Op::MetaInner, [_, body]) => compile(body)?,
        (Op::MetaOuter, [_, body]) => compile(body)?,
        (Op::Try, [expr]) => {
            let expr = compile(expr)?;
            Box::new(move |e| expr(e)?.try_hard(e))
        }
        (Op::Index, [expr, idx]) => {
            let (expr, idx) = (compile(expr)?, compile(idx)?);
            Box::new(move |e| expr(e)?.index(idx(e)?, e))
        }
        (Op::Call, [callee, At(Expr::Op(Op::Tuple, args), _)]) if args.is_empty() => {
            match compile_place(callee)? {
                Some(callee) => Box::new(move |e| callee(e)?.call(e, vec![])),
                None => {
                    let callee = compile(callee)?;
                    Box::new(move |e| callee(e)?.call(e, vec![]))
                }
            }
        }
        (Op::Call, [callee, At(Expr::Op(Op::Tuple, args), _)]) => {
            let args: Vec<_> = args.iter().map(compile).collect::<Result<Vec<_>, _>>()?;
            match compile_place(callee)? {
                Some(callee) => Box::new(move |e| {
                    let args = args.iter().map(|v| v(e)).collect::<Result<_, _>>()?;
                    callee(e)?.call(e, args)
                }),
                None => {
                    let callee = compile(callee)?;
                    Box::new(move |e| {
                        let args = args.iter().map(|v| v(e)).collect::<Result<_, _>>()?;
                        callee(e)?.call(e, args)
                    })
                }
            }
        }
        (Op::Pub | Op::Const | Op::Static, [body]) => compile(body)?,
        (Op::Macro, _) => cl_todo!("Macros in the interpreter")?,
        (Op::Quote, [expr]) => {
            let expr = Box::new(expr.clone());
            Box::new(move |_| Ok(ConValue::Quote(expr.clone())))
        }
        (Op::Loop, [body]) => {
            let span = body.1;
            let body = compile(body)?;
            Box::new(move |e| {
                loop {
                    match body(&mut e.frame("loop", Some(span), None)) {
                        Ok(_) | Err(Error { kind: ErrorKind::Continue, .. }) => {}
                        Err(e) => Err(e)?,
                    }
                }
            })
        }
        (Op::If, [cond, pass, fail]) => {
            let (cond, pass, fail) = (compile(cond)?, compile(pass)?, compile(fail)?);
            Box::new(move |e| if cond(e)?.truthy(e)? { pass(e) } else { fail(e) })
        }
        (Op::Defer, _) => cl_todo!("defer in the interpreter's 'compiler'")?,
        (Op::Break, [At(Expr::Label(label), _)]) => {
            let Label(label, body) = label.as_ref();
            let body = compile(body)?;
            let label = label.to_ref();
            Box::new(move |e| Err(Error::BreakLabel(label, body(e)?)))
        }
        (Op::Break, [body]) => {
            let body = compile(body)?;
            Box::new(move |e| Err(Error::Break(body(e)?)))
        }
        (Op::Return, [body]) => {
            let body = compile(body)?;
            Box::new(move |e| Err(Error::Return(body(e)?)))
        }
        (Op::Continue, _) => Box::new(|_| Err(Error::Continue())),
        (Op::Dot, [object, At(Expr::Op(Op::Call, args), _)]) => match args.as_slice() {
            // Case 1: A path with one part can be looked up as a member function
            [
                At(Expr::Id(Path { parts }), _),
                At(Expr::Op(Op::Tuple, args), _),
            ] if let [name] = parts[..] => {
                let args: Vec<_> = args.iter().map(compile).collect::<Result<Vec<_>, _>>()?;
                // Optimization?: if `object` is a place, pass it by reference
                match compile_place(object)? {
                    Some(place) => Box::new(move |e| {
                        let object = place(e)?;
                        let callee = e
                            .get_impl(object.get(e)?.type_of(e), name)
                            .or_else(|_| e.get(name))?;
                        let args = iter::once(Ok(ConValue::Ref(object)))
                            .chain(args.iter().map(|f| f(e)))
                            .collect::<Result<_, _>>()?;
                        callee.call(e, args)
                    }),
                    None => {
                        let object = compile(object)?;
                        Box::new(move |e| {
                            let object = object(e)?;
                            let callee = e
                                .get_impl(object.type_of(e), name)
                                .or_else(|_| e.get(name))?;
                            let args = iter::once(Ok(object))
                                .chain(args.iter().map(|f| f(e)))
                                .collect::<Result<_, _>>()?;
                            callee.call(e, args)
                        })
                    }
                }
            }
            // Case 2: Anything else gets sentenced to Gay Value Jail
            [callee, At(Expr::Op(Op::Tuple, exprs), _)] => {
                let callee = compile(callee)?;
                let args: Vec<_> = exprs.iter().map(compile).collect::<Result<Vec<_>, _>>()?;
                // Optimization?: if `object` is a place, pass it by reference
                match compile_place(object)? {
                    Some(place) => Box::new(move |e| {
                        let object = place(e)?;
                        let args = iter::once(Ok(ConValue::Ref(object)))
                            .chain(args.iter().map(|f| f(e)))
                            .collect::<Result<_, _>>()?;
                        callee(e)?.call(e, args)
                    }),
                    None => {
                        let object = compile(object)?;
                        Box::new(move |e| {
                            let object = object(e)?;
                            let args = iter::once(Ok(object))
                                .chain(args.iter().map(|f| f(e)))
                                .collect::<Result<_, _>>()?;
                            callee(e)?.call(e, args)
                        })
                    }
                }
            }
            _ => cl_todo!("{_within}")?,
        },
        (
            Op::Dot,
            [
                At(Expr::Lit(Literal::Int(whole, _)), _),
                At(Expr::Lit(Literal::Int(frac, _)), span),
            ],
        ) => {
            let value = format!("{whole}.{frac:00$}", span.len()).parse().unwrap();
            Box::new(move |_| Ok(ConValue::Float(value)))
        }
        (Op::Dot, [scrutinee, At(Expr::Lit(Literal::Int(idx, _)), _)]) => {
            cl_todo!("{scrutinee}.{idx}")?
        }
        (Op::Dot, [scrutinee, At(Expr::Id(path), _)]) if path.parts.len() == 1 => {
            cl_todo!("{scrutinee}.{}", path.parts[0])?
        }
        (Op::Dot, _) => cl_todo!(".")?,
        (Op::RangeEx, [rhs]) => {
            let rhs = compile(rhs)?;
            Box::new(move |e| {
                let ty = e.get_type_or_err(e.kw.RangeTo)?;
                Ok(ConValue::TupleStruct(ty, vec![rhs(e)?]))
            })
        }
        (Op::RangeEx, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| {
                let ty = e.get_type_or_err(e.kw.RangeExc)?;
                Ok(ConValue::TupleStruct(ty, vec![lhs(e)?, rhs(e)?]))
            })
        }
        (Op::RangeIn, [rhs]) => {
            let rhs = compile(rhs)?;
            Box::new(move |e| {
                let ty = e.get_type_or_err(e.kw.RangeToInc)?;
                Ok(ConValue::TupleStruct(ty, vec![rhs(e)?]))
            })
        }
        (Op::RangeIn, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| {
                let ty = e.get_type_or_err(e.kw.RangeInc)?;
                Ok(ConValue::TupleStruct(ty, vec![lhs(e)?, rhs(e)?]))
            })
        }
        (Op::Neg, [body]) => {
            let body = compile(body)?;
            Box::new(move |e| body(e)?.neg(e))
        }
        (Op::Not, [body]) => {
            let body = compile(body)?;
            Box::new(move |e| body(e)?.not(e))
        }
        (Op::Identity, [body]) => compile(body)?,
        (Op::Refer, [body]) if let Some(place) = compile_place(body)? => {
            Box::new(move |e| Ok(ConValue::Ref(place(e)?)))
        }
        (Op::Deref, _) => cl_todo!()?,
        (Op::Mul, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| lhs(e)?.mul(rhs(e)?, e))
        }
        (Op::Div, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| lhs(e)?.div(rhs(e)?, e))
        }
        (Op::Rem, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| lhs(e)?.rem(rhs(e)?, e))
        }
        (Op::Add, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| lhs(e)?.add(rhs(e)?, e))
        }
        (Op::Sub, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| lhs(e)?.sub(rhs(e)?, e))
        }
        (Op::Shl, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| lhs(e)?.shl(rhs(e)?, e))
        }
        (Op::Shr, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| lhs(e)?.shr(rhs(e)?, e))
        }
        (Op::And, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| lhs(e)?.and(rhs(e)?, e))
        }
        (Op::Xor, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| lhs(e)?.xor(rhs(e)?, e))
        }
        (Op::Or, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| lhs(e)?.or(rhs(e)?, e))
        }
        (Op::Lt, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| lhs(e)?.lt(&rhs(e)?, e))
        }
        (Op::Leq, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| lhs(e)?.lt_eq(&rhs(e)?, e))
        }
        (Op::Eq, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| lhs(e)?.eq(&rhs(e)?, e))
        }
        (Op::Neq, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| lhs(e)?.neq(&rhs(e)?, e))
        }
        (Op::Geq, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| lhs(e)?.gt_eq(&rhs(e)?, e))
        }
        (Op::Gt, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| lhs(e)?.gt(&rhs(e)?, e))
        }
        (Op::LogAnd, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| {
                let lhs = lhs(e)?;
                if lhs.truthy(e)? { rhs(e) } else { Ok(lhs) }
            })
        }
        (Op::LogOr, [lhs, rhs]) => {
            let (lhs, rhs) = (compile(lhs)?, compile(rhs)?);
            Box::new(move |e| {
                let lhs = lhs(e)?;
                if lhs.truthy(e)? { Ok(lhs) } else { rhs(e) }
            })
        }
        (Op::Set, [place, value]) => {
            type SetFn = Box<dyn Fn(&mut Environment, ConValue) -> IResult<()>>;
            fn set_pattern(place: &At<Expr>) -> IResult<SetFn> {
                Ok(match place {
                    place if let Some(place) = compile_place(place)? => {
                        Box::new(move |e, value| Ok(*(place(e)?.get_mut(e)?) = value))
                    }
                    At(Expr::Op(Op::Group, places), _) if let [place] = &places[..] => {
                        set_pattern(place)?
                    }
                    At(Expr::Op(Op::Array, places), _) => {
                        let places: Vec<_> = places
                            .iter()
                            .map(set_pattern)
                            .collect::<Result<Vec<_>, _>>()?;
                        Box::new(move |e, value| match value {
                            ConValue::Array(values) if places.len() == values.len() => {
                                for (place, value) in places.iter().zip(values) {
                                    place(e, value)?;
                                }
                                Ok(())
                            }
                            other => Err(Error::TypeError("[_]", other.type_of(e))),
                        })
                    }
                    At(Expr::Op(Op::Tuple, places), _) => {
                        let places = places
                            .iter()
                            .map(set_pattern)
                            .collect::<Result<Vec<_>, _>>()?;
                        Box::new(move |e, value| match value {
                            ConValue::Tuple(values) if places.len() == values.len() => {
                                for (place, value) in places.iter().zip(values) {
                                    place(e, value)?;
                                }
                                Ok(())
                            }
                            other => Err(Error::TypeError("[_]", other.type_of(e))),
                        })
                    }
                    // Make for Struct, Call for NamedTuple, ...
                    _ => cl_todo!("set_place({place}")?,
                })
            }

            let set_place = set_pattern(place)?;
            let value = compile(value)?;
            Box::new(move |e| {
                let value = value(e)?;
                set_place(e, value)?;
                Ok(ConValue::Unit)
            })
        }
        (Op::MulSet, [place, value]) => assign_modify!(place.mul(value)),
        (Op::DivSet, [place, value]) => assign_modify!(place.div(value)),
        (Op::RemSet, [place, value]) => assign_modify!(place.rem(value)),
        (Op::AddSet, [place, value]) => assign_modify!(place.add(value)),
        (Op::SubSet, [place, value]) => assign_modify!(place.sub(value)),
        (Op::ShlSet, [place, value]) => assign_modify!(place.shl(value)),
        (Op::ShrSet, [place, value]) => assign_modify!(place.shr(value)),
        (Op::AndSet, [place, value]) => assign_modify!(place.and(value)),
        (Op::XorSet, [place, value]) => assign_modify!(place.xor(value)),
        (Op::OrSet, [place, value]) => assign_modify!(place.or(value)),
        (op, exprs) => unimplemented!("Compile {op:?} {exprs:#?}"),
    })
}

macro assign_modify($place:ident . $op:ident ($value:ident)) {{
    let value = compile($value)?;
    let Some(place) = compile_place($place)? else {
        Err(Error::NotPlace().with_span($place.1))?
    };
    Box::new(move |e| {
        let place = place(e)?;
        let value = value(e)?;
        *place.get_mut(e)? = place.get_mut(e)?.take().$op(value, e)?;
        Ok(ConValue::Unit)
    })
}}

// /// PATTERN MATCHING CODE BELOW /// //

// step 1: determine whether the value matches
pub type Matcher = Box<dyn Fn(&mut Environment, &ConValue) -> IResult<bool>>;
// step 2: bind the value to the names
pub type Binder = Box<dyn Fn(&mut Environment, ConValue)>;
// step 3: execute match guards against the final environment
pub type Guard = Box<dyn Fn(&mut Environment) -> IResult<bool>>;

fn compile_guard_patterns(pat: &At<Pat>) -> IResult<Option<Guard>> {
    fn find_guards(pat: &At<Pat>, guards: &mut Vec<Guard>) -> IResult<()> {
        match pat.value() {
            Pat::Op(PatOp::Guard, pats) => match pats.as_slice() {
                [_, At(Pat::Value(guard), _)] => {
                    let guard = compile(guard)?;
                    guards.push(Box::new(move |e| guard(e)?.truthy(e)));
                    Ok(())
                }
                _ => cl_todo!("Guards with more than one subpattern: {pat}")?,
            },
            Pat::Op(_, pats) => pats.iter().try_for_each(|pat| find_guards(pat, guards)),
            _ => Ok(()),
        }
    }
    let mut guards = vec![];
    find_guards(pat, &mut guards)?;
    Ok(match guards.len() {
        ..=1 => guards.pop(),
        _ => Some(Box::new(move |e| {
            for guard in guards.iter() {
                if !guard(e)? {
                    return Ok(false);
                }
            }
            return Ok(true);
        })),
    })
}

#[inline]
fn got(
    matcher: impl Fn(&mut Environment, &ConValue) -> IResult<bool> + 'static,
    binder: impl Fn(&mut Environment, ConValue) + 'static,
) -> (Matcher, Binder) {
    (Box::new(matcher), Box::new(binder))
}

#[inline]
fn just_match(
    matcher: impl Fn(&mut Environment, &ConValue) -> IResult<bool> + 'static,
) -> (Matcher, Binder) {
    fn nop(_: &mut Environment, _: ConValue) {}
    got(matcher, nop)
}

fn compile_let_pattern(pat: &At<Pat>) -> IResult<(Matcher, Binder)> {
    let_inner_pattern(&pat.0)
}

fn let_inner_pattern(pat: &Pat) -> IResult<(Matcher, Binder)> {
    Ok(match pat {
        Pat::Ignore => just_match(|_, _| Ok(true)),
        Pat::Never => just_match(|_, _| Ok(true)),
        Pat::MetId(_) => cl_todo!("Metaidentifiers in patterns!")?,
        &Pat::Name(name) => got(
            |_, _| Ok(true),
            move |e, v| {
                e.bind(name, v);
            },
        ),
        Pat::Value(value) => {
            let value = compile(value)?;
            just_match(move |e, v| {
                value(e)?
                    .eq(v, e)
                    .and_then(|v| v.truthy(e))
                    .or_else(|_| Ok(false))
            })
        }
        Pat::Op(pat_op, pats) => match (pat_op, pats.as_slice()) {
            (PatOp::MetaInner, [_, pat]) => compile_let_pattern(pat)?,
            (PatOp::MetaOuter, [_, pat]) => compile_let_pattern(pat)?,
            (PatOp::Pub | PatOp::Mut, [pat]) => compile_let_pattern(pat)?,
            (PatOp::Ref, [pat]) => {
                let (matcher, binder) = compile_let_pattern(pat)?;
                got(
                    move |e, value| match value {
                        ConValue::Ref(r) => matcher(e, &r.get(e)?.clone()),
                        other => matcher(e, other),
                    },
                    move |e, value| match value {
                        ConValue::Ref(r) => binder(e, r.get(e).unwrap().clone()),
                        other => binder(e, other),
                    },
                )
            }
            (PatOp::Ptr, [_]) => cl_todo!("Pointers")?,
            (PatOp::Rest, []) => just_match(|_, _| Ok(true)),
            (PatOp::Rest, &[At(Pat::Value(ref end), _span)]) => {
                let end = compile(end)?;
                just_match(move |e, value| end(e)?.gt(value, e)?.truthy(e))
            }
            (PatOp::Rest, [rest]) => compile_let_pattern(rest)?,
            (PatOp::RangeEx, &[At(Pat::Value(ref start), _span)]) => {
                let start = compile(start)?;
                just_match(move |e, value| start(e)?.lt_eq(&value, e)?.truthy(e))
            }
            (PatOp::RangeEx, [At(Pat::Value(start), _ssp), At(Pat::Value(end), _esp)]) => {
                let (start, end) = (compile(start)?, compile(end)?);
                just_match(move |e, v| {
                    Ok(start(e)?.lt_eq(v, e)?.truthy(e)? && end(e)?.gt(v, e)?.truthy(e)?)
                })
            }
            (PatOp::RangeIn, &[At(Pat::Value(ref end), _span)]) => {
                let end = compile(end)?;
                just_match(move |e, value| end(e)?.gt_eq(&value, e)?.truthy(e))
            }
            (PatOp::RangeIn, [At(Pat::Value(start), _ssp), At(Pat::Value(end), _esp)]) => {
                let (start, end) = (compile(start)?, compile(end)?);
                just_match(move |e, v| {
                    Ok(start(e)?.lt_eq(v, e)?.truthy(e)? && end(e)?.gt_eq(v, e)?.truthy(e)?)
                })
            }
            (PatOp::Record, pats) => {
                fn member(pat: &At<Pat>) -> IResult<(Symbol, Matcher, Binder)> {
                    match pat.value() {
                        &Pat::Name(name) => Ok((
                            name,
                            Box::new(|_, _| Ok(true)),
                            Box::new(move |e, v| {
                                e.bind(name, v);
                            }),
                        )),
                        Pat::Op(op, ats) => match (op, &ats[..]) {
                            (PatOp::MetaInner | PatOp::MetaOuter, [_, e]) => member(e),
                            (PatOp::Pub | PatOp::Mut, [expr]) => member(expr),
                            (PatOp::Ref | PatOp::Ptr, [body]) => {
                                let (name, _, _) = member(body)?;
                                let (matcher, binder) = compile_let_pattern(body)?;
                                Ok((name, matcher, binder))
                            }
                            (PatOp::Rest, []) => cl_todo!("Rest pattern {pat} in record"),
                            (PatOp::Rest, [_]) => cl_todo!("Rest pattern {pat} in record"),
                            (PatOp::Record, _) => cl_todo!("{pat} subpatterns within records"),
                            (PatOp::Tuple, _) => cl_todo!("{pat} subpatterns within records"),
                            (PatOp::Slice, _) => cl_todo!("{pat} subpatterns within records"),
                            (PatOp::ArRep, _) => cl_todo!("{pat} subpatterns within records"),
                            (PatOp::Typed, &[At(Pat::Name(name), _), ref bind]) => {
                                let (matcher, binder) = compile_let_pattern(bind)?;
                                Ok((name, matcher, binder))
                            }
                            (PatOp::Guard, [pat, _]) => member(pat),
                            (PatOp::Alt, _) => cl_todo!("{pat} subpatterns within records"),
                            _ => Err(Error::PatFailed(pat.value().clone().into())),
                        },
                        _ => Err(Error::PatFailed(pat.value().clone().into())),
                    }
                }

                let mut matchers = vec![];
                let mut binders = vec![];

                for pat in pats {
                    let (name, matcher, binder) = member(pat)?;
                    matchers.push((name, matcher));
                    binders.push((name, binder));
                }

                matchers.sort_by_key(|(name, _)| *name);
                matchers.dedup_by_key(|(name, _)| *name);
                binders.sort_by_key(|(name, _)| *name);
                binders.dedup_by_key(|(name, _)| *name);

                // TODO: field exhaustiveness
                got(
                    move |e, value| match value {
                        ConValue::Struct(_, members) => {
                            for (name, matcher) in &matchers {
                                let Some(field) = members.get(name) else {
                                    return Ok(false);
                                };
                                let _ = matcher(e, field)? || return Ok(false);
                            }
                            Ok(true)
                        }
                        v => Err(Error::TypeError("struct", v.type_of(e))),
                    },
                    move |e, value| {
                        let ConValue::Struct(_, mut members) = value else {
                            unreachable!("{value} is not struct")
                        };
                        for (name, binder) in &binders {
                            let Some(value) = members.remove(name) else {
                                continue;
                            };
                            binder(e, value)
                        }
                    },
                )
            }
            (PatOp::Tuple, pats) => slice_or_tuple_pat(false, pats)?,
            (PatOp::Slice, _) => slice_or_tuple_pat(true, pats)?,
            (PatOp::ArRep, [pat, At(Pat::Value(rep), _)]) => {
                let rep = compile(rep)?;
                let (matcher, binder) = compile_let_pattern(pat)?;
                got(
                    move |e, value| {
                        let rep = match rep(e)? {
                            ConValue::Int(i) => i as usize,
                            other => Err(Error::TypeError("{integer}", other.type_of(e)))?,
                        };
                        match value {
                            ConValue::Array(values) if values.len() == rep => {
                                for value in values {
                                    let _ = matcher(e, value)? || return Ok(false);
                                }
                                Ok(true)
                            }
                            _ => Ok(false),
                        }
                    },
                    move |e, value| {
                        let ConValue::Array(values) = value else {
                            unreachable!("Unchecked matcher in ArRep")
                        };
                        for value in values {
                            binder(e, value) // FIXME: all bound vars should be the same
                        }
                    },
                )
            }
            (PatOp::Typed, [pat, _ty]) => compile_let_pattern(pat)?,
            (PatOp::TypePrefixed, [At(Pat::Value(ty), _), pat]) => {
                let (matcher, binder) = compile_let_pattern(pat)?;
                let ty = compile(ty)?;
                got(
                    move |e, v| match ty(e)? {
                        ConValue::TypeInfo(ty) => match v {
                            ConValue::Struct(t2, _) | ConValue::TupleStruct(t2, _) if ty != *t2 => {
                                Ok(false)
                            }
                            _ => matcher(e, v),
                        },
                        other => Err(Error::TypeError("type", other.type_of(e))),
                    },
                    binder,
                )
            }
            (PatOp::TypePrefixed, &[At(Pat::Name(ty), _), ref pat]) => {
                let (matcher, binder) = compile_let_pattern(pat)?;
                let ty: OwnClexpr = Box::new(move |e| e.get(ty));
                got(
                    move |e, v| match ty(e)? {
                        ConValue::TypeInfo(ty) => match v {
                            ConValue::Struct(t2, _) | ConValue::TupleStruct(t2, _) if ty != *t2 => {
                                Ok(false)
                            }
                            _ => matcher(e, v),
                        },
                        other => Err(Error::TypeError("type", other.type_of(e))),
                    },
                    binder,
                )
            }
            (PatOp::PrefixGeneric, [pat, _subs @ ..]) => compile_let_pattern(pat)?,
            (PatOp::PostfixGeneric, [_subs @ .., pat]) => compile_let_pattern(pat)?,
            (PatOp::Fn, [args, _]) => compile_let_pattern(args)?,
            (PatOp::Guard, [pat, _]) => compile_let_pattern(pat)?,
            (PatOp::Alt, _) => cl_todo!("Alt patterns (dynamic guards??)")?,
            _ => unimplemented!("{pat}"),
        },
    })
}

fn slice_or_tuple_pat(slice: bool, pats: &[At<Pat>]) -> IResult<(Matcher, Binder)> {
    // Match forward until we find a rest pattern
    let mut forward_matchers = vec![];
    let mut forward_binders = vec![];

    let mut pats = pats.iter().peekable();
    while let Some(pat) = pats.peek()
        && !matches!(pat, At(Pat::Op(PatOp::Rest, _), ..))
    {
        let (m, b) = compile_let_pattern(pat)?;
        forward_matchers.push(m);
        forward_binders.push(b);
        pats.next();
    }

    // TODO: account for ..10 patterns

    // Capture the "rest pattern", which will hold the rest of the args
    let (rest_pat, rspan) = match pats.next() {
        Some(pat @ At(Pat::Op(PatOp::Rest, _), rspan)) => (pat, rspan),
        None => {
            return Ok(got(
                move |e, value| {
                    let values = match value {
                        ConValue::Array(values) if slice => values,
                        ConValue::Tuple(values) if !slice => values,
                        ConValue::TupleStruct(_, values) if !slice => values,
                        _ => return Ok(false),
                    };
                    if values.len() != forward_matchers.len() {
                        return Ok(false);
                    };
                    for (pat, value) in forward_matchers.iter().zip(values.into_iter()) {
                        let _ = pat(e, value)? || return Ok(false);
                    }

                    Ok(true)
                },
                move |e, value| match value {
                    ConValue::Array(vs) | ConValue::Tuple(vs) | ConValue::TupleStruct(_, vs) => {
                        for (pat, value) in forward_binders.iter().zip(vs) {
                            pat(e, value);
                        }
                    }
                    _ => unreachable!("Cannot match {value} against list pattern!"),
                },
            ));
        }
        _ => unreachable!(),
    };

    // Match backward until we find a rest pattern
    let mut reverse_matchers = vec![];
    let mut reverse_binders = vec![];

    let mut pats = pats.rev().peekable();
    while let Some(pat) = pats.peek()
        && !matches!(pat, At(Pat::Op(PatOp::Rest, _), ..))
    {
        let (m, b) = compile_let_pattern(pat)?;
        reverse_matchers.push(m);
        reverse_binders.push(b);
        pats.next();
    }
    if let Some(At(second_rest, r2span)) = pats.next() {
        unimplemented!(
            "Conflicting rest-patterns would require more expensive search:\n
                first: {rest_pat} at {rspan}
                last:  {second_rest} at {r2span}",
        )
    }

    let (_rest_matcher, rest_binder) = compile_let_pattern(rest_pat)?;
    let to_collection = if slice { ConValue::Array } else { ConValue::Tuple };

    Ok(got(
        move |e, value| {
            let values = match value {
                ConValue::Array(values) if slice => values,
                ConValue::Tuple(values) if !slice => values,
                ConValue::TupleStruct(_, values) if !slice => values,
                _ => return Ok(false),
            };
            if values.len() < forward_matchers.len() + reverse_matchers.len() {
                return Ok(false);
            };
            let mut values = values.iter();
            for (pat, value) in forward_matchers.iter().zip(values.by_ref()) {
                let _ = pat(e, value)? || return Ok(false);
            }
            for (pat, value) in reverse_matchers.iter().zip(values.by_ref().rev()) {
                let _ = pat(e, value)? || return Ok(false);
            }
            // TODO: check rest pattern!

            Ok(true)
        },
        move |e, value| {
            let (ConValue::Array(vs) | ConValue::Tuple(vs) | ConValue::TupleStruct(_, vs)) = value
            else {
                unreachable!("Cannot match {value} against list pattern!")
            };
            let mut values = vs.into_iter();
            for (pat, value) in forward_binders.iter().zip(values.by_ref()) {
                pat(e, value);
            }
            for (pat, value) in reverse_binders.iter().zip(values.by_ref().rev()) {
                pat(e, value);
            }
            rest_binder(e, to_collection(values.collect()))
        },
    ))
}
