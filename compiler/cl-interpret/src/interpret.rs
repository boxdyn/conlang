//! A work-in-progress tree walk interpreter for Conlang
//!
//! Currently, major parts of the interpreter are not yet implemented, and major parts will never be
//! implemented in its current form. Namely, since no [ConValue] has a stable location, it's
//! meaningless to get a pointer to one, and would be undefined behavior to dereference a pointer to
//! one in any situation.
#![expect(unused, reason = "Work in progress")]

use super::*;
use crate::{
    function::Function,
    place::Place,
    typeinfo::{Model, Type, TypeInfo},
};
use cl_ast::{
    types::{Literal, Path},
    *,
};
use cl_structures::intern::interned::Interned;
use std::{collections::HashMap, iter, rc::Rc, slice};

macro trace($($t:tt)*) {{
    #[cfg(debug_assertions)]
    if std::env::var("CONLANG_TRACE").is_ok() {
        eprintln!($($t)*)
    }
}}

/// Turns a `todo!` invocation into an [Error::Panic]
pub macro cl_todo {
    () => {
        Err(Error::Panic(format!("Not yet implemented (at {}:{}:{})", file!(), line!(), column!())))
    },
    ($($t:tt)*) => {
        Err(Error::Panic(format!("Not yet implemented: {} (at {}:{}:{})", format_args!($($t)*), file!(), line!(), column!())))
    }
}

/// Turns an `unimplemented!` invocation into an [Error::Panic]
pub macro cl_unimplemented {
    () => {
        Err(Error::Panic(format!("Not implemented (at {}:{}:{})", file!(), line!(), column!())))
    },
    ($($t:tt)*) => {
        Err(Error::Panic(format!("Not implemented: {} (at {}:{}:{})", format_args!($($t)*), file!(), line!(), column!())))
    }
}

pub use self::{cl_todo as todo, cl_unimplemented as unimplemented};

/// A work-in-progress tree walk interpreter for Conlang
pub trait Interpret {
    /// Interprets this thing in the given [`Environment`].
    ///
    /// Everything returns a value!™
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue>;
}

impl<T: Annotation + Interpret> Interpret for At<T> {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        self.0.interpret(env).map_err(|e| e.with_span(self.1))
    }
}

impl Interpret for Expr<DefaultTypes> {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        match self {
            Self::Omitted => Ok(ConValue::Empty),
            Self::Id(path) => path.interpret(env),
            Self::MetId(_) => cl_todo!("Meta-identifiers are not allowed here"),
            Self::Lit(Literal::Bool(v)) => Ok(ConValue::Bool(*v)),
            Self::Lit(Literal::Char(v)) => Ok(ConValue::Char(*v)),
            Self::Lit(Literal::Int(v, _)) => Ok(ConValue::Int(*v as _)),
            Self::Lit(Literal::Str(v)) => Ok(ConValue::Str(v.as_str().into())),
            Self::Use(_) => cl_todo!("Use `{self}`"),
            Self::Bind(bind) => bind.interpret(env),
            Self::Make(make) => make.interpret(env),
            Self::Match(mtch) => mtch.interpret(env),
            Self::Op(op, exprs) => {
                if self.is_place()
                    && let Ok(place) = Place::new(self, env)
                {
                    place.get(env).cloned()
                } else {
                    (*op, exprs.as_slice()).interpret(env)
                }
            }
        }
    }
}

impl Interpret for (Op, &[At<Expr>]) {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        match self {
            (Op::Do, []) => Ok(ConValue::Empty),
            (Op::Do, [ats @ .., ret]) => {
                for at in ats {
                    at.0.interpret(env)?;
                }
                ret.0.interpret(env)
            }
            (Op::As, [value, ty]) => match ty.interpret(env)? {
                ConValue::TypeInfo(ty) => value.interpret(env).map(|v| v.cast(&ty)),
                other => Err(Error::TypeError("type", other.typename())),
            },
            (Op::As, [value, ty]) => cl_todo!("{value} as {ty} operator"),
            (Op::Block, []) => Ok(ConValue::Empty),
            (Op::Block, [expr]) => expr.interpret(&mut env.frame("block", None)),
            (Op::Array, []) => Ok(ConValue::Array(Box::new([]))),
            (Op::Array, exprs) => Ok(ConValue::Array(
                exprs
                    .iter()
                    .map(|e| e.interpret(env))
                    .collect::<Result<Box<_>, _>>()?,
            )),
            (Op::ArRep, [v, rep]) => {
                let v = v.interpret(env)?;
                match rep.interpret(env)? {
                    ConValue::Int(rep) => {
                        Ok(ConValue::Array(vec![v; rep as usize].into_boxed_slice()))
                    }
                    rep => Err(Error::TypeError("int", rep.typename())),
                }
            }
            (Op::Group, [expr]) => expr.interpret(env),
            (Op::Tuple, []) => Ok(ConValue::Empty),
            (Op::Tuple, exprs) => Ok(ConValue::Tuple(
                exprs
                    .iter()
                    .map(|e| e.interpret(env))
                    .collect::<Result<Box<_>, _>>()?,
            )),
            (Op::MetaInner, [_, expr]) => expr.interpret(env),
            (Op::MetaOuter, [_, expr]) => expr.interpret(env),
            (Op::Try, [expr]) => expr.interpret(env),
            (Op::Index, [expr, idx]) => expr.interpret(env)?.index(&idx.interpret(env)?, env),
            (Op::Call, [expr, arg]) => {
                let callee = expr.interpret(env)?;
                match arg.interpret(env)? {
                    ConValue::Empty => callee.call(env, &[]),
                    ConValue::Tuple(args) => callee.call(env, &args),
                    arg => callee.call(env, slice::from_ref(&arg)),
                }
            }

