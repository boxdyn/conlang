use std::{collections::HashMap, fmt::Display, sync::OnceLock};

use cl_ast::{Pat, PatOp, fmt::FmtAdapter, types::Symbol};
use cl_structures::intern::{
    interned::Interned, leaky_interner::LeakyInterner, string_interner::StringInterner,
};

use crate::{
    Callable,
    convalue::ConValue,
    env::Environment,
    error::{Error, IResult},
};

pub type TypeId = usize;
pub type Type = Interned<'static, TypeInfo>;

pub(crate) static TYPE_INTERNER: OnceLock<LeakyInterner<TypeInfo>> = OnceLock::new();

/// The elements of a type's value
#[rustfmt::skip]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Model {
    Integer { signed: bool, size: usize, min: i128, max: i128 },
    Float { size: usize },
    Bool,
    Char,
    Str,
    Any,
    /// The "never" type (no variants)
    Never,
    /// The unit type (no elements)
    Unit(usize),
    /// Reference to a value of [Type]
    Ref(Type),
    /// Slice of a list of [Type]
    Slice(Type),
    /// The elements of a tuple
    Tuple(Box<[Type]>),
    /// The elements of a struct, and whether they are exhaustive
    Struct(Box<[(Symbol, Type)]>, bool),
    /// The variants of an enumeration
    Enum(Box<[(Symbol, Type)]>),
}

impl Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;
        match self {
            Self::Integer { signed, size, .. } => {
                write!(f, "{}{}", if *signed { "i" } else { "u" }, size * 8)
            }
            Self::Float { size } => write!(f, "f{}", size * 8),
            Self::Bool => "bool".fmt(f),
            Self::Char => "char".fmt(f),
            Self::Str => "str".fmt(f),
            Self::Any => "_".fmt(f),
            Self::Never => "!".fmt(f),
            Self::Unit(_) => "()".fmt(f),
            Self::Ref(t) => write!(f, "&{t}"),
            Self::Slice(t) => write!(f, "[{t}]"),
            Self::Tuple(items) => f.delimit("(", ")").list(items, ", "),
            Self::Struct(items, _) => {
                let mut f = f.delimit("{", " }");
                for (idx, (name, ty)) in items.iter().enumerate() {
                    if idx > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, " {name}: {ty}")?;
                }
                Ok(())
            }
            Self::Enum(items) => {
                let mut f = f.delimit_indented("enum {", "\n}");
                for (name, idx) in items {
                    write!(f, "\n{name}: {idx},")?;
                }
                Ok(())
            }
        }
    }
}

/// The unabridged information for a type
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeInfo {
    pub ident: Option<Symbol>,
    pub model: Model,
}

macro make_int($T:ty, $signed: expr) {
    Model::Integer {
        signed: $signed,
        size: size_of::<$T>(),
        min: <$T>::MIN as _,
        max: <$T>::MAX as _,
    }
}

impl TypeInfo {
    pub fn new(ident: impl Into<Symbol>, model: Model) -> Self {
        Self { ident: Some(ident.into()), model }
    }

