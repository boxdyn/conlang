//! [Place]-expressions in the interpreter
use crate::{
    convalue::ConValue,
    env::Environment,
    error::{Error, ErrorKind, IResult},
    interpret::{Interpret, todo},
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Projection {
    Deref,
    Index(usize, bool),
    DotSym(Symbol),
    DotIdx(usize),
}

impl Place {
    /// Creates a new [Place] [Projection] expression from an [Expr].
    ///
    /// If a place-projection can't be constructed, returns [Error::NotPlace].
    pub fn new(value: &Expr, env: &mut Environment) -> IResult<Self> {
        // base case
        if let Expr::Id(names) = value
            && let &[name] = names.parts.as_slice()
        {
            return Ok(Self { place: env.id_of(name)?, projections: vec![] });
        };
        let Expr::Op(op, exprs) = value else {
            return Err(Error::NotPlace());
        };

        match (op, exprs.as_slice()) {
            (Op::As, exprs) => Err(Error::NotPlace()),
            (Op::Block | Op::Group | Op::MetaInner | Op::MetaOuter | Op::Pub, [expr]) => {
                Self::new(expr.value(), env)
            }
            (Op::Index, [place, idx]) => {
                let place = Self::new(place.value(), env)?;
                // this may be surprising, as it evaluates an arbitrary expression!!!
                match idx.interpret(env)? {
                    ConValue::Int(idx @ ..0) => Ok(place.index(-idx as usize, true)),
                    ConValue::Int(idx @ 0..) => Ok(place.index(idx as usize, false)),
                    ConValue::TupleStruct(t, ref arr)
                        if let ([ConValue::Int(start), ConValue::Int(end)], "RangeExc") =
                            (&arr[..], t.0.name()) =>
                    {
                        todo!("Projection::Slice ({start}, {end})")
                    }
                    err => Err(Error::TypeError("int", err.type_of()))?,
                }
            }
            (Op::Dot, [place, At(Expr::Lit(Literal::Int(idx, _)), _)]) => {
                Ok(Self::new(place.value(), env)?.dot_idx(*idx as _))
            }
            (Op::Dot, [place, At(Expr::Id(path), _)]) if path.parts.len() == 1 => {
                Ok(Self::new(place.value(), env)?.dot_sym(path.parts[0]))
            }
            (Op::Deref, [place]) => Ok(Self::new(place.value(), env)?.deref()),
            _ => Err(Error::NotPlace()),
        }
    }

    /// Creates a new [Place] if this [Expr] is a place expression,
    /// otherwise allocates a new temporary and returns a Place-reference to it.
    pub fn new_or_temporary(value: &Expr, env: &mut Environment) -> IResult<Self> {
        let new = Self::new(value, env);
        if let Err(Error { kind: ErrorKind::NotPlace, .. }) = new {
            let value = value.interpret(env)?;
            return Ok(Self::from_index(env.stack_alloc(value)?));
        }
        new
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
    pub fn index(mut self, idx: usize, from_end: bool) -> Self {
        self.with(Projection::Index(idx, from_end))
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
            .ok_or(Error::StackOob(*place as _))?;

        for projection in projections {
            place = match (place, projection) {
                // SAFETY: fuck it, we ball. Polonius would totally have our back here
                (ConValue::Ref(place), Projection::Deref) => {
                    place.get_mut(unsafe { &mut *(env) })?
                }
                (ConValue::Ref(place), projection) => place
                    .clone()
                    .with(*projection)
                    .get_mut(unsafe { &mut *env })?,
                (value, Projection::Deref) => value,
                (ConValue::Array(arr), &Projection::Index(idx, from_end)) => {
                    let len = arr.len();
                    let idx = if from_end { len - idx } else { idx };
                    arr.get_mut(idx).ok_or_else(|| Error::OobIndex(idx, len))?
                }
                (ConValue::Slice(place, start, len), &Projection::Index(idx, from_end)) => {
                    if *start + idx >= *len {
                        Err(Error::OobIndex(idx, *len))?
                    };
                    // SAFETY: see above
                    place
                        .clone()
                        .index(idx, from_end)
                        .get_mut(unsafe { &mut *env })?
                }
                (place, Projection::Index(_, _)) => Err(Error::NotIndexable())?,
                (ConValue::Struct(_, values), Projection::DotSym(sym)) => {
                    values.get_mut(sym).ok_or(Error::NotDefined(*sym))?
                }
                (place, Projection::DotSym(name)) => Err(Error::TypeError(name, place.type_of()))?,
                (ConValue::Tuple(values), &Projection::DotIdx(idx)) => {
                    let len = values.len();
                    values.get_mut(idx).ok_or(Error::OobIndex(idx, len))?
                }
                (ConValue::TupleStruct(_, values), &Projection::DotIdx(idx)) => {
                    let len = values.len();
                    values.get_mut(idx).ok_or(Error::OobIndex(idx, len))?
                }
                (place, Projection::DotIdx(idx)) => todo!("{place}.{idx}")?,
                (value, place) => Err(Error::Panic(format!(
                    "Failed to match {value} against {place:?}"
                )))?,
            }
        }

        Ok(place)
    }

    pub fn get<'e>(&self, env: &'e Environment) -> IResult<&'e ConValue> {
        let Self { place, projections } = self;

        let mut place = env.get_id(*place).ok_or(Error::StackOob(*place as _))?;

        for projection in projections {
            place = match (place, projection) {
                (ConValue::Ref(place), Projection::Deref) => place.get(env)?,
                (ConValue::Ref(place), projection) => place.clone().with(*projection).get(env)?,
                (ConValue::Array(arr), &Projection::Index(idx, from_end)) => {
                    let len = arr.len();
                    let idx = if from_end { len - idx } else { idx };
                    arr.get(idx).ok_or(Error::OobIndex(idx, len))?
                }
                (ConValue::Slice(place, start, len), &Projection::Index(idx, from_end)) => {
                    let idx = if from_end { len - idx } else { idx };
                    place.clone().index(start + idx, false).get(env)?
                }
                (place, Projection::Index(_, _)) => todo!("Index {self}")?,
                (ConValue::Struct(_, values), Projection::DotSym(sym)) => {
                    values.get(sym).ok_or(Error::NotDefined(*sym))?
                }
                (place, Projection::DotSym(interned)) => todo!(".sym projection for {place}")?,
                (ConValue::Tuple(values), &Projection::DotIdx(idx)) => {
                    values.get(idx).ok_or(Error::OobIndex(idx, values.len()))?
                }
                (ConValue::TupleStruct(_, values), &Projection::DotIdx(idx)) => {
                    values.get(idx).ok_or(Error::OobIndex(idx, values.len()))?
                }
                (place, Projection::DotIdx(_)) => todo!(".idx projection for {place}")?,
                _ => Err(Error::NotPlace())?,
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
                [Projection::Index(idx, from_end), rest @ ..] => {
                    format_inner(place, rest, f)?;
                    write!(f, "[{}{idx}]", if *from_end { "-" } else { "" })
                }
                [Projection::DotIdx(idx), rest @ ..] => {
                    format_inner(place, rest, f)?;
                    write!(f, ".{idx}")
                }
                [Projection::DotSym(idx), rest @ ..] => {
                    format_inner(place, rest, f)?;
                    write!(f, ".{idx}")
                }
                _ => unreachable!("{projections:?}"),
            }
        }
        format_inner(*place, projections, f)
    }
}

#[derive(Clone, Debug)]
pub struct PlaceIndexIter {
    place: Place,
    start: usize,
    end: usize,
    projection: Projection,
}

impl PlaceIndexIter {
    pub fn index(place: Place, start: usize, length: usize) -> Self {
        Self { place, start, end: start + length, projection: Projection::Index(0, false) }
    }
    pub fn slice(place: Place, start: usize, end: usize) -> Self {
        Self { place, start, end, projection: Projection::Index(0, false) }
    }
    pub fn dot_idx(place: Place, start: usize, length: usize) -> Self {
        Self { place, start, end: start + length, projection: Projection::DotIdx(0) }
    }
}

impl Iterator for PlaceIndexIter {
    type Item = Place;

    fn next(&mut self) -> Option<Self::Item> {
        let Self { place, start, end, projection } = self;
        if start >= end {
            return None;
        }
        let out = match projection {
            Projection::Index(_, from_end) => place.clone().index(*start, *from_end),
            Projection::DotIdx(_) => place.clone().dot_idx(*start),
            _ => place.clone().with(*projection),
        };
        *start += 1;
        Some(out)
    }
}
