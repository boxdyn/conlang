//! A work-in-progress tree walk interpreter for Conlang
//!
//! Currently, major parts of the interpreter are not yet implemented, and major parts will never be
//! implemented in its current form. Namely, since no [ConValue] has a stable location, it's
//! meaningless to get a pointer to one, and would be undefined behavior to dereference a pointer to
//! one in any situation.
#![expect(unused, reason = "Work in progress")]

use crate::function::Function;

use super::*;
use cl_ast::{
    types::{Literal, Path},
    *,
};
use cl_structures::intern::interned::Interned;
use std::{collections::HashMap, rc::Rc, slice};

macro trace($($t:tt)*) {{
    #[cfg(debug_assertions)]
    if std::env::var("CONLANG_TRACE").is_ok() {
        eprintln!($($t)*)
    }
}}

/// Turns a `todo!` invocation into an [Error::BuiltinError]
macro cl_todo {
    () => {
        Err(Error::BuiltinError(format!("Not yet implemented (at {}:{}:{})", file!(), line!(), column!())))
    },
    ($($t:tt)*) => {
        Err(Error::BuiltinError(format!("Not yet implemented: {} (at {}:{}:{})", format_args!($($t)*), file!(), line!(), column!())))
    }
}

/// Turns an `unimplemented!` invocation into an [Error::BuiltinError]
macro cl_unimplemented {
    () => {
        Err(Error::BuiltinError(format!("Not implemented (at {}:{}:{})", file!(), line!(), column!())))
    },
    ($($t:tt)*) => {
        Err(Error::BuiltinError(format!("Not implemented: {} (at {}:{}:{})", format_args!($($t)*), file!(), line!(), column!())))
    }
}

use self::{cl_todo as todo, cl_unimplemented as unimplemented};

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
            Self::Id(path) => match path.parts.as_slice() {
                [name] => env.get(*name),
                _ => cl_todo!("Extract value of {path}"),
            },
            Self::MetId(_) => cl_todo!("Meta-identifiers are not allowed here"),
            Self::Lit(Literal::Bool(v)) => Ok(ConValue::Bool(*v)),
            Self::Lit(Literal::Char(v)) => Ok(ConValue::Char(*v)),
            Self::Lit(Literal::Int(v, _)) => Ok(ConValue::Int(*v as _)),
            Self::Lit(Literal::Str(v)) => Ok(ConValue::Str(v.as_str().into())),
            Self::Use(_) => cl_todo!("Use `{self}`"),
            Self::Bind(bind) => bind.interpret(env),
            Self::Make(make) => cl_todo!("Make `{make}`"),
            Self::Op(op, exprs) => (*op, exprs.as_slice()).interpret(env),
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
            (Op::As, [value, At(Expr::Id(Path { parts }), ..)]) => match parts.as_slice() {
                [part] => Ok(value.interpret(env)?.cast(part.to_ref())),
                _ => Err(Error::TypeError()),
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
            (Op::ArRep, [v, rep]) => cl_todo!("[{v}; {rep}]"),
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
            (Op::Static, [expr]) => cl_todo!("static {expr}"),

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
            (Op::Match, [scrutinee, arms @ ..]) => cl_todo!("match {scrutinee} {{{arms:#?}}}"),
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
                    let mut scope = env.frame("while", None);
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
            (Op::Break, [expr]) => Err(Error::Break(expr.interpret(env)?)),
            (Op::Return, [expr]) => Err(Error::Return(expr.interpret(env)?)),
            (Op::Continue, []) => Err(Error::Continue()),

            // Dot projection
            (Op::Dot, [scrutinee, proj]) => cl_todo!("dot: {scrutinee}.{proj}"),

            // Range operators
            (Op::RangeEx, [lhs, rhs]) => Ok(ConValue::TupleStruct(
                "RangeExc".into(),
                Box::new(Box::new([lhs.interpret(env)?, rhs.interpret(env)?])),
            )),
            (Op::RangeIn, [lhs, rhs]) => Ok(ConValue::TupleStruct(
                "RangeInc".into(),
                Box::new(Box::new([lhs.interpret(env)?, rhs.interpret(env)?])),
            )),
            (Op::RangeEx, [rhs]) => Ok(ConValue::TupleStruct(
                "RangeTo".into(),
                Box::new(Box::new([rhs.interpret(env)?])),
            )),
            (Op::RangeIn, [rhs]) => Ok(ConValue::TupleStruct(
                "RangeToInc".into(),
                Box::new(Box::new([rhs.interpret(env)?])),
            )),

            // Unary operators
            (Op::Neg, [expr]) => {
                let value = expr.interpret(env)?;
                env.get("not".into())?.call(env, &[value])
            }
            (Op::Not, [expr]) => {
                let value = expr.interpret(env)?;
                env.get("not".into())?.call(env, &[value]) // why is this a builtin
            }
            (Op::Identity, [expr]) => expr.interpret(env),

            // Reference manipulation operators
            (Op::Refer, [expr]) => cl_todo!("References: &{expr}"),
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
            (Op::Set, [target, value]) => cl_todo!("Set {target} to {value}"),
            (Op::MulSet, [target, value]) => cl_todo!("MulSet {target} to {value}"),
            (Op::DivSet, [target, value]) => cl_todo!("DivSet {target} to {value}"),
            (Op::RemSet, [target, value]) => cl_todo!("RemSet {target} to {value}"),
            (Op::AddSet, [target, value]) => cl_todo!("AddSet {target} to {value}"),
            (Op::SubSet, [target, value]) => cl_todo!("SubSet {target} to {value}"),
            (Op::ShlSet, [target, value]) => cl_todo!("ShlSet {target} to {value}"),
            (Op::ShrSet, [target, value]) => cl_todo!("ShrSet {target} to {value}"),
            (Op::AndSet, [target, value]) => cl_todo!("AndSet {target} to {value}"),
            (Op::XorSet, [target, value]) => cl_todo!("XorSet {target} to {value}"),
            (Op::OrSet, [target, value]) => cl_todo!("OrSet {target} to {value}"),
            (op, exprs) => cl_unimplemented!("Evaluate {op:?} {exprs:?}"),
        }
    }
}