            // Annotation operators
            (Op::Pub, [expr]) => expr.interpret(env),
            (Op::Const, [expr]) => expr.interpret(env), // we are const
            (Op::Static, [expr]) => expr.interpret(env), // TODO: "static"

            // Control-flow
            (Op::Macro, _) => cl_todo!("Macros are not supported in the interpreter"),
            (Op::Loop, [expr]) => loop {
                match expr.interpret(env) {
                    Ok(_) => {}
                    Err(Error { kind: ErrorKind::Break(v), .. }) => break Ok(v),
                    Err(Error { kind: ErrorKind::Continue, .. }) => continue,
                    Err(e) => Err(e)?,
                }
            },
            (Op::If, [cond, pass, fail]) => {
                let mut scope = env.frame("if", None);
                if cond.interpret(&mut scope)?.truthy()? {
                    return pass.interpret(&mut scope);
                }
                drop(scope);
                fail.interpret(env)
            }
            (Op::While, [cond, pass, fail]) => {
                loop {
                    let mut scope = env.frame("while", Some(cond.1.merge(pass.1)));
                    if cond.interpret(&mut scope)?.truthy()? {
                        match pass.interpret(&mut scope) {
                            Ok(_) => {}
                            Err(Error { kind: ErrorKind::Break(value), .. }) => return Ok(value),
                            Err(Error { kind: ErrorKind::Continue, .. }) => continue,
                            Err(e) => Err(e)?,
                        }
                    } else {
                        break;
                    }
                }
                fail.interpret(env)
            }
            (Op::Defer, [expr]) => {
                env.defer(expr.value().clone());
                Ok(ConValue::Empty)
            }
            (Op::Break, [expr]) => Err(Error::Break(expr.interpret(env)?)),
            (Op::Return, [expr]) => Err(Error::Return(expr.interpret(env)?)),
            (Op::Continue, []) => Err(Error::Continue()),

            // Dot projection
            (Op::Dot, [scrutinee, At(Expr::Op(Op::Call, args), _)]) => {
                let [callee, args] = args.as_slice() else {
                    cl_todo!("Interpret non-call {args:?}")?
                };
                let scrutinee = Place::new_or_temporary(scrutinee.value(), env)?;
                let function = callee.interpret(env)?;
                let args = args.interpret(env)?;
                match args {
                    ConValue::Empty => function.call(env, &[ConValue::Ref(scrutinee)]),
                    ConValue::Tuple(args) => function.call(
                        env,
                        &iter::once(ConValue::Ref(scrutinee))
                            .chain(args) // TODO: remove allocation
                            .collect::<Box<_>>(),
                    ),
                    other => function.call(env, &[ConValue::Ref(scrutinee), other]),
                }
            }
            (
                Op::Dot,
                [
                    At(Expr::Lit(Literal::Int(whole, _)), _),
                    At(Expr::Lit(Literal::Int(frac, _)), _),
                ],
            ) => Ok(ConValue::Float(format!("{whole}.{frac}").parse().unwrap())),
            (Op::Dot, [scrutinee, At(Expr::Lit(Literal::Int(idx, _)), _)]) => {
                let place = Place::new_or_temporary(scrutinee.value(), env)?;
                Ok(ConValue::Ref(place.dot_idx(*idx as _)))
            }
            (Op::Dot, [scrutinee, At(Expr::Id(path), _)]) if path.parts.len() == 1 => {
                let name = path.parts[0];
                match scrutinee.interpret(env)? {
                    ConValue::Ref(r) => Ok(ConValue::Ref(r.dot_sym(name))),
                    ConValue::Struct(_, mut p) => p.remove(&name).ok_or(Error::NotDefined(name)),
                    ConValue::TypeInfo(ti) => ti.getattr(name),
                    ConValue::Module(m) => m.get(&name).cloned().ok_or(Error::NotDefined(name)),
                    other => Err(Error::TypeError(name.to_ref(), other.typename())),
                }
            }
            (Op::Dot, [scrutinee, proj]) => cl_todo!("dot: {scrutinee}.{proj}"),

            // Range operators
            (Op::RangeEx, [lhs, rhs]) => Ok(ConValue::TupleStruct(
                env.get_type("RangeExc".into())
                    .ok_or_else(|| Error::NotDefined("RangeExc".into()))?,
                Box::new([lhs.interpret(env)?, rhs.interpret(env)?]),
            )),
            (Op::RangeIn, [lhs, rhs]) => Ok(ConValue::TupleStruct(
                env.get_type("RangeInc".into())
                    .ok_or_else(|| Error::NotDefined("RangeInc".into()))?,
                Box::new([lhs.interpret(env)?, rhs.interpret(env)?]),
            )),
            (Op::RangeEx, [rhs]) => Ok(ConValue::TupleStruct(
                env.get_type("RangeTo".into())
                    .ok_or_else(|| Error::NotDefined("RangeTo".into()))?,
                Box::new([rhs.interpret(env)?]),
            )),
            (Op::RangeIn, [rhs]) => Ok(ConValue::TupleStruct(
                env.get_type("RangeToInc".into())
                    .ok_or_else(|| Error::NotDefined("RangeToInc".into()))?,
                Box::new([rhs.interpret(env)?]),
            )),

