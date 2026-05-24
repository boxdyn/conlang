//! Runtime [type information](Model)

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
pub type Type = Interned<'static, Model>;

pub(crate) static TYPE_INTERNER: OnceLock<LeakyInterner<Model>> = OnceLock::new();

/// The elements of a type's value
#[rustfmt::skip]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Model {
    /// A primitive integer ([u8], [i32], ...)
    Integer { signed: bool, size: usize, min: i128, max: i128 },
    /// A float ([f32], [f64])
    Float { size: usize },
    /// A [bool]
    Bool,
    /// A [char]
    Char,
    /// A [str]
    Str,
    /// Any type. A placeholder wildcard type
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
    Tuple(Option<Symbol>, Box<[Type]>),
    /// The elements of a struct, and whether they are exhaustive
    Struct(Option<Symbol>, Box<[(Symbol, Type)]>, bool),
    /// The variants of an enumeration
    Enum(Symbol, Box<[(Symbol, Type)]>),
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
            Self::Tuple(Some(name), items) => {
                f.delimit(format_args!("{name}("), ")").list(items, ", ")
            }
            Self::Tuple(_name, items) => f.delimit("(", ")").list(items, ", "),
            Self::Struct(Some(name), items, _exhaustive) => f
                .delimit(format_args!("{name}{{"), " }")
                .list(items.iter().map(|(name, ty)| format!(" {name}: {ty}")), ","),
            Self::Struct(name, items, _exhaustive) => f
                .delimit("{", " }")
                .list(items.iter().map(|(name, ty)| format!(" {name}: {ty}")), ","),
            Self::Enum(name, items) => {
                let mut f = f.delimit_indented(format_args!("enum {name}{{"), "\n}");
                for (name, idx) in items {
                    write!(f, "\n{name}: {idx},")?;
                }
                Ok(())
            }
        }
    }
}

impl Model {
    pub fn intern(self) -> Type {
        TYPE_INTERNER
            .get_or_init(LeakyInterner::new)
            .get_or_insert(self)
    }
    pub fn already_interned(&self) -> Type {
        TYPE_INTERNER
            .get_or_init(LeakyInterner::new)
            .get(self)
            .unwrap_or_else(|| panic!("Brand new type was assumed interned: {}", self))
    }
    pub fn default_integer() -> Type {
        make_int!(i128, true).already_interned()
    }
    pub fn default_float() -> Type {
        Self::Float { size: size_of::<f64>() }.already_interned()
    }

    pub fn with_name(self, name: Symbol) -> Self {
        match self {
            Self::Tuple(_, items) => Self::Tuple(Some(name), items),
            Self::Struct(_, items, e) => Self::Struct(Some(name), items, e),
            Self::Enum(_, items) => Self::Enum(name, items),
            _ => self,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Integer { signed: true, size: 1, .. } => "i8",
            Self::Integer { signed: true, size: 2, .. } => "i16",
            Self::Integer { signed: true, size: 4, .. } => "i32",
            Self::Integer { signed: true, size: 8, .. } => "i64",
            Self::Integer { signed: true, size: 16, .. } => "i128",
            Self::Integer { signed: true, .. } => "int",
            Self::Integer { signed: false, size: 1, .. } => "u8",
            Self::Integer { signed: false, size: 2, .. } => "u16",
            Self::Integer { signed: false, size: 4, .. } => "u32",
            Self::Integer { signed: false, size: 8, .. } => "u64",
            Self::Integer { signed: false, size: 16, .. } => "u128",
            Self::Integer { signed: false, .. } => "uint",
            Self::Float { size: 4 } => "f32",
            Self::Float { size: 8 } => "f64",
            Self::Bool => "bool",
            Self::Char => "char",
            Self::Str => "str",
            Self::Any => "",
            Self::Never => "!",
            Self::Unit(_) => "unit",
            Self::Ref(interned) => "&...",
            Self::Slice(interned) => "[...]",
            Self::Tuple(Some(name), ..) => name.to_ref(),
            Self::Struct(Some(name), ..) => name.to_ref(),
            Self::Enum(name, _items) => name.to_ref(),
            _ => "",
        }
    }

    pub fn make_tuple(&self, values: Box<[ConValue]>) -> IResult<ConValue> {
        match self {
            Model::Any => Ok(ConValue::TupleStruct(self.already_interned(), values)),
            Model::Tuple(_, typeids) if typeids.len() != values.len() => {
                Err(Error::ArgNumber(typeids.len(), values.len()))
            }
            Model::Tuple(_, typeids) => Ok(ConValue::TupleStruct(self.already_interned(), values)),
            _ => Err(Error::NotCallable(ConValue::TypeInfo(
                self.already_interned(),
            ))),
        }
    }

