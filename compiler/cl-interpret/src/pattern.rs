//! Unification algorithm for cl-ast patterns and ConValues
//!
//! [`variables()`] returns a flat list of symbols that are bound by a given pattern
//! [`substitution()`] unifies a ConValue with a pattern, and produces a list of bound names

use crate::{
    convalue::ConValue,
    error::{Error, IResult},
};
use cl_ast::{Literal, Pattern, Sym};
use std::{collections::HashMap, rc::Rc};

/// Gets the path variables in the given Pattern
pub fn variables(pat: &Pattern) -> Vec<&Sym> {
    fn patvars<'p>(set: &mut Vec<&'p Sym>, pat: &'p Pattern) {
        match pat {
            Pattern::Name(name) if &**name == "_" => {}
            Pattern::Name(name) => set.push(name),
            Pattern::Literal(_) => {}
            Pattern::Ref(_, pattern) => patvars(set, pattern),
            Pattern::Tuple(patterns) | Pattern::Array(patterns) => {
                patterns.iter().for_each(|pat| patvars(set, pat))
            }
            Pattern::Struct(_path, items) => {
                items.iter().for_each(|(name, pat)| match pat {
                    Some(pat) => patvars(set, pat),
                    None => set.push(name),
                });
            }
            Pattern::TupleStruct(_path, items) => {
                items.iter().for_each(|pat| patvars(set, pat));
            }
        }
    }
    let mut set = Vec::new();
    patvars(&mut set, pat);
    set
}

/// Appends a substitution to the provided table
pub fn append_sub<'pat>(
    sub: &mut HashMap<&'pat Sym, ConValue>,
    pat: &'pat Pattern,
    value: ConValue,
) -> IResult<()> {
    match (pat, value) {
        (Pattern::Array(patterns), ConValue::Array(values))
        | (Pattern::Tuple(patterns), ConValue::Tuple(values)) => {
            if patterns.len() != values.len() {
                Err(Error::ArgNumber { want: patterns.len(), got: values.len() })?
            }
            for (pat, value) in patterns.iter().zip(Vec::from(values).into_iter()) {
                append_sub(sub, pat, value)?;
            }
            Ok(())
        }
        (Pattern::Tuple(patterns), ConValue::Empty) if patterns.is_empty() => Ok(()),

        (Pattern::Literal(Literal::Bool(a)), ConValue::Bool(b)) => {
            (*a == b).then_some(()).ok_or(Error::NotAssignable)
        }
        (Pattern::Literal(Literal::Char(a)), ConValue::Char(b)) => {
            (*a == b).then_some(()).ok_or(Error::NotAssignable)
        }
        (Pattern::Literal(Literal::Float(a)), ConValue::Float(b)) => (f64::from_bits(*a) == b)
            .then_some(())
            .ok_or(Error::NotAssignable),
        (Pattern::Literal(Literal::Int(a)), ConValue::Int(b)) => {
            (b == *a as _).then_some(()).ok_or(Error::NotAssignable)
        }
        (Pattern::Literal(Literal::String(a)), ConValue::String(b)) => {
            (*a == *b).then_some(()).ok_or(Error::NotAssignable)
        }
        (Pattern::Literal(_), _) => Err(Error::NotAssignable),

        (Pattern::Name(name), _) if "_".eq(&**name) => Ok(()),
        (Pattern::Name(name), value) => {
            sub.insert(name, value);
            Ok(())
        }

        (Pattern::Ref(_, pat), ConValue::Ref(r)) => append_sub(sub, pat, Rc::unwrap_or_clone(r)),

        (Pattern::Struct(path, patterns), ConValue::Struct(parts)) => {
            let (name, mut values) = *parts;
            if !path.ends_with(&name) {
                Err(Error::TypeError)?
            }
            if patterns.len() != values.len() {
                return Err(Error::ArgNumber { want: patterns.len(), got: values.len() });
            }
            for (name, pat) in patterns {
                let value = values.remove(name).ok_or(Error::TypeError)?;
                match pat {
                    Some(pat) => append_sub(sub, pat, value)?,
                    None => {
                        sub.insert(name, value);
                    }
                }
            }
            Ok(())
        }

        (Pattern::TupleStruct(path, patterns), ConValue::TupleStruct(parts)) => {
            let (name, values) = *parts;
            if !path.ends_with(&name) {
                Err(Error::TypeError)?
            }
            if patterns.len() != values.len() {
                Err(Error::ArgNumber { want: patterns.len(), got: values.len() })?
            }
            for (pat, value) in patterns.iter().zip(Vec::from(values).into_iter()) {
                append_sub(sub, pat, value)?;
            }
            Ok(())
        }

        (pat, value) => {
            eprintln!("Could not match pattern `{pat}` with value `{value}`!");
            Err(Error::NotAssignable)
        }
    }
}

/// Constructs a substitution from a pattern and a value
pub fn substitution(pat: &Pattern, value: ConValue) -> IResult<HashMap<&Sym, ConValue>> {
    let mut sub = HashMap::new();
    append_sub(&mut sub, pat, value)?;
    Ok(sub)
}