            // Unary operators
            (Op::Neg, [expr]) => {
                let value = expr.interpret(env)?;
                env.get("neg".into())?.call(env, &[value])
            }
            (Op::Not, [expr]) => {
                let value = expr.interpret(env)?;
                env.get("not".into())?.call(env, &[value]) // why is this a builtin
            }
            (Op::Identity, [expr]) => expr.interpret(env),

            // Reference manipulation operators
            (Op::Refer, [expr]) => Ok(ConValue::Ref(Place::new(expr.value(), env)?)),
            (Op::Deref, [expr]) => match expr.interpret(env)? {
                ConValue::Ref(place) => place.get(env).cloned(),
                other => Ok(other),
            },
            (Op::Deref, [expr]) => cl_todo!("References: *{expr}"),

            // Binary computation operators
            (Op::Mul, [lhs, rhs]) => lhs.interpret(env)? * rhs.interpret(env)?,
            (Op::Div, [lhs, rhs]) => lhs.interpret(env)? / rhs.interpret(env)?,
            (Op::Rem, [lhs, rhs]) => lhs.interpret(env)? % rhs.interpret(env)?,
            (Op::Add, [lhs, rhs]) => lhs.interpret(env)? + rhs.interpret(env)?,
            (Op::Sub, [lhs, rhs]) => lhs.interpret(env)? - rhs.interpret(env)?,
            (Op::Shl, [lhs, rhs]) => lhs.interpret(env)? << rhs.interpret(env)?,
            (Op::Shr, [lhs, rhs]) => lhs.interpret(env)? >> rhs.interpret(env)?,
            (Op::And, [lhs, rhs]) => lhs.interpret(env)? & rhs.interpret(env)?,
            (Op::Xor, [lhs, rhs]) => lhs.interpret(env)? ^ rhs.interpret(env)?,
            (Op::Or, [lhs, rhs]) => lhs.interpret(env)? | rhs.interpret(env)?,

            // Comparison operators
            (Op::Lt, [lhs, rhs]) => lhs.interpret(env)?.lt(&rhs.interpret(env)?),
            (Op::Leq, [lhs, rhs]) => lhs.interpret(env)?.lt_eq(&rhs.interpret(env)?),
            (Op::Eq, [lhs, rhs]) => lhs.interpret(env)?.eq(&rhs.interpret(env)?),
            (Op::Neq, [lhs, rhs]) => lhs.interpret(env)?.neq(&rhs.interpret(env)?),
            (Op::Geq, [lhs, rhs]) => lhs.interpret(env)?.gt_eq(&rhs.interpret(env)?),
            (Op::Gt, [lhs, rhs]) => lhs.interpret(env)?.gt(&rhs.interpret(env)?),

            // Logical (control flow) operators
            (Op::LogAnd, [lhs, rhs]) => {
                let lhs = lhs.interpret(env)?;
                if lhs.truthy()? {
                    rhs.interpret(env)
                } else {
                    Ok(lhs)
                }
            }
            (Op::LogXor, [lhs, rhs]) => todo!(),
            (Op::LogOr, [lhs, rhs]) => {
                let lhs = lhs.interpret(env)?;
                if lhs.truthy()? {
                    Ok(lhs)
                } else {
                    rhs.interpret(env)
                }
            }

            // Assignment operators
            (Op::Set, [target, value]) => {
                let place = Place::new(target.value(), env)?;
                let value = value.interpret(env)?;
                *(place.get_mut(env)?) = value;
                Ok(ConValue::Empty)
            }
            (Op::MulSet, [target, value]) => {
                let place = Place::new(target.value(), env)?;
                let value = value.interpret(env)?;
                place.get_mut(env)?.mul_assign(value)?;
                Ok(ConValue::Empty)
            }
            (Op::DivSet, [target, value]) => {
                let place = Place::new(target.value(), env)?;
                let value = value.interpret(env)?;
                place.get_mut(env)?.div_assign(value)?;
                Ok(ConValue::Empty)
            }
            (Op::RemSet, [target, value]) => {
                let place = Place::new(target.value(), env)?;
                let value = value.interpret(env)?;
                place.get_mut(env)?.rem_assign(value)?;
                Ok(ConValue::Empty)
            }
            (Op::AddSet, [target, value]) => {
                let place = Place::new(target.value(), env)?;
                let value = value.interpret(env)?;
                place.get_mut(env)?.add_assign(value)?;
                Ok(ConValue::Empty)
            }
            (Op::SubSet, [target, value]) => {
                let place = Place::new(target.value(), env)?;
                let value = value.interpret(env)?;
                place.get_mut(env)?.sub_assign(value)?;
                Ok(ConValue::Empty)
            }
            (Op::ShlSet, [target, value]) => {
                let place = Place::new(target.value(), env)?;
                let value = value.interpret(env)?;
                place.get_mut(env)?.shl_assign(value)?;
                Ok(ConValue::Empty)
            }
            (Op::ShrSet, [target, value]) => {
                let place = Place::new(target.value(), env)?;
                let value = value.interpret(env)?;
                place.get_mut(env)?.shr_assign(value)?;
                Ok(ConValue::Empty)
            }
            (Op::AndSet, [target, value]) => {
                let place = Place::new(target.value(), env)?;
                let value = value.interpret(env)?;
                place.get_mut(env)?.bitand_assign(value)?;
                Ok(ConValue::Empty)
            }
            (Op::XorSet, [target, value]) => {
                let place = Place::new(target.value(), env)?;
                let value = value.interpret(env)?;
                place.get_mut(env)?.bitxor_assign(value)?;
                Ok(ConValue::Empty)
            }
            (Op::OrSet, [target, value]) => {
                let place = Place::new(target.value(), env)?;
                let mut value = value.interpret(env)?;
                place.get_mut(env)?.bitor_assign(value)?;
                Ok(ConValue::Empty)
            }
            (op, exprs) => cl_unimplemented!("Evaluate {op:?} {exprs:#?}"),
        }
    }
}