    pub fn make_struct(&self, mut values: HashMap<Symbol, ConValue>) -> IResult<ConValue> {
        let mut members = HashMap::new();
        match self {
            Model::Struct(_, model, true) => {
                for (key, _id) in model {
                    let value = values.get_mut(key).ok_or(Error::NotInitialized(*key))?;
                    members.insert(*key, value.take());
                }
            }
            Model::Struct(..) | Model::Any => members = values,
            _ => Err(Error::TypeError("struct", self.already_interned()))?,
        }

        Ok(ConValue::Struct(self.already_interned(), Box::new(members)))
    }
    #[rustfmt::skip]
    pub fn defaults() -> Vec<(&'static str, Self)> {
        let any = Model::Any.intern();
        let types = [
            ("_", Model::Any),
            ("unit", Model::Unit(0)),
            ("bool", Model::Bool),
            ("char", Model::Char),
            ("str", Model::Str),
            ("never", Model::Never),
            ("f32", Model::Float{size: size_of::<f32>()}),
            ("f64", Model::Float{size: size_of::<f64>()}),
            ("i8", make_int!(i8, true)),
            ("i16", make_int!(i16, true)),
            ("i32", make_int!(i32, true)),
            ("i64", make_int!(i64, true)),
            ("i128", make_int!(i128, true)),
            ("isize", make_int!(isize, true)),
            ("int", make_int!(isize, true)),
            ("u8", make_int!(u8, false)),
            ("u16", make_int!(u16, false)),
            ("u32", make_int!(u32, false)),
            ("u64", make_int!(u64, false)),
            ("u128", make_int!(u128, false)),
            ("usize", make_int!(usize, false)),
            ("uint", make_int!(usize, false)),
            ("RangeExc", Model::Tuple(Some("RangeExc".into()), [any, any].into())),
            ("RangeInc", Model::Tuple(Some("RangeInc".into()), [any, any].into())),
            ("RangeTo", Model::Tuple(Some("RangeTo".into()), [any].into())),
            ("RangeToInc", Model::Tuple(Some("RangeToInc".into()), [any].into())),
        ];
        types.into()
    }

    pub fn getattr(&self, attr: Symbol) -> IResult<ConValue> {
        Ok(match (self, attr.0) {
            (_, "Self") => ConValue::TypeInfo(self.already_interned()),
            (&Model::Integer { signed, .. }, "SIGNED") => ConValue::Bool(signed),
            (&Model::Integer { size, .. }, "SIZE") => ConValue::Int(size as _),
            (&Model::Integer { size, .. }, "BITS") => ConValue::Int(8 * size as i128),
            (&Model::Integer { min, .. }, "MIN") => ConValue::Int(min),
            (&Model::Integer { max, .. }, "MAX") => ConValue::Int(max),
            (&Model::Float { size }, "SIZE") => ConValue::Int(size as _),
            (&Model::Float { .. }, "INF") => ConValue::Float(f64::INFINITY),
            (&Model::Float { .. }, "NAN" | "NaN") => ConValue::Float(f64::NAN),
            (Model::Bool, "SIZE") => ConValue::Int(size_of::<bool>() as _),
            (Model::Char, "SIZE") => ConValue::Int(size_of::<char>() as _),
            (Model::Never, _) => Err(Error::NotDefined(attr))?,
            (Model::Unit(_), "SIZE") => ConValue::Int(0),
            (Model::Unit(_), _) => Err(Error::NotDefined(attr))?,
            (Model::Tuple(_, items), "ARITY") => ConValue::Int(items.len() as _),
            (Model::Struct(_, items, _), "NAMES") => {
                ConValue::Array(items.iter().map(|(n, _)| ConValue::Str(*n)).collect())
            }
            (Model::Struct(_, items, _), "TYPES") => {
                ConValue::Array(items.iter().map(|(_, t)| ConValue::TypeInfo(*t)).collect())
            }
            (Model::Struct(_, items, _), "MEMBERS") => ConValue::Array(
                items
                    .iter()
                    .map(|(n, t)| {
                        ConValue::Tuple([ConValue::Str(*n), ConValue::TypeInfo(*t)].into())
                    })
                    .collect(),
            ),
            (Model::Struct(_, items, exhaustive), _) => items
                .iter()
                .find_map(|&(name, ty)| (name == attr).then_some(ConValue::TypeInfo(ty)))
                .ok_or(Error::NotDefined(attr))?,
            (Model::Enum(_, items), _) => items
                .iter()
                .find_map(|&(name, ty)| (name == attr).then_some(ConValue::TypeInfo(ty)))
                .ok_or(Error::NotDefined(attr))?,
            (model, _) => Err(Error::NotDefined(attr))?,
        })
    }
}

macro make_int($T:ty, $signed: expr) {
    Model::Integer {
        signed: $signed,
        size: size_of::<$T>(),
        min: <$T>::MIN as _,
        max: <$T>::MAX as _,
    }
}

impl Callable for Model {
    fn call(&self, env: &mut Environment, args: &[ConValue]) -> IResult<ConValue> {
        self.make_tuple(args.into())
    }

    fn name(&self) -> Option<Symbol> {
        Some(format!("{self}").as_str().into())
    }
}
