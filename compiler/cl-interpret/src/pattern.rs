//! Unification algorithm for cl-ast [Pattern]s and [ConValue]s
//!
//! [`variables()`] returns a flat list of symbols that are bound by a given pattern
//! [`substitution()`] unifies a ConValue with a pattern, and produces a list of bound names

use crate::{
    convalue::ConValue,
    env::Environment,
    error::{Error, IResult},
};
use cl_ast::{Literal, Pattern, Sym};
use std::collections::{HashMap, VecDeque};

/// Gets the path variables in the given Pattern
pub fn variables(pat: &Pattern) -> Vec<&Sym> {
    fn patvars<'p>(set: &mut Vec<&'p Sym>, pat: &'p Pattern) {
        match pat {
            Pattern::Name(name) if name.to_ref() == "_" => {}
            Pattern::Name(name) => set.push(name),
            Pattern::Path(_) => {}
            Pattern::Literal(_) => {}
            Pattern::Rest(Some(pattern)) => patvars(set, pattern),
            Pattern::Rest(None) => {}
            Pattern::Ref(_, pattern) => patvars(set, pattern),
            Pattern::RangeExc(_, _) => {}
            Pattern::RangeInc(_, _) => {}
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

fn rest_binding<'pat>(
    env: &Environment,
    sub: &mut HashMap<Sym, ConValue>,
    mut patterns: &'pat [Pattern],
    mut values: VecDeque<ConValue>,
) -> IResult<Option<(&'pat Pattern, VecDeque<ConValue>)>> {
    // Bind the head of the list
    while let [pattern, tail @ ..] = patterns {
        if matches!(pattern, Pattern::Rest(_)) {
            break;
        }
        let value = values
            .pop_front()
            .ok_or_else(|| Error::PatFailed(Box::new(pattern.clone())))?;
        append_sub(env, sub, pattern, value)?;
        patterns = tail;
    }
    // Bind the tail of the list
    while let [head @ .., pattern] = patterns {
        if matches!(pattern, Pattern::Rest(_)) {
            break;
        }
        let value = values
            .pop_back()
            .ok_or_else(|| Error::PatFailed(Box::new(pattern.clone())))?;
        append_sub(env, sub, pattern, value)?;
        patterns = head;
    }
    // Bind the ..rest of the list
    match patterns {
        [] | [Pattern::Rest(None)] => Ok(None),
        [Pattern::Rest(Some(pattern))] => Ok(Some((pattern.as_ref(), values))),
        _ => Err(Error::PatFailed(Box::new(Pattern::Array(patterns.into())))),
    }
}

fn rest_binding_ref<'pat>(
    env: &Environment,
    sub: &mut HashMap<Sym, ConValue>,
    mut patterns: &'pat [Pattern],
    mut head: usize,
    mut tail: usize,
) -> IResult<Option<(&'pat Pattern, usize, usize)>> {
    // Bind the head of the list
    while let [pattern, pat_tail @ ..] = patterns {
        if matches!(pattern, Pattern::Rest(_)) {
            break;
        }
        if head >= tail {
            return Err(Error::PatFailed(Box::new(pattern.clone())));
        }

        append_sub(env, sub, pattern, ConValue::Ref(head))?;
        head += 1;
        patterns = pat_tail;
    }
    // Bind the tail of the list
    while let [pat_head @ .., pattern] = patterns {
        if matches!(pattern, Pattern::Rest(_)) {
            break;
        }
        if head >= tail {
            return Err(Error::PatFailed(Box::new(pattern.clone())));
        };
        append_sub(env, sub, pattern, ConValue::Ref(tail))?;
        tail -= 1;
        patterns = pat_head;
    }
    // Bind the ..rest of the list
    match (patterns, tail - head) {
        ([], 0) | ([Pattern::Rest(None)], _) => Ok(None),
        ([Pattern::Rest(Some(pattern))], _) => Ok(Some((pattern.as_ref(), head, tail))),
        _ => Err(Error::PatFailed(Box::new(Pattern::Array(patterns.into())))),
    }
}