impl Interpret for Bind<DefaultTypes> {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Bind(op, _generics, pat, exprs) = self;
        match (op, pat.value(), exprs.as_slice()) {
            (BindOp::Let, _, []) => cl_todo!("let {pat}"),
            (BindOp::Let, _, [scrutinee]) => {
                let mut bind = HashMap::new();
                let out = pat.matches(
                    scrutinee.interpret(env)?,
                    &mut MatchEnv::new(env, &mut bind),
                );

                Ok(ConValue::Bool(match out {
                    Ok(_) => {
                        for (name, value) in bind {
                            env.bind(name, value);
                        }
                        true
                    }
                    Err(e) => false,
                }))
            }
            (BindOp::Let, _, [scrutinee, default]) => {
                let mut bind = HashMap::new();
                let out = pat.matches(
                    scrutinee.interpret(env)?,
                    &mut MatchEnv::new(env, &mut bind),
                );

                if out.is_ok() {
                    for (name, value) in bind {
                        env.bind(name, value);
                    }
                    return Ok(ConValue::Empty);
                }

                bind.clear();
                pat.matches(default.interpret(env)?, &mut MatchEnv::new(env, &mut bind))?;
                for (name, value) in bind {
                    env.bind(name, value);
                }

                Ok(ConValue::Empty)
            }
            (BindOp::Type, _, []) => cl_todo!("type {pat}"),
            (BindOp::Type, _, [body]) => cl_todo!("type {pat} = {body}"),
            (BindOp::Fn, _, [body]) => {
                let func = Rc::new(Function::new(self));
                if let Some(name) = func.name() {
                    env.bind(name, ConValue::Function(func.clone()))
                }
                Ok(ConValue::Function(func))
            }
            (BindOp::Mod, _, [At(Expr::Op(Op::Block, exprs), ..)]) => {
                let [body] = exprs.as_slice() else {
                    todo!("{exprs:?}")?
                };
                body.interpret(env)
            }
            (BindOp::Mod, _, [body]) => body.interpret(env),
            (BindOp::Impl, _, [body]) => cl_todo!("impl {pat} {body}"),
            (BindOp::Struct, pat, []) => {
                let (name, model) = bind_struct(pat, env)?;
                Ok(ConValue::TypeInfo(if let Some(name) = name {
                    let typeid = env.def_type(name, model);
                    env.bind(name, ConValue::TypeInfo(typeid));
                    typeid
                } else {
                    TypeInfo { ident: Interned::default(), model }.intern()
                }))
            }
            (BindOp::Struct, Pat::Name(name), []) => {
                let typeid = env.def_type(*name, typeinfo::Model::Unit(0));
                env.bind(*name, ConValue::TypeInfo(typeid));
                Ok(ConValue::TypeInfo(typeid))
            }
            (BindOp::Struct, pat, []) => cl_todo!("struct {pat}"),
            (BindOp::Enum, Pat::Name(name), []) => {
                let typeid = env.def_type(*name, typeinfo::Model::Never);
                env.bind(*name, ConValue::TypeInfo(typeid));
                Ok(ConValue::TypeInfo(typeid))
            }
            (BindOp::Enum, pat, []) => bind_enum(pat, env),
            (BindOp::Enum, _, []) => cl_todo!("enum {pat}"),
            (BindOp::For, _, [iter, pass, fail]) => {
                let iter: Box<dyn Iterator<Item = ConValue>> = match iter.interpret(env)? {
                    ConValue::Array(values) | ConValue::Tuple(values) => {
                        Box::new(values.into_iter())
                    }
                    ConValue::String(str) => Box::new(str.into_chars().map(ConValue::Char)),
                    ConValue::Str(str) => Box::new(str.to_ref().chars().map(ConValue::Char)),
                    ConValue::TupleStruct(
                        Interned(TypeInfo { ident: Interned("RangeExc", ..), .. }, ..),
                        bounds,
                    ) => match *bounds {
                        [ConValue::Int(start), ConValue::Int(end)] => {
                            Box::new((start..end).map(ConValue::Int))
                        }
                        _ => Err(Error::NotIterable())?,
                    },
                    ConValue::TupleStruct(
                        Interned(TypeInfo { ident: Interned("RangeInc", ..), .. }, ..),
                        bounds,
                    ) => match *bounds {
                        [ConValue::Int(start), ConValue::Int(end)] => {
                            Box::new((start..=end).map(ConValue::Int))
                        }
                        _ => Err(Error::NotIterable())?,
                    },
                    _ => Err(Error::NotIterable())?,
                };
                for item in iter {
                    let mut bind = HashMap::new();
                    pat.matches(item, &mut MatchEnv::new(env, &mut bind))?;

                    let mut scope = env.with_frame("for-loop", bind);
                    match pass.interpret(&mut scope) {
                        Ok(_) => {}
                        Err(Error { kind: ErrorKind::Break(value), .. }) => return Ok(value),
                        Err(Error { kind: ErrorKind::Continue, .. }) => continue,
                        Err(e) => Err(e)?,
                    }
                }
                fail.interpret(env)
            }
            _ => cl_unimplemented!("{self}"),
        }
    }
}

