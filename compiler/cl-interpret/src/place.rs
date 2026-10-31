//! [Place]-expressions in the interpreter
use crate::{
    Callable,
    convalue::ConValue,
    env::{Environment, Location},
    error::{Error, IResult},
    interpret::todo,
    typeinfo::Model,
};
use cl_ast::types::Symbol;
use std::fmt::Display;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Place {
    place: Location,
    projections: Vec<Projection>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Projection {
    Deref,
    Index(usize, bool),
    DotSym(Symbol),
    DotIdx(usize),
}

impl Callable for Place {
    fn call(&self, env: &mut Environment, args: Vec<ConValue>) -> IResult<ConValue> {
        match self.get(env)? {
            ConValue::Struct(ty, _) | ConValue::TupleStruct(ty, _) => {
                let func = env.get_impl(*ty, env.kw.call)?;
                let mut self_args = vec![ConValue::Ref(self.clone())];
                self_args.extend(args);
                func.call(env, self_args)
            }
            module @ ConValue::Module(m) => match m.get(&env.kw.call) {
                Some(func) => func.clone().call(env, args),
                None => Err(Error::NotCallable(module.clone())),
            },
            other => other.clone().call(env, args),
        }
    }

    fn name(&self) -> Option<Symbol> {
        None
    }
}

impl Place {
    pub fn from_index(index: Location) -> Self {
        Self { place: index, projections: vec![] }
    }

    pub fn with(mut self, projection: Projection) -> Self {
        self.projections.push(projection);
        self
    }
    pub fn deref(self) -> Self {
        self.with(Projection::Deref)
    }
    pub fn index(self, idx: usize, from_end: bool) -> Self {
        self.with(Projection::Index(idx, from_end))
    }
    pub fn dot_idx(self, idx: usize) -> Self {
        self.with(Projection::DotIdx(idx))
    }
    pub fn dot_sym(self, sym: Symbol) -> Self {
        self.with(Projection::DotSym(sym))
    }

    pub fn get_mut<'e>(&self, env: &'e mut Environment) -> IResult<&'e mut ConValue> {
        let Self { place, projections } = self;
        let mut place = env
            .get_id_mut(*place)
            .ok_or(Error::LocationOob(*place as _))?;

        for projection in projections {
            place = match (place, projection) {
                (ConValue::Ref(place), Projection::Deref) => place.clone().get_mut(env)?,
                (ConValue::Ref(place), projection) => {
                    place.clone().with(*projection).get_mut(env)?
                }
                (value, Projection::Deref) => value,
                (ConValue::Array(arr), &Projection::Index(idx, from_end)) => {
                    let len = arr.len();
                    let idx = if from_end { len - idx } else { idx };
                    arr.get_mut(idx).ok_or_else(|| Error::OobIndex(idx, len))?
                }
                (_place, Projection::Index(_, _)) => Err(Error::NotIndexable())?,
                (ConValue::Struct(ty, values), Projection::DotSym(sym)) => match ty.to_ref() {
                    Model::Struct { exhaustive: false, .. } => values.entry(*sym).or_default(),
                    _ => values.get_mut(sym).ok_or(Error::NotDefined(*sym))?,
                },
                (place, Projection::DotSym(name)) => {
                    Err(Error::TypeError(name, place.clone().type_of(env)))?
                }
                (ConValue::Tuple(values), &Projection::DotIdx(idx)) => {
                    let len = values.len();
                    values.get_mut(idx).ok_or(Error::OobIndex(idx, len))?
                }
                (ConValue::TupleStruct(_, values), &Projection::DotIdx(idx)) => {
                    let len = values.len();
                    values.get_mut(idx).ok_or(Error::OobIndex(idx, len))?
                }
                (place, Projection::DotIdx(idx)) => todo!("{place}.{idx}")?,
            }
        }

        Ok(place)
    }

    pub fn get<'e>(&self, env: &'e Environment) -> IResult<&'e ConValue> {
        let Self { place, projections } = self;
        let mut value = env.get_id(*place).ok_or(Error::LocationOob(*place as _))?;

        for projection in projections {
            value = match (value, projection) {
                (ConValue::Ref(place), Projection::Deref) => place.get(env)?,
                (ConValue::Ref(next), projection) if next.place != *place => {
                    next.clone().with(*projection).get(env)?
                }
                (ConValue::Array(arr), &Projection::Index(idx, from_end)) => {
                    let len = arr.len();
                    let idx = if from_end { len.wrapping_sub(idx) } else { idx };
                    arr.get(idx).ok_or(Error::OobIndex(idx, len))?
                }
                (_place, Projection::Index(_, _)) => todo!("Index {self}")?,
                (ConValue::Struct(_, values), Projection::DotSym(sym)) => {
                    values.get(sym).ok_or(Error::NotDefined(*sym))?
                }
                (place, Projection::DotSym(sym)) => todo!(".{sym} projection for {place}")?,
                (ConValue::Tuple(values), &Projection::DotIdx(idx)) => {
                    values.get(idx).ok_or(Error::OobIndex(idx, values.len()))?
                }
                (ConValue::TupleStruct(_, values), &Projection::DotIdx(idx)) => {
                    values.get(idx).ok_or(Error::OobIndex(idx, values.len()))?
                }
                (place, Projection::DotIdx(idx)) => todo!(".{idx} projection for {place}")?,
                (value, place) => Err(Error::Panic(format!(
                    "Failed to match {value} against {place:?}"
                )))?,
            }
        }

        Ok(value)
    }
}

impl Display for Place {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { place, projections } = self;
        fn format_inner(
            place: Location,
            projections: &[Projection],
            f: &mut std::fmt::Formatter<'_>,
        ) -> std::fmt::Result {
            match projections {
                [] => write!(f, "{place:?}"),
                [first @ .., Projection::Deref] => {
                    "*".fmt(f)?;
                    format_inner(place, first, f)
                }
                [rest @ .., Projection::Index(idx, from_end)] => {
                    format_inner(place, rest, f)?;
                    write!(f, "[{}{idx}]", if *from_end { "-" } else { "" })
                }
                [rest @ .., Projection::DotIdx(idx)] => {
                    format_inner(place, rest, f)?;
                    write!(f, ".{idx}")
                }
                [rest @ .., Projection::DotSym(idx)] => {
                    format_inner(place, rest, f)?;
                    write!(f, ".{idx}")
                }
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