/// Appends a substitution to the provided table
pub fn append_sub(
    env: &Environment,
    sub: &mut HashMap<Sym, ConValue>,
    pat: &Pattern,
    value: ConValue,
) -> IResult<()> {
    match (pat, value) {
        (Pattern::Literal(Literal::Bool(a)), ConValue::Bool(b)) => {
            (*a == b).then_some(()).ok_or(Error::NotAssignable())
        }
        (Pattern::Literal(Literal::Char(a)), ConValue::Char(b)) => {
            (*a == b).then_some(()).ok_or(Error::NotAssignable())
        }
        (Pattern::Literal(Literal::Float(a)), ConValue::Float(b)) => (f64::from_bits(*a) == b)
            .then_some(())
            .ok_or(Error::NotAssignable()),
        (Pattern::Literal(Literal::Int(a)), ConValue::Int(b)) => {
            (b == *a as _).then_some(()).ok_or(Error::NotAssignable())
        }
        (Pattern::Literal(Literal::String(a)), ConValue::Str(b)) => {
            (*a == *b).then_some(()).ok_or(Error::NotAssignable())
        }
        (Pattern::Literal(Literal::String(a)), ConValue::String(b)) => {
            (*a == *b).then_some(()).ok_or(Error::NotAssignable())
        }
        (Pattern::Literal(_), _) => Err(Error::NotAssignable()),

        (Pattern::Rest(Some(pat)), value) => match (pat.as_ref(), value) {
            (Pattern::Literal(Literal::Int(a)), ConValue::Int(b)) => {
                (b < *a as _).then_some(()).ok_or(Error::NotAssignable())
            }
            (Pattern::Literal(Literal::Char(a)), ConValue::Char(b)) => {
                (b < *a as _).then_some(()).ok_or(Error::NotAssignable())
            }
            (Pattern::Literal(Literal::Bool(a)), ConValue::Bool(b)) => {
                (!b & *a).then_some(()).ok_or(Error::NotAssignable())
            }
            (Pattern::Literal(Literal::Float(a)), ConValue::Float(b)) => {
                (b < *a as _).then_some(()).ok_or(Error::NotAssignable())
            }
            (Pattern::Literal(Literal::String(a)), ConValue::Str(b)) => {
                (&*b < a).then_some(()).ok_or(Error::NotAssignable())
            }
            (Pattern::Literal(Literal::String(a)), ConValue::String(b)) => {
                (&*b < a).then_some(()).ok_or(Error::NotAssignable())
            }
            _ => Err(Error::NotAssignable()),
        },

        (Pattern::Name(name), _) if "_".eq(&**name) => Ok(()),
        (Pattern::Name(name), value) => {
            sub.insert(*name, value);
            Ok(())
        }

        (Pattern::Ref(_, pat), ConValue::Ref(r)) => match env.get_id(r) {
            Some(value) => append_sub(env, sub, pat, value.clone()),
            None => Err(Error::PatFailed(pat.clone())),
        },

        (Pattern::Ref(_, pat), ConValue::Slice(head, len)) => {
            let mut values = Vec::with_capacity(len);
            for idx in head..(head + len) {
                values.push(env.get_id(idx).cloned().ok_or(Error::StackOverflow(idx))?);
            }
            append_sub(env, sub, pat, ConValue::Array(values.into_boxed_slice()))
        }

        (Pattern::RangeExc(head, tail), value) => match (head.as_ref(), tail.as_ref(), value) {
            (
                Pattern::Literal(Literal::Int(a)),
                Pattern::Literal(Literal::Int(c)),
                ConValue::Int(b),
            ) => (*a as isize <= b as _ && b < *c as isize)
                .then_some(())
                .ok_or(Error::NotAssignable()),
            (
                Pattern::Literal(Literal::Char(a)),
                Pattern::Literal(Literal::Char(c)),
                ConValue::Char(b),
            ) => (*a <= b && b < *c)
                .then_some(())
                .ok_or(Error::NotAssignable()),
            (
                Pattern::Literal(Literal::Float(a)),
                Pattern::Literal(Literal::Float(c)),
                ConValue::Float(b),
            ) => (f64::from_bits(*a) <= b && b < f64::from_bits(*c))
                .then_some(())
                .ok_or(Error::NotAssignable()),
            (
                Pattern::Literal(Literal::String(a)),
                Pattern::Literal(Literal::String(c)),
                ConValue::Str(b),
            ) => (a.as_str() <= b.to_ref() && b.to_ref() < c.as_str())
                .then_some(())
                .ok_or(Error::NotAssignable()),
            (
                Pattern::Literal(Literal::String(a)),
                Pattern::Literal(Literal::String(c)),
                ConValue::String(b),
            ) => (a.as_str() <= b.as_str() && b.as_str() < c.as_str())
                .then_some(())
                .ok_or(Error::NotAssignable()),
            _ => Err(Error::NotAssignable()),
        },

        (Pattern::RangeInc(head, tail), value) => match (head.as_ref(), tail.as_ref(), value) {
            (
                Pattern::Literal(Literal::Int(a)),
                Pattern::Literal(Literal::Int(c)),
                ConValue::Int(b),
            ) => (*a as isize <= b && b <= *c as isize)
                .then_some(())
                .ok_or(Error::NotAssignable()),
            (
                Pattern::Literal(Literal::Char(a)),
                Pattern::Literal(Literal::Char(c)),
                ConValue::Char(b),
            ) => (*a <= b && b <= *c)
                .then_some(())
                .ok_or(Error::NotAssignable()),
            (
                Pattern::Literal(Literal::Float(a)),
                Pattern::Literal(Literal::Float(c)),
                ConValue::Float(b),
            ) => (f64::from_bits(*a) <= b && b <= f64::from_bits(*c))
                .then_some(())
                .ok_or(Error::NotAssignable()),
            (
                Pattern::Literal(Literal::String(a)),
                Pattern::Literal(Literal::String(c)),
                ConValue::Str(b),
            ) => (a.as_str() <= b.to_ref() && b.to_ref() <= c.as_str())
                .then_some(())
                .ok_or(Error::NotAssignable()),
            (
                Pattern::Literal(Literal::String(a)),
                Pattern::Literal(Literal::String(c)),
                ConValue::String(b),
            ) => (a.as_str() <= b.as_str() && b.as_str() <= c.as_str())
                .then_some(())
                .ok_or(Error::NotAssignable()),
            _ => Err(Error::NotAssignable()),
        },

        (Pattern::Array(patterns), ConValue::Array(values)) => {
            match rest_binding(env, sub, patterns, values.into_vec().into())? {
                Some((pattern, values)) => {
                    append_sub(env, sub, pattern, ConValue::Array(Vec::from(values).into()))
                }
                _ => Ok(()),
            }
        }

        (Pattern::Array(patterns), ConValue::Slice(head, len)) => {
            match rest_binding_ref(env, sub, patterns, head, head + len)? {
                Some((pat, head, tail)) => {
                    append_sub(env, sub, pat, ConValue::Slice(head, tail - head))
                }
                None => Ok(()),
            }
        }

        (Pattern::Tuple(patterns), ConValue::Empty) if patterns.is_empty() => Ok(()),
        (Pattern::Tuple(patterns), ConValue::Tuple(values)) => {
            match rest_binding(env, sub, patterns, values.into_vec().into())? {
                Some((pattern, values)) => {
                    append_sub(env, sub, pattern, ConValue::Tuple(Vec::from(values).into()))
                }
                _ => Ok(()),
            }
        }

        (Pattern::TupleStruct(path, patterns), ConValue::TupleStruct(parts)) => {
            let (id, values) = *parts;

            let tid = path
                .as_sym()
                .ok_or_else(|| Error::PatFailed(pat.clone().into()))?;
            if id != tid.to_ref() {
                return Err(Error::PatFailed(pat.clone().into()));
            }
            match rest_binding(env, sub, patterns, values.into_vec().into())? {
                Some((pattern, values)) => {
                    append_sub(env, sub, pattern, ConValue::Tuple(Vec::from(values).into()))
                }
                _ => Ok(()),
            }
        }

        (Pattern::Struct(path, patterns), ConValue::Struct(parts)) => {
            let (id, mut values) = *parts;
            let tid = path
                .as_sym()
                .ok_or_else(|| Error::PatFailed(pat.clone().into()))?;
            if id != tid.to_ref() {
                return Err(Error::PatFailed(pat.clone().into()));
            }
            for (name, pat) in patterns {
                let value = values.remove(name).ok_or(Error::TypeError())?;
                match pat {
                    Some(pat) => append_sub(env, sub, pat, value)?,
                    None => {
                        sub.insert(*name, value);
                    }
                }
            }
            Ok(())
        }

        _ => {
            // eprintln!("Could not match pattern `{pat}` with value `{value}`!");
            Err(Error::NotAssignable())
        }
    }
}

/// Constructs a substitution from a pattern and a value
pub fn substitution(
    env: &Environment,
    pat: &Pattern,
    value: ConValue,
) -> IResult<HashMap<Sym, ConValue>> {
    let mut sub = HashMap::new();
    append_sub(env, &mut sub, pat, value)?;
    Ok(sub)
}
