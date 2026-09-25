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
    Unit(Option<Symbol>, Option<Type>, usize),
    /// The elements of a tuple
    Tuple(Option<Symbol>, Option<Type>, Box<[Type]>),
    /// The elements of a struct, and whether they are exhaustive
    Struct(Option<Symbol>, Option<Type>, Box<[(Symbol, Type)]>, bool),
    /// The variants of an enumeration
    Enum(Symbol, Box<[(Symbol, Type)]>),
    /// Reference to a value of [Type]
    Ref(Type),
    /// Slice of a list of [Type]
    Slice(Type),
    /// An arbitrary function (TODO: encode signature)
    Function,
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
            Self::Unit(Some(name), _, _) => name.fmt(f),
            Self::Unit(None, _, _) => "()".fmt(f),
            Self::Tuple(Some(name), _, items) => {
                f.delimit(format_args!("{name}("), ")").list(items, ", ")
            }
            Self::Tuple(_name, _, items) => f.delimit("(", ")").list(items, ", "),
            Self::Struct(name, _, i, ex) if i.is_empty() => {
                let name = name.map(Interned::to_ref).unwrap_or_default();
                let ex = if *ex { "" } else { " .. " };
                write!(f, "{name} {{{ex}}}",)
            }
            Self::Struct(Some(name), _, items, ex) => f
                .delimit(format_args!("{name} {{"), if *ex { " }" } else { ", .. }" })
                .list(items.iter().map(|(name, ty)| format!(" {name}: {ty}")), ","),
            Self::Struct(None, _, items, exhaust) => f
                .delimit("{", if *exhaust { " }" } else { ", .. }" })
                .list(items.iter().map(|(name, ty)| format!(" {name}: {ty}")), ","),
            Self::Enum(name, items) => {
                let mut f = f.delimit_indented(format_args!("enum {name} {{"), "\n}");
                for (name, ty) in items {
                    if (name.to_ref() == ty.name()) {
                        write!(f, "\n{ty},")?;
                    } else {
                        write!(f, "\n{name}: {ty},")?;
                    }
                }
                Ok(())
            }
            Self::Ref(t) => write!(f, "&{t}"),
            Self::Slice(t) => write!(f, "[{t}]"),
            Self::Function => {
                write!(f, "fn()")
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
            Self::Tuple(_, parent, items) => Self::Tuple(Some(name), parent, items),
            Self::Struct(_, parent, items, e) => Self::Struct(Some(name), parent, items, e),
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
            Self::Unit(Some(name), ..) => name.to_ref(),
            Self::Unit(None, ..) => "unit",
            Self::Tuple(Some(name), ..) => name.to_ref(),
            Self::Struct(Some(name), ..) => name.to_ref(),
            Self::Enum(name, _items) => name.to_ref(),
            Self::Ref(interned) => "&...",
            Self::Slice(interned) => "[...]",
            Self::Function => "",
            _ => "",
        }
    }

    pub fn parent(&self) -> Option<Type> {
        match self {
            Model::Integer { .. } => {
                let default_integer = Self::default_integer();
                (default_integer.to_ref() != self).then_some(default_integer)
            }
            Model::Float { .. } => {
                let default_float = Self::default_float();
                (default_float.to_ref() != self).then_some(default_float)
            }
            Model::Bool => None,
            Model::Char => None,
            Model::Str => None,
            Model::Any => None,
            Model::Never => None,
            Model::Unit(_, Some(parent), _) => Some(*parent),
            Model::Unit(_, None, _) => None,
            Model::Tuple(_, Some(parent), _) => Some(*parent),
            Model::Tuple(_, None, _) => None,
            Model::Struct(_, Some(parent), _, _) => Some(*parent),
            Model::Struct(_, None, _, _) => None,
            Model::Enum(_, items) => None,
            Model::Ref(Interned(Model::Any, ..)) => None,
            Model::Ref(_) => Some(Model::Ref(Model::Any.already_interned()).intern()),
            Model::Slice(Interned(Model::Any, ..)) => None,
            Model::Slice(_) => Some(Model::Slice(Model::Any.already_interned()).intern()),
            Model::Function => None,
        }
    }

    pub fn make_tuple(&self, values: Box<[ConValue]>) -> IResult<ConValue> {
        match self {
            Model::Any => Ok(ConValue::TupleStruct(self.already_interned(), values)),
            Model::Tuple(_, _, typeids) if typeids.len() != values.len() => {
                Err(Error::ArgNumber(typeids.len(), values.len()))
            }
            Model::Tuple(_, _, typeids) => {
                Ok(ConValue::TupleStruct(self.already_interned(), values))
            }
            _ => Err(Error::NotCallable(ConValue::TypeInfo(
                self.already_interned(),
            ))),
        }
    }

    pub fn make_struct(&self, mut values: HashMap<Symbol, ConValue>) -> IResult<ConValue> {
        let mut members = HashMap::new();
        match self {
            Model::Struct(_, _, model, true) => {
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
            ("fn", Model::Function),
            ("unit", Model::Unit(None, None, 0)),
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
            ("RangeExc", Model::Tuple(Some("RangeExc".into()), None, [any, any].into())),
            ("RangeInc", Model::Tuple(Some("RangeInc".into()), None, [any, any].into())),
            ("RangeTo", Model::Tuple(Some("RangeTo".into()), None, [any].into())),
            ("RangeToInc", Model::Tuple(Some("RangeToInc".into()), None, [any].into())),
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
            (Model::Bool, "SIZE") => ConValue::Int(size_of::<bool>() as _),
            (Model::Char, "SIZE") => ConValue::Int(size_of::<char>() as _),
            (Model::Never, _) => Err(Error::NotDefined(attr))?,
            (Model::Unit(_, _, _), "SIZE") => ConValue::Int(0),
            (Model::Unit(_, _, _), _) => Err(Error::NotDefined(attr))?,
            (Model::Tuple(_, _, items), "ARITY") => ConValue::Int(items.len() as _),
            (Model::Tuple(_, _, items), "TYPES") => ConValue::Tuple(
                Vec::from_iter(items.iter().copied().map(ConValue::TypeInfo)).into_boxed_slice(),
            ),
            (Model::Struct(_, _, items, _), "NAMES") => {
                ConValue::Array(items.iter().map(|(n, _)| ConValue::Str(*n)).collect())
            }
            (Model::Struct(_, _, items, _), "TYPES") => {
                ConValue::Array(items.iter().map(|(_, t)| ConValue::TypeInfo(*t)).collect())
            }
            (Model::Struct(_, _, items, _), "MEMBERS") => ConValue::Array(
                items
                    .iter()
                    .map(|(n, t)| {
                        ConValue::Tuple([ConValue::Str(*n), ConValue::TypeInfo(*t)].into())
                    })
                    .collect(),
            ),
            (Model::Struct(_, _, items, exhaustive), _) => items
                .iter()
                .find_map(|&(name, ty)| (name == attr).then_some(ConValue::TypeInfo(ty)))
                .ok_or(Error::NotDefined(attr))?,
            (Model::Enum(_, items), "VARIANTS") => {
                ConValue::Array(items.iter().map(|v| v.1).map(ConValue::TypeInfo).collect())
            }
            (Model::Enum(_, items), "COUNT") => ConValue::Int(items.len() as _),
            (Model::Enum(_, items), _) => items
                .iter()
                .find_map(|&(name, ty)| (name == attr).then_some(ConValue::TypeInfo(ty)))
                .ok_or(Error::NotDefined(attr))?,
            (model, "super") if let Some(ty) = model.parent() => ConValue::TypeInfo(ty),
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
    fn call(&self, env: &mut Environment, args: Vec<ConValue>) -> IResult<ConValue> {
        self.make_tuple(args.into())
    }

    fn name(&self) -> Option<Symbol> {
        Some(format!("{self}").as_str().into())
    }
}