impl Interpret for Path {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        match self.parts.as_slice() {
            &[name] => env.get(name),
            [first, names @ ..] => {
                let mut value = env.get(*first)?;
                for name in names {
                    value = match value {
                        ConValue::Module(values) => {
                            values.get(name).cloned().ok_or(Error::NotDefined(*name))?
                        }
                        ConValue::TypeInfo(ty) => ty.getattr(*name)?,
                        _ => todo!("{self}")?,
                    };
                }
                Ok(value)
            }
            [] => unimplemented!("Empty paths"),
            _ => todo!("Extract value at {self}"),
        }
    }
}

impl Interpret for Sym {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        env.get(*self)
    }
}

fn find_interned_type(model: &Model) -> Option<Type> {
    let mut ti: Option<Type> = None;
    // TODO: these functions should return typeinfos
    typeinfo::TYPE_INTERNER.get().unwrap().foreach(|v| {
        if v.model == *model {
            ti = Some(v.already_interned())
        }
    });
    ti
}

// TODO: these functions should return typeinfos
fn bind_struct(pat: &Pat, env: &mut Environment) -> IResult<(Option<Sym>, Model)> {
    fn bind_struct_op(
        op: PatOp,
        pats: &[At<Pat>],
        env: &mut Environment,
    ) -> IResult<(Option<Sym>, Model)> {
        match (op, pats) {
            (PatOp::MetaOuter | PatOp::MetaInner, [_doc, pat]) => bind_struct(pat.value(), env),
            (PatOp::Pub | PatOp::Mut, [expr]) => bind_struct(expr.value(), env),
            (PatOp::Ref, [..]) => todo!("Ref in bind_struct_op?"),
            (PatOp::Ptr, [..]) => todo!("Ptr in bind_struct_op?"),
            (PatOp::Rest, [..]) => {
                println!(
                    "Host backtrace:\n{}",
                    std::backtrace::Backtrace::force_capture()
                );
                todo!("Rest in bind_struct_op?")
            }
            (PatOp::RangeEx, [..]) => todo!("RangeEx in bind_struct_op?"),
            (PatOp::RangeIn, [..]) => todo!("RangeIn in bind_struct_op?"),
            (PatOp::Record, elements) => {
                let mut members = Vec::new();
                let mut exhaustive = true;
                for (idx, member) in elements.iter().enumerate() {
                    if let Pat::Op(PatOp::Rest, _) = member.value() {
                        exhaustive = false;
                        continue;
                    }
                    if let (Some(name), model) = bind_struct(member.value(), env)? {
                        let mut ti: Option<Type> = find_interned_type(&model);
                        members.push((name, ti.unwrap_or(env.get_type("_".into()).unwrap())));
                    }
                }
                Ok((None, Model::Struct(members.into_boxed_slice(), exhaustive)))
            }
            (PatOp::Tuple, elements) => {
                let members = elements
                    .iter()
                    .map(|pat| {
                        let (_, model) = bind_struct_op(
                            PatOp::Typed,
                            &[Pat::Ignore.at(pat.1), pat.clone()],
                            env,
                        )?;
                        Ok(find_interned_type(&model).unwrap_or(env.get_type("_".into()).unwrap()))
                    })
                    .collect::<IResult<_>>()?;
                Ok((None, Model::Tuple(members)))
            }
            (PatOp::Typed, [name, At(Pat::Name(ty), ..)]) => match env.get(*ty)? {
                ConValue::TypeInfo(ty) => {
                    bind_struct(name.value(), env).map(|(name, _)| (name, ty.model.clone()))
                }
                other => todo!("Typed {name}: {other}"),
            },
            (PatOp::Typed, [name, At(Pat::Value(expr), ..)]) => match expr.interpret(env)? {
                ConValue::TypeInfo(ty) => {
                    bind_struct(name.value(), env).map(|(name, _)| (name, ty.model.clone()))
                }
                other => todo!("Typed {name}: {other}"),
            },
            (PatOp::Typed, [name, _ty]) => bind_struct(name.value(), env),
            (PatOp::TypePrefixed, [name, ty]) => match bind_struct(ty.value(), env)? {
                (None, model) => Ok((name.value().name(), model)),
                (Some(name), model) => todo!("Typeprefixed {name} :: {model:?}"),
            },
            (PatOp::Generic, [first, ..]) => bind_struct(first.value(), env),
            _ => todo!("{op:?} ({pats:?})"),
        }
    }
    Ok(match pat {
        Pat::Ignore => (None, Model::Any),
        Pat::Never => todo!("Pat::Never in struct binding")?,
        Pat::MetId(_) => todo!("Pat::MetId in struct binding")?,
        Pat::Name(name) => (Some(*name), typeinfo::Model::Unit(0)),
        Pat::Value(at) => match at.interpret(env)? {
            ConValue::TypeInfo(t) => (None, t.model.clone()),
            other => todo!("Pat::Value({other}) in struct binding")?,
        },
        Pat::Op(pat_op, pats) => bind_struct_op(*pat_op, pats, env)?,
    })
}

