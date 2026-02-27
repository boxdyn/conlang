//! Place-expressions in the interpreter
use crate::{
    convalue::ConValue,
    env::Environment,
    error::{Error, IResult},
    interpret::Interpret,
};
use cl_ast::{
    At, Expr, Op,
    types::{Literal, Symbol},
};
use std::{fmt::Display, mem::take};

#[derive(Clone, Debug, Default)]
pub struct Place {
    place: usize,
    projections: Vec<Projection>,
}

#[derive(Clone, Debug)]
pub enum Projection {
    Deref,
    Index(usize),
    DotSym(Symbol),
    DotIdx(usize),
}

impl Place {
    pub fn new(value: &Expr, env: &mut Environment) -> IResult<Self> {
        // base case
        if let Expr::Id(names) = value
            && let &[name] = names.parts.as_slice()
        {
            return Ok(Self { place: env.id_of(name)?, projections: vec![] });
        };
        let Expr::Op(op, exprs) = value else {
            return Err(Error::NotAssignable());
        };

        match (op, exprs.as_slice()) {
            (Op::As, exprs) => Err(Error::NotAssignable()),
            (Op::Block | Op::Group | Op::MetaInner | Op::MetaOuter | Op::Pub, [expr]) => {
                Self::new(expr.value(), env)
            }
            (Op::Index, [place, idx]) => {
                let place = Self::new(place.value(), env)?;
                // this may be surprising, as it evaluates an arbitrary expression!!!
                match idx.interpret(env)? {
                    ConValue::Int(idx) => Ok(place.index(idx as usize)),
                    err => Err(Error::TypeError("int", err.typename()))?,
                }
            }
            (Op::Dot, [place, At(Expr::Lit(Literal::Int(idx, _)), _)]) => {
                Ok(Self::new(place.value(), env)?.dot_idx(*idx as _))
            }
            (Op::Dot, [place, At(Expr::Id(path), _)]) if path.parts.len() == 1 => {
                Ok(Self::new(place.value(), env)?.dot_sym(path.parts[0]))
            }
            (Op::Deref, [place]) => Ok(Self::new(place.value(), env)?.deref()),
            _ => Err(Error::NotAssignable()),
        }
    }

    pub fn from_index(index: usize) -> Self {
        Self { place: index, projections: vec![] }
    }

    pub fn with(mut self, projection: Projection) -> Self {
        self.projections.push(projection);
        self
    }
    pub fn deref(mut self) -> Self {
        self.with(Projection::Deref)
    }
    pub fn index(mut self, idx: usize) -> Self {
        self.with(Projection::Index(idx))
    }
    pub fn dot_idx(mut self, idx: usize) -> Self {
        self.with(Projection::DotIdx(idx))
    }
    pub fn dot_sym(mut self, sym: impl Into<Symbol>) -> Self {
        self.with(Projection::DotSym(sym.into()))
    }

    pub fn get_mut<'v, 'e: 'v>(&self, env: &'e mut Environment) -> IResult<&'v mut ConValue> {
        let Self { place, projections } = self;
        let env = env as *mut Environment;

        let mut place = unsafe { &mut *env }
            .get_id_mut(*place)
            .ok_or(Error::StackOverflow(*place as _))?;

        for projection in projections {
            place = match (place, projection) {
                // SAFETY: fuck it, we ball. Polonius would totally have our back here
                (ConValue::Ref(place), Projection::Deref) => {
                    place.get_mut(unsafe { &mut *(env) })?
                }
                (value, Projection::Deref) => value,
                (ConValue::Array(arr), &Projection::Index(idx)) => {
                    let len = arr.len();
                    arr.get_mut(idx).ok_or_else(|| Error::OobIndex(idx, len))?
                }
                (ConValue::Slice(place, len), &Projection::Index(idx)) => {
                    if idx >= *len {
                        Err(Error::OobIndex(idx, *len))?
                    };
                    // SAFETY: see above
                    place.clone().index(idx).get_mut(unsafe { &mut *env })?
                }
                (place, Projection::Index(idx)) => Err(Error::NotIndexable())?,
                (ConValue::Struct(_, values), Projection::DotSym(sym)) => {
                    values.get_mut(sym).ok_or(Error::NotDefined(*sym))?
                }
                (place, Projection::DotSym(name)) => {
                    Err(Error::TypeError(name.to_ref(), place.typename()))?
                }
                (ConValue::Tuple(values), &Projection::DotIdx(idx)) => {
                    let len = values.len();
                    values.get_mut(idx).ok_or(Error::OobIndex(idx, len))?
                }
                (ConValue::TupleStruct(_, values), &Projection::DotIdx(idx)) => {
                    let len = values.len();
                    values.get_mut(idx).ok_or(Error::OobIndex(idx, len))?
                }
                (place, Projection::DotIdx(_)) => todo!(),
                (value, place) => Err(Error::Panic(format!(
                    "Failed to match {value} against {place:?}"
                )))?,
            }
        }

        Ok(place)
    }

    pub fn get<'e>(&self, env: &'e Environment) -> IResult<&'e ConValue> {
        let Self { place, projections } = self;

        let mut place = env
            .get_id(*place)
            .ok_or(Error::StackOverflow(*place as _))?;

        for projection in projections {
            place = match (place, projection) {
                (ConValue::Ref(reference), Projection::Deref) => reference.get(env)?,
                (ConValue::Array(arr), &Projection::Index(idx)) => {
                    arr.get(idx).ok_or(Error::OobIndex(idx, arr.len()))?
                }
                (place, Projection::Index(idx)) => todo!(),
                (place, Projection::DotSym(interned)) => todo!(),
                (ConValue::Tuple(values), &Projection::DotIdx(idx)) => {
                    values.get(idx).ok_or(Error::OobIndex(idx, values.len()))?
                }
                (ConValue::TupleStruct(_, values), &Projection::DotIdx(idx)) => {
                    values.get(idx).ok_or(Error::OobIndex(idx, values.len()))?
                }
                (place, Projection::DotIdx(_)) => todo!(),
                _ => Err(Error::MatchNonexhaustive())?,
            }
        }

        Ok(place)
    }
}

impl Display for Place {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { place, projections } = self;
        fn format_inner(
            place: usize,
            projections: &[Projection],
            f: &mut std::fmt::Formatter<'_>,
        ) -> std::fmt::Result {
            match projections {
                [] => place.fmt(f),
                [first @ .., Projection::Deref] => {
                    "*".fmt(f)?;
                    format_inner(place, first, f)
                }
                [Projection::DotIdx(idx), rest @ ..] => {
                    format_inner(place, rest, f)?;
                    write!(f, ".{idx}")
                }
                [Projection::DotSym(idx), rest @ ..] => {
                    format_inner(place, rest, f)?;
                    write!(f, ".{idx}")
                }
                [Projection::Index(idx), rest @ ..] => {
                    format_inner(place, rest, f)?;
                    write!(f, "[{idx}]")
                }
                _ => unreachable!("{projections:?}"),
            }
        }
        format_inner(*place, projections, f)
    }
}