    pub fn name(&self) -> &'static str {
        match self.ident {
            Some(name) => name.to_ref(),
            None => "_",
        }
    }

    pub fn default_int() -> Type {
        Self::new("i128", make_int!(i128, true)).intern()
    }

    #[rustfmt::skip]
    pub fn defaults() -> Vec<Self> {
        let unknown = Self::new("_", Model::Any).intern();
        let types = [
            Self::new("_", Model::Any),
            Self::new("unit", Model::Unit(0)),
            Self::new("bool", Model::Bool),
            Self::new("char", Model::Char),
            Self::new("str", Model::Str),
            Self::new("never", Model::Never),
            Self::new("f32", Model::Float{size: size_of::<f32>()}),
            Self::new("f64", Model::Float{size: size_of::<f64>()}),
            Self::new("i8", make_int!(i8, true)),
            Self::new("i16", make_int!(i16, true)),
            Self::new("i32", make_int!(i32, true)),
            Self::new("i64", make_int!(i64, true)),
            Self::new("i128", make_int!(i128, true)),
            Self::new("isize", make_int!(isize, true)),
            Self::new("int", make_int!(isize, true)),
            Self::new("u8", make_int!(u8, false)),
            Self::new("u16", make_int!(u16, false)),
            Self::new("u32", make_int!(u32, false)),
            Self::new("u64", make_int!(u64, false)),
            Self::new("u128", make_int!(u128, false)),
            Self::new("usize", make_int!(usize, false)),
            Self::new("uint", make_int!(usize, false)),
            Self::new("RangeExc", Model::Tuple([unknown, unknown].into())),
            Self::new("RangeInc", Model::Tuple([unknown, unknown].into())),
            Self::new("RangeTo", Model::Tuple([unknown].into())),
            Self::new("RangeToInc", Model::Tuple([unknown].into())),
        ];
        types.into()
    }

    pub fn getattr(&self, attr: Symbol) -> IResult<ConValue> {
        Ok(match (&self.model, attr.0) {
            (_, "Self") => ConValue::TypeInfo(self.already_interned()),
            (&Model::Integer { signed, .. }, "is_signed") => ConValue::Bool(signed),
            (&Model::Integer { size, .. }, "size") => ConValue::Int(size as _),
            (&Model::Integer { size, .. }, "bits") => ConValue::Int(8 * size as i128),
            (&Model::Integer { min, .. }, "min") => ConValue::Int(min),
            (&Model::Integer { max, .. }, "max") => ConValue::Int(max),
            (&Model::Float { size }, "size") => ConValue::Int(size as _),
            (&Model::Float { .. }, "inf") => ConValue::Float(f64::INFINITY),
            (&Model::Float { .. }, "nan") => ConValue::Float(f64::NAN),
            (Model::Bool, "size") => ConValue::Int(size_of::<bool>() as _),
            (Model::Char, "size") => ConValue::Int(size_of::<char>() as _),
            (Model::Never, _) => Err(Error::NotDefined(attr))?,
            (Model::Unit(_), "size") => ConValue::Int(0),
            (Model::Unit(_), _) => Err(Error::NotDefined(attr))?,
            (Model::Tuple(items), "ARITY") => ConValue::Int(items.len() as _),
            (Model::Struct(items, _), "NAMES") => {
                ConValue::Array(items.iter().map(|(n, _)| ConValue::Str(*n)).collect())
            }
            (Model::Struct(items, _), "TYPES") => {
                ConValue::Array(items.iter().map(|(_, t)| ConValue::TypeInfo(*t)).collect())
            }
            (Model::Struct(items, _), "MEMBERS") => ConValue::Array(
                items
                    .iter()
                    .map(|(n, t)| {
                        ConValue::Tuple([ConValue::Str(*n), ConValue::TypeInfo(*t)].into())
                    })
                    .collect(),
            ),
            (Model::Struct(items, exhaustive), _) => items
                .iter()
                .find_map(|&(name, ty)| (name == attr).then_some(ConValue::TypeInfo(ty)))
                .ok_or(Error::NotDefined(attr))?,
            (Model::Enum(items), _) => items
                .iter()
                .find_map(|&(name, ty)| (name == attr).then_some(ConValue::TypeInfo(ty)))
                .ok_or(Error::NotDefined(attr))?,
            (model, _) => Err(Error::NotDefined(attr))?,
        })
    }

    pub fn make_tuple(&self, values: Box<[ConValue]>) -> IResult<ConValue> {
        let Model::Tuple(typeids) = &self.model else {
            Err(Error::TypeError(self.name(), "tuple struct"))?
        };
        if typeids.len() != values.len() {
            return Err(Error::ArgNumber(typeids.len(), values.len()));
        }
        Ok(ConValue::TupleStruct(self.already_interned(), values))
    }

    pub fn make_struct(&self, mut values: HashMap<Symbol, ConValue>) -> IResult<ConValue> {
        let Model::Struct(model, exhaustive) = &self.model else {
            Err(Error::TypeError(self.name(), "struct"))?
        };

        let mut members = HashMap::new();
        if *exhaustive {
            for (key, _id) in model {
                let value = values.get_mut(key).ok_or(Error::NotInitialized(*key))?;
                members.insert(*key, value.take());
            }
        } else {
            members = values;
        }

        Ok(ConValue::Struct(self.already_interned(), Box::new(members)))
    }

    pub fn intern(self) -> Type {
        TYPE_INTERNER
            .get_or_init(LeakyInterner::new)
            .get_or_insert(self)
    }
    pub fn already_interned(&self) -> Type {
        TYPE_INTERNER
            .get_or_init(LeakyInterner::new)
            .get(self)
            .unwrap_or_else(|| panic!("{}", self.name()))
    }
}

impl Callable for TypeInfo {
    fn call(&self, env: &mut Environment, args: &[ConValue]) -> IResult<ConValue> {
        match &self.model {
            Model::Tuple(_) => self.make_tuple(args.into()),
            _ => Err(Error::NotCallable(ConValue::TypeInfo(
                self.already_interned(),
            )))?,
        }
    }

    fn name(&self) -> Option<Symbol> {
        self.ident
    }
}

impl std::fmt::Display for TypeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { ident, model } = self;
        let Some(ident) = ident else {
            return model.fmt(f);
        };
        match model {
            Model::Any | Model::Unit(0) => write!(f, "{ident}"),
            Model::Unit(n) => write!(f, "{ident} = {n}"),
            Model::Integer { .. }
            | Model::Float { .. }
            | Model::Bool
            | Model::Char
            | Model::Str
            | Model::Never
            | Model::Ref(_)
            | Model::Slice(_) => write!(f, "{model}"),
            Model::Tuple(_) | Model::Struct(_, _) | Model::Enum(_) => {
                write!(f, "{ident} {model}")
            }
        }
    }
}