fn bind_enum(pat: &Pat, env: &mut Environment) -> IResult<ConValue> {
    let name = pat
        .name()
        .ok_or_else(|| Error::PatFailed(Box::new(pat.clone())))?;
    let mut variants = vec![];
    if let Pat::Op(PatOp::TypePrefixed, pats) = pat
        && let [prefix, pats] = &pats[..]
        && let Pat::Op(PatOp::Record, pats) = pats.value()
    {
        let mut scope = env.frame(name.to_ref(), Default::default());
        if let Pat::Op(PatOp::Generic, gens) = prefix.value()
            && let [prefix, vars @ ..] = gens.as_slice()
        {
            for var in vars {
                let Some(name) = var.value().name() else {
                    continue;
                };
                let t = TypeInfo::new(name, Model::Any).intern();
                scope.bind(name, ConValue::TypeInfo(t));
            }
        }
        for (idx, pat) in pats.iter().enumerate() {
            if let (Some(name), model) = bind_struct(pat.value(), &mut scope)? {
                let model = match model {
                    Model::Unit(_) => Model::Unit(idx),
                    _ => model,
                };
                let typeid = scope.def_type(name, model);
                variants.push((name, typeid));
            }
        }
    } else {
        todo!("Bind other enum: {pat}")?
    };

    let typeid = env.def_type(name, Model::Enum(variants.into_boxed_slice()));
    env.bind(name, ConValue::TypeInfo(typeid));
    Ok(ConValue::TypeInfo(typeid))
}

impl Interpret for Make<DefaultTypes> {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self(ty, entries) = self;

        let tyinfo = match ty.interpret(env)? {
            ConValue::TypeInfo(info) => info,
            other => Err(Error::TypeError("type", other.typename()))?,
        };

        let mut members = HashMap::new();

        for MakeArm(name, value) in entries {
            // todo: disallow redefinition?
            members.insert(
                *name,
                match value {
                    Some(value) => value.interpret(env),
                    None => env.get(*name),
                }?,
            );
        }

        tyinfo.make_struct(members)
    }
}

impl Interpret for cl_ast::ast::Match<DefaultTypes> {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self(scrutinee, arms) = self;
        let scrutinee = scrutinee.interpret(env)?;
        for MatchArm(pat, expr) in arms {
            let mut bind = HashMap::new();
            if pat
                .matches(scrutinee.clone(), &mut MatchEnv::new(env, &mut bind))
                .is_ok()
            {
                return expr.interpret(&mut env.with_frame("match-arm", bind));
            }
        }
        Err(Error::MatchNonexhaustive(ConValue::Empty))
    }
}

pub enum BindingMode {
    Value,
    Ref,
    Ptr,
}

#[derive(Debug)]
pub struct MatchEnv<'env> {
    env: &'env mut Environment,
    bind: &'env mut HashMap<Sym, ConValue>,
    public: bool,
    mutable: bool,
}

impl MatchEnv<'_> {
    pub fn new<'e>(env: &'e mut Environment, bind: &'e mut HashMap<Sym, ConValue>) -> MatchEnv<'e> {
        MatchEnv { env, bind, public: false, mutable: false } // TODO: env.public
    }
    pub fn public(&mut self) -> MatchEnv<'_> {
        let Self { ref mut env, ref mut bind, public: _, mutable } = *self;
        MatchEnv { env, bind, public: true, mutable }
    }

    pub fn mutable(&mut self) -> MatchEnv<'_> {
        let Self { ref mut env, ref mut bind, public, mutable: _ } = *self;
        MatchEnv { env, bind, public, mutable: true }
    }
}

pub trait Match<Value = ConValue> {
    fn matches<'env>(&self, value: Value, in_env: &mut MatchEnv<'env>) -> IResult<()>;
}

impl Match for At<Pat> {
    fn matches<'env>(&self, value: ConValue, in_env: &mut MatchEnv<'env>) -> IResult<()> {
        self.value().matches(value, in_env)
    }
}

impl Match for Pat {
    fn matches<'env>(&self, value: ConValue, in_env: &mut MatchEnv<'env>) -> IResult<()> {
        match self {
            Self::Ignore => Ok(()),
            Self::Never => todo!("Never"),
            Self::MetId(_) => todo!("Meta-identifiers are not allowed here"),
            &Self::Name(name) => {
                in_env.bind.insert(name, value);
                Ok(())
            }
            Self::Value(at) => {
                let truth = at.interpret(in_env.env)?;
                if truth.neq(&value)?.truthy()? {
                    Err(Error::PatFailed(self.clone().into()))
                } else {
                    Ok(())
                }
            }
            Self::Op(pat_op, pats) => (*pat_op, &pats[..]).matches(value, in_env),
        }
    }
}