impl Interpret for Bind<DefaultTypes> {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Bind(op, _generics, pat, exprs) = self;
        match (op, pat, exprs.as_slice()) {
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
                    Err(e) => {
                        println!("{e}");
                        false
                    }
                }))
            }
            (BindOp::Let, _, [scrutinee, default]) => {
                let mut bind = HashMap::new();
                if pat
                    .matches(
                        scrutinee.interpret(env)?,
                        &mut MatchEnv::new(env, &mut bind),
                    )
                    .is_ok()
                {
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
                println!("Warning: not creating module for {pat}");
                let [body] = exprs.as_slice() else {
                    todo!("{exprs:?}")?
                };
                body.interpret(env)
            }
            (BindOp::Mod, _, [body]) => {
                println!("Warning: not creating module for {pat}");
                body.interpret(env)
            }
            (BindOp::Impl, _, [body]) => cl_todo!("impl {pat} {body}"),
            (BindOp::Struct, _, []) => cl_todo!("struct {pat}"),
            (BindOp::Enum, _, []) => cl_todo!("enum {pat}"),
            (BindOp::For, _, [iter, pass, fail]) => {
                let iter: Box<dyn Iterator<Item = ConValue>> = match iter.interpret(env)? {
                    ConValue::Array(values) | ConValue::Tuple(values) => {
                        Box::new(values.into_iter())
                    }
                    ConValue::String(str) => Box::new(str.into_chars().map(ConValue::Char)),
                    ConValue::Str(str) => Box::new(str.to_ref().chars().map(ConValue::Char)),
                    ConValue::TupleStruct(Interned("RangeExc", ..), bounds) => match **bounds {
                        [ConValue::Int(start), ConValue::Int(end)] => {
                            Box::new((start..end).map(ConValue::Int))
                        }
                        _ => Err(Error::NotIterable())?,
                    },
                    ConValue::TupleStruct(Interned("RangeInc", ..), bounds) => match **bounds {
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

impl Match for Pat {
    fn matches<'env>(&self, value: ConValue, in_env: &mut MatchEnv<'env>) -> IResult<()> {
        match (self) {
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
            Self::Op(pat_op, pats) => (*pat_op, pats.as_slice()).matches(value, in_env),
        }
    }
}

impl Match for (PatOp, &[Pat]) {
    fn matches<'env>(&self, value: ConValue, in_env: &mut MatchEnv<'env>) -> IResult<()> {
        match self {
            (PatOp::Pub, [pat]) => pat.matches(value, &mut in_env.public()),
            (PatOp::Pub, _) => unimplemented!(),
            (PatOp::Mut, [pat]) => pat.matches(value, &mut in_env.mutable()),
            (PatOp::Mut, _) => unimplemented!(),
            (PatOp::Ref, [pat]) => match value {
                ConValue::Ref(idx) => {
                    let value = in_env
                        .env
                        .get_id(idx)
                        .cloned()
                        .ok_or_else(|| Error::PatFailed(pat.clone().into()))?;
                    pat.matches(value, in_env)
                }
                _ => Err(Error::PatFailed(
                    Pat::Op(PatOp::Ref, vec![pat.clone()]).into(),
                )),
            },
            (PatOp::Ref, _) => unimplemented!(),
            (PatOp::Ptr, _) => todo!(),
            (PatOp::Rest, []) => Ok(()),
            (PatOp::Rest, [rest]) => rest.matches(value, in_env),
            (PatOp::Rest, _) => unimplemented!("rest pattern with more than one arg"),
            (PatOp::RangeEx, _) => todo!("Range patterns"),
            (PatOp::RangeIn, _) => todo!("Range patterns"),
            (PatOp::Record, _) => todo!("Record patterns"),
            (PatOp::Tuple, pats) => match value {
                ConValue::Empty if pats.is_empty() => Ok(()),
                ConValue::Tuple(values) if pats.len() <= values.len() => {
                    (SliceMode::Tuple, *pats).matches(values, in_env)
                }
                _ => todo!("Match {pats:?} against {value}"),
            },
            (PatOp::Slice, pats) => match value {
                ConValue::Array(values) if pats.len() <= values.len() => {
                    (SliceMode::Slice, *pats).matches(values, in_env)
                }
                _ => todo!("Match {pats:?} against {value}"),
            },
            (PatOp::ArRep, _) => todo!(),
            (PatOp::Typed, [pat, _ty @ ..]) => pat.matches(value, in_env),
            (PatOp::Typed, _) => todo!(),
            (PatOp::TypePrefixed, [Pat::Name(pat_name), pat]) => match value {
                ConValue::TupleStruct(value_name, values) => {
                    if (*pat_name != value_name) {
                        Err(Error::MatchNonexhaustive())?
                    }
                    pat.matches(ConValue::Tuple(*values), in_env)
                }
                _ => pat.matches(value, in_env),
            },
            (PatOp::TypePrefixed, _) => todo!("TypePrefixed {self:?}"),
            (PatOp::Generic, [pat, _subs @ ..]) => pat.matches(value, in_env),
            (PatOp::Generic, _) => todo!(),
            (PatOp::Fn, [args, _]) => args.matches(value, in_env),
            (PatOp::Fn, _) => todo!(),
            (PatOp::Alt, alts) => todo!(
                "Alternate patterns require trying the same value multiple times, which is expensive."
            ),
        }
    }
}

enum SliceMode {
    Slice,
    Tuple,
}

impl Match<Box<[ConValue]>> for (SliceMode, &[Pat]) {
    fn matches<'env>(&self, values: Box<[ConValue]>, in_env: &mut MatchEnv<'env>) -> IResult<()> {
        let (mode, pats) = self;
        let mut values = values.into_iter();
        let mut pats = pats.iter().peekable();
        while !matches!(pats.peek(), None | Some(Pat::Op(PatOp::Rest, _))) {
            let (Some(pat), Some(value)) = (pats.next(), values.next()) else {
                break;
            };
            pat.matches(value, in_env)?;
        }

        let mut values = values.rev();
        let mut pats = pats.rev().peekable();
        while !matches!(pats.peek(), None | Some(Pat::Op(PatOp::Rest, _))) {
            let (Some(pat), Some(value)) = (pats.next(), values.next()) else {
                break;
            };
            pat.matches(value, in_env)?;
        }

        if let Some(Pat::Op(PatOp::Rest, pats)) = pats.next() {
            if let [pat] = pats.as_slice() {
                let values = values.into_inner().collect();
                match mode {
                    SliceMode::Slice => pat.matches(ConValue::Array(values), in_env)?,
                    SliceMode::Tuple => pat.matches(ConValue::Tuple(values), in_env)?,
                };
            }
        } else if values.next().is_some() {
            Err(Error::TypeError())?;
        }

        Ok(())
    }
}
