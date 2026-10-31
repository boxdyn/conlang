//! A work-in-progress tree walk [interpreter](Interpret) for Conlang
//!
//! Currently, major parts of the interpreter are not yet implemented, and major parts will never be
//! implemented in its current form. Namely, since no [ConValue] has a stable location, it's
//! meaningless to get a pointer to one, and would be undefined behavior to dereference a pointer to
//! one in any situation.

use super::*;
use crate::{
    closurpreter::compile,
    typeinfo::{Model, Type},
};
use cl_ast::*;

#[expect(unused, reason = "To be reintroduced later")]
macro trace($($t:tt)*) {{
    #[cfg(debug_assertions)]
    if std::env::var("CONLANG_TRACE").is_ok() {
        eprintln!($($t)*)
    }
}}

/// Turns a `todo!` invocation into an [Error::Panic]
pub macro cl_todo {
    () => {{
        std::hint::cold_path();
        Err(Error::Panic(format!("Not yet implemented (at {}:{}:{})", file!(), line!(), column!())))
    }},
    ($($t:tt)*) => {{
        std::hint::cold_path();
        Err(Error::Panic(format!("Not yet implemented: {} (at {}:{}:{})", format_args!($($t)*), file!(), line!(), column!())))
    }}
}

/// Turns an `unimplemented!` invocation into an [Error::Panic]
pub macro cl_unimplemented {
    () => {{
        std::hint::cold_path();
        Err(Error::Panic(format!("Not implemented (at {}:{}:{})", file!(), line!(), column!())))
    }},
    ($($t:tt)*) => {{
        std::hint::cold_path();
        Err(Error::Panic(format!("Not implemented: {} (at {}:{}:{})", format_args!($($t)*), file!(), line!(), column!())))
    }}
}

pub use self::{cl_todo as todo, cl_unimplemented as unimplemented};

fn find_interned_type(model: &Model) -> Option<Type> {
    // TODO: these functions should return typeinfos
    typeinfo::TYPE_INTERNER.get().unwrap().get(model)
}

// TODO: these functions should return typeinfos
pub(crate) fn struct_binds(pat: &Pat, env: &mut Environment) -> IResult<(Option<Sym>, Model)> {
    fn bind_struct_op(
        op: PatOp,
        pats: &[At<Pat>],
        env: &mut Environment,
    ) -> IResult<(Option<Sym>, Model)> {
        match (op, pats) {
            (PatOp::MetaOuter | PatOp::MetaInner, [_doc, pat]) => struct_binds(pat.value(), env),
            (PatOp::Pub | PatOp::Mut, [expr]) => struct_binds(expr.value(), env),
            (PatOp::Ref, [pat]) => {
                let (name, ty) = struct_binds(pat.value(), env)?;
                match find_interned_type(&ty) {
                    Some(ty) => Ok((name, Model::Ref(ty))),
                    None => todo!("Ref {pat} in bind_struct_op?"),
                }
            }
            (PatOp::Slice, [pat]) => {
                let (name, ty) = struct_binds(pat.value(), env)?;
                match find_interned_type(&ty) {
                    Some(ty) => Ok((name, Model::Slice(ty))),
                    None => todo!("Ref {pat} in bind_struct_op?"),
                }
            }
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
                let mut fields = Vec::new();
                let mut exhaustive = true;
                for field in elements.iter() {
                    if let Pat::Op(PatOp::Rest, _) = field.value() {
                        exhaustive = false;
                        continue;
                    }
                    if let (Some(name), model) = struct_binds(field.value(), env)? {
                        let ti: Option<Type> = find_interned_type(&model);
                        fields.push((name, ti.unwrap_or(env.get_type(env.kw.underscore).unwrap())));
                    }
                }
                Ok((
                    None,
                    Model::Struct {
                        name: None,
                        parent: None,
                        fields: fields.into_boxed_slice(),
                        exhaustive,
                    },
                ))
            }
            (PatOp::Tuple, elements) => {
                let fields = elements
                    .iter()
                    .map(|pat| {
                        let (_, model) = bind_struct_op(
                            PatOp::Typed,
                            &[Pat::Ignore.at(pat.1), pat.clone()],
                            env,
                        )?;
                        Ok(model.intern())
                    })
                    .collect::<IResult<_>>()?;
                let model = Model::Tuple { name: None, parent: None, fields };
                Ok((None, model))
            }
            (PatOp::Typed, [name, ty]) => {
                let (name, _) = struct_binds(name.value(), env)?;
                let (_, ty) = struct_binds(ty.value(), env)?;
                Ok((name, ty))
            }
            (PatOp::TypePrefixed, [name, ty]) => match struct_binds(ty.value(), env)? {
                (None, model) if let Some(name) = name.value().name() => {
                    Ok((Some(name), model.with_name(name)))
                }
                (None, model) => Ok((None, model)),
                (Some(name), model) => todo!("Typeprefixed {name} :: {model:?}"),
            },
            (PatOp::PostfixGeneric, [first, ..]) => struct_binds(first.value(), env),
            (PatOp::Fn, [..]) => Ok((None, Model::Function)),
            _ => todo!("{op:?} ({pats:?})"),
        }
    }
    Ok(match pat {
        Pat::Ignore => (None, Model::Any),
        Pat::Never => todo!("Pat::Never in struct binding")?,
        Pat::MetId(_) => todo!("Pat::MetId in struct binding")?,
        Pat::Name(name) => match env.get(*name) {
            Ok(ConValue::TypeInfo(t)) => (Some(*name), t.to_ref().clone()),
            _ => (
                Some(*name),
                Model::Unit { name: Some(*name), parent: None, value: 0 },
            ),
        },
        // TODO: refactor this whole thing to do all the matching upfront
        // and only do strictly necessary binding at runtime
        Pat::Value(at) => match compile(at)?(env)? {
            ConValue::TypeInfo(t) => (None, t.to_ref().clone()),
            other => todo!("Pat::Value({other}) in struct binding")?,
        },
        Pat::Op(pat_op, pats) => bind_struct_op(*pat_op, pats, env)?,
    })
}

pub(crate) fn enum_binds(pat: &Pat, env: &mut Environment) -> IResult<(Sym, Vec<(Sym, Type)>)> {
    let name = pat
        .name()
        .ok_or_else(|| Error::PatFailed(Box::new(pat.clone())))?;
    let mut variants = vec![];
    if let Pat::Op(PatOp::TypePrefixed, _) = pat
        && let Pat::Op(PatOp::Record, pats) = pat.inner()
    {
        let mut scope = env.frame(name.to_ref(), Default::default(), None);
        match pat.generics() {
            Some(Pat::Op(PatOp::Tuple, vars)) => {
                for var in vars {
                    let Some(name) = var.value().name() else {
                        continue;
                    };
                    let t = Model::Any.intern();
                    scope.bind(name, ConValue::TypeInfo(t));
                }
            }
            Some(var) if let Some(name) = var.name() => {
                let t = Model::Any.intern();
                scope.bind(name, ConValue::TypeInfo(t));
            }
            _ => {}
        }
        for (idx, pat) in pats.iter().enumerate() {
            if let (Some(name), model) = struct_binds(pat.value(), &mut scope)? {
                let model = match model {
                    Model::Unit { name, parent, value: _ } => {
                        Model::Unit { name, parent, value: idx }
                    }
                    _ => model,
                };
                let typeid = scope.def_type(name, model.intern());
                variants.push((name, typeid));
            }
        }
    } else {
        todo!("Bind other enum: {pat:?}")?
    };

    Ok((name, variants))
}