impl Match for (PatOp, &[At<Pat>]) {
    fn matches<'env>(&self, value: ConValue, in_env: &mut MatchEnv<'env>) -> IResult<()> {
        match self {
            (PatOp::MetaInner | PatOp::MetaOuter, [_doc, pat]) => pat.matches(value, in_env),
            (PatOp::MetaInner | PatOp::MetaOuter, _) => unimplemented!(),
            (PatOp::Pub, [pat]) => pat.matches(value, &mut in_env.public()),
            (PatOp::Pub, _) => unimplemented!(),
            (PatOp::Mut, [pat]) => pat.matches(value, &mut in_env.mutable()),
            (PatOp::Mut, _) => unimplemented!(),
            (PatOp::Ref, [pat]) => match value {
                ConValue::Ref(place) => {
                    let value = place
                        .get(in_env.env)
                        .cloned()
                        .map_err(|_| Error::PatFailed(pat.value().clone().into()))?;
                    pat.matches(value, in_env)
                }
                // Auto-referencing in patterns..?
                other => pat.matches(other, in_env),
            },
            (PatOp::Ref, _) => unimplemented!("Ref patterns!"),
            (PatOp::Ptr, _) => todo!("Raw pointer/deref patterns?"),
            (PatOp::Rest, []) => Ok(()),
            // Rest pattern with const value is upper-bounded exclusive range
            (PatOp::Rest, [At(Pat::Value(end), ..)]) => {
                if end.interpret(in_env.env)?.lt_eq(&value)?.truthy()? {
                    return Err(Error::MatchNonexhaustive(value));
                }
                Ok(())
            }
            (PatOp::Rest, [rest]) => rest.matches(value, in_env),
            (PatOp::Rest, _) => unimplemented!("rest pattern with more than one arg"),
            (PatOp::RangeEx, [At(Pat::Value(start), ..)]) => {
                // RangeEx pattern with const value is lower-bounded exclusive range
                if start.interpret(in_env.env)?.gt(&value)?.truthy()? {
                    return Err(Error::MatchNonexhaustive(value));
                }
                Ok(())
            }
            (PatOp::RangeEx, [At(Pat::Value(start), ..), At(Pat::Value(end), ..)]) => {
                if start.interpret(in_env.env)?.gt(&value)?.truthy()? {
                    return Err(Error::MatchNonexhaustive(value));
                }
                if end.interpret(in_env.env)?.lt_eq(&value)?.truthy()? {
                    return Err(Error::MatchNonexhaustive(value));
                }
                Ok(())
            }
            (PatOp::RangeEx, pats) => todo!("RangeEx patterns: {pats:?}"),
            (PatOp::RangeIn, [At(Pat::Value(start), ..), At(Pat::Value(end), ..)]) => {
                if start.interpret(in_env.env)?.gt(&value)?.truthy()? {
                    return Err(Error::MatchNonexhaustive(value));
                }
                if end.interpret(in_env.env)?.lt(&value)?.truthy()? {
                    return Err(Error::MatchNonexhaustive(value));
                }
                Ok(())
            }
            (PatOp::RangeIn, pats) => todo!("Range patterns: {pats:?}"),
            (PatOp::Record, pats) => match_pat_for_struct(pats, value, in_env),
            (PatOp::Tuple, pats) => match value {
                ConValue::Empty if pats.is_empty() => Ok(()),
                ConValue::Tuple(values) => (SliceMode::Tuple, *pats).matches(values, in_env),
                _ => todo!("Match {pats:?} against {value}"),
            },
            (PatOp::Slice, pats) => match value {
                ConValue::Array(values) => (SliceMode::Slice, *pats).matches(values, in_env),
                _ => todo!("Match {pats:?} against {value}"),
            },
            (PatOp::ArRep, _) => todo!(),
            (PatOp::Typed, [pat, _ty @ ..]) => pat.matches(value, in_env),
            (PatOp::Typed, _) => todo!(),
            (PatOp::TypePrefixed, [At(Pat::Value(e), ..), pat]) => {
                let ty = match e.interpret(in_env.env)? {
                    ConValue::TypeInfo(ty) => ty,
                    other => Err(Error::TypeError("type", other.typename()))?,
                };
                match value {
                    ConValue::Struct(value_ty, _) | ConValue::TupleStruct(value_ty, _)
                        if ty != value_ty =>
                    {
                        Err(Error::TypeError(ty.ident.to_ref(), value_ty.ident.to_ref()))?
                    }
                    ConValue::TupleStruct(value_ty, values) => {
                        pat.matches(ConValue::Tuple(values), in_env)
                    }
                    _ => pat.matches(value, in_env),
                }
            }
            (PatOp::TypePrefixed, [pat_name, pat]) => match value {
                ConValue::TupleStruct(value_type, values) => {
                    let Some(pat_name) = pat_name.value().name() else {
                        todo!("{pat_name}({pat})")?
                    };
                    if in_env.env.get_type(pat_name) != Some(value_type) {
                        Err(Error::TypeError(
                            pat_name.to_ref(),
                            value_type.ident.to_ref(),
                        ))?
                    }
                    pat.matches(ConValue::Tuple(values), in_env)
                }
                ConValue::Struct(ty, _) => {
                    let Some(pat_name) = pat_name.value().name() else {
                        todo!("{pat_name}({pat})")?
                    };
                    if in_env.env.get_type(pat_name) != Some(ty) {
                        Err(Error::TypeError(pat_name.to_ref(), ty.ident.to_ref()))?
                    }
                    pat.matches(value, in_env)
                }
                _ => pat.matches(value, in_env),
            },
            (PatOp::TypePrefixed, _) => todo!("TypePrefixed {self:?}"),
            (PatOp::Generic, [pat, _subs @ ..]) => pat.matches(value, in_env),
            (PatOp::Generic, _) => todo!(),
            (PatOp::Fn, [args, _]) => args.matches(value, in_env),
            (PatOp::Fn, _) => todo!(),
            (PatOp::Guard, [pat, At(Pat::Value(cond), ..)]) => {
                use std::mem::{replace, take};
                pat.matches(value, in_env)?;
                let mut scope = in_env.env.with_frame("if-guard", take(in_env.bind));
                if cond.interpret(&mut scope)?.truthy()? {
                    *in_env.bind = scope.pop_values().unwrap_or_default();
                    Ok(())
                } else {
                    Err(Error::MatchNonexhaustive(ConValue::Bool(false)))
                }
            }
            (PatOp::Guard, _) => unimplemented!("Nonbinary guard patterns!"),
            &(PatOp::Alt, [first @ .., last]) => {
                for alt in first {
                    let mut bind = HashMap::new();
                    if alt
                        .matches(value.clone(), &mut MatchEnv::new(in_env.env, &mut bind))
                        .is_ok()
                    {
                        in_env.bind.extend(bind);
                        return Ok(());
                    }
                }
                last.matches(value, in_env)
            }
            (PatOp::Alt, _) => Err(Error::MatchNonexhaustive(value)),
        }
    }
}

fn match_pat_for_struct<'env>(
    pats: &[At<Pat>],
    value: ConValue,
    in_env: &mut MatchEnv<'env>,
) -> IResult<()> {
    fn match_typed<'env>(
        pats: &[At<Pat>],
        values: &mut HashMap<Sym, ConValue>,
        in_env: &mut MatchEnv<'env>,
    ) -> IResult<()> {
        let [At(Pat::Name(name), ..), dest] = pats else {
            todo!("match_typed could not find name in typed pattern")?
        };
        let Some(value) = values.remove(name) else {
            todo!("Struct {values:?} has no value at {name}")?
        };
        in_env.bind.insert(*name, value);
        Ok(())
    }

    let ConValue::Struct(_, mut values) = value else {
        todo!("match_pat_for_struct called on {}", value)?
    };

    for pat in pats {
        match pat.value() {
            Pat::Name(name) => {
                let Some(value) = values.remove(name) else {
                    todo!("Struct {values:?} has no value at {name}")?
                };
                in_env.bind.insert(*name, value);
            }
            Pat::Op(PatOp::Typed, pats) => match_typed(pats, &mut values, in_env)?,
            Pat::Op(PatOp::Rest, pats) if pats.is_empty() => break,
            pat => todo!("{pat}")?,
        }
    }
    Ok(())
}

enum SliceMode {
    Slice,
    Tuple,
}

impl Match<Box<[ConValue]>> for (SliceMode, &[At<Pat>]) {
    fn matches<'env>(&self, values: Box<[ConValue]>, in_env: &mut MatchEnv<'env>) -> IResult<()> {
        let (mode, pats) = self;

        let mut values = values.into_iter();
        let mut pats = pats.iter().peekable();
        while !matches!(pats.peek(), None | Some(At(Pat::Op(PatOp::Rest, _), ..))) {
            let (Some(pat), Some(value)) = (pats.next(), values.next()) else {
                Err(Error::MatchNonexhaustive(ConValue::Empty))?
            };
            pat.matches(value, in_env)?;
        }

        let mut values = values.rev();
        let mut pats = pats.rev().peekable();
        while !matches!(pats.peek(), None | Some(At(Pat::Op(PatOp::Rest, _), ..))) {
            let (Some(pat), Some(value)) = (pats.next(), values.next()) else {
                Err(Error::MatchNonexhaustive(ConValue::Empty))?
            };
            pat.matches(value, in_env)?;
        }

        let mut values = values.into_inner();

        match (pats.peek(), values.next()) {
            (None, None) => Ok(()),
            (None, Some(value)) => Err(Error::MatchNonexhaustive(value)),
            (Some(At(Pat::Op(PatOp::Rest, rests), ..)), value) if let [pat] = &rests[..] => {
                let values = value.into_iter().chain(values).collect();
                match mode {
                    SliceMode::Slice => pat.matches(ConValue::Array(values), in_env),
                    SliceMode::Tuple => pat.matches(ConValue::Tuple(values), in_env),
                }
            }
            (Some(At(Pat::Op(PatOp::Rest, rests), ..)), _) if rests.is_empty() => Ok(()),
            (Some(pat), _) => Err(Error::PatFailed(pat.0.clone().into())),
        }
    }
}
