//! [Conlang Values](ConValue) in the dynamically typed AST interpreter.
//!
//! > The most permanent fix is a temporary one.
use cl_ast::{At, Expr, fmt::FmtAdapter, types::Symbol};
use cl_structures::intern::interned::Interned;

use crate::{
    place::Place,
    typeinfo::{Model, Type},
};

use super::{
    Callable, Environment,
    builtin::Builtin,
    error::{Error, IResult},
    function::Function,
};
use std::{collections::HashMap, ops::*, rc::Rc};

/*
A Value can be:
- A Primitive (Empty, isize, etc.)
- A Record (Array, Tuple, Struct)
- A Variant (discriminant, Value) pair

array [
    10,     // 0
    20,     // 1
]

tuple (
    10,     // 0
    20,     // 1
)

struct {
    x: 10,  // x => 0
    y: 20,  // y => 1
}
*/

type Integer = i128;

/// A Conlang value stores data in the interpreter
#[derive(Clone, Debug, Default)]
pub enum ConValue {
    /// The empty/unit `()` type
    #[default]
    Empty,
    /// An integer
    Int(Integer),
    /// A floating point number
    Float(f64),
    /// A boolean
    Bool(bool),
    /// A unicode character
    Char(char),
    /// A string literal
    Str(Symbol),
    /// A dynamic string
    String(String),
    /// A reference
    Ref(Place),
    /// A reference to an array
    Slice(Place, usize, usize),
    /// An Array
    Array(Box<[ConValue]>),
    /// A tuple
    Tuple(Box<[ConValue]>),
    // TODO: Instead of storing the identifier, store the index of the struct module
    /// A value of a product type
    Struct(Type, Box<HashMap<Symbol, ConValue>>),
    /// A value of a product type with anonymous members
    TupleStruct(Type, Box<[ConValue]>),
    /// An entire namespace
    Module(Box<HashMap<Symbol, ConValue>>),
    /// A quoted expression
    Quote(Box<At<Expr>>),
    /// A callable thing
    Function(Rc<Function>),
    /// A built-in function
    Builtin(&'static Builtin),
    /// The definition of a type, by index
    TypeInfo(Type),
}

impl ConValue {
    /// Gets whether the current value is true or false
    pub fn truthy(&self) -> IResult<bool> {
        match self {
            ConValue::Bool(v) => Ok(*v),
            ConValue::Int(v) => Ok(*v != 0),
            _ => Err(Error::TypeError("type implements Truth", self.type_of()))?,
        }
    }

    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }

    pub fn is_cheap_to_copy(&self) -> bool {
        match self {
            Self::Empty
            | Self::Int(_)
            | Self::Float(_)
            | Self::Bool(_)
            | Self::Char(_)
            | Self::Str(_) => true,
            Self::Ref(_) => true,
            Self::Slice(_, _, _) => true,
            Self::Quote(_) => true,
            Self::Function(_) => true,
            Self::Builtin(_) => true,
            Self::TypeInfo(_) => true,
            Self::Module(_)
            | Self::String(_)
            | Self::Array(_)
            | Self::Tuple(_)
            | Self::Struct(_, _)
            | Self::TupleStruct(_, _) => false,
        }
    }

    pub fn type_of(&self) -> Type {
        match self {
            Self::Empty => Model::Unit(None, 0).already_interned(),
            Self::Int(_) => Model::default_integer(),
            Self::Float(_) => Model::default_float(),
            Self::Bool(_) => Model::Bool.already_interned(),
            Self::Char(_) => Model::Char.already_interned(),
            Self::Str(_) => Model::Str.already_interned(),
            Self::String(_) => Model::Str.already_interned(),
            Self::Ref(place) => Model::Ref(Model::Any.already_interned()).intern(),
            Self::Slice(place, _, _) => Model::Slice(Model::Any.already_interned()).intern(),
            Self::Array(arr) if !arr.is_empty() => Model::Slice(arr[0].type_of()).intern(),
            Self::Array(_) => Model::Slice(Model::Any.already_interned()).intern(),
            Self::Tuple(vs) => Model::Tuple(None, vs.iter().map(Self::type_of).collect()).intern(),
            Self::Struct(ty, _) => *ty,
            Self::TupleStruct(ty, _) => *ty,
            Self::Module(_) => todo!("type_of({self})"),
            Self::Quote(_) => todo!("type_of({self})"),
            Self::Function(_) => todo!("type_of({self})"),
            Self::Builtin(_) => todo!("type_of({self})"),
            Self::TypeInfo(ty) => *ty,
        }
    }

    pub fn cast(self, ty: &Model) -> Self {
        match ty {
            &Model::Integer { signed, size, min, max } => {
                let i = match self {
                    Self::Int(v) => v,
                    Self::Float(v) => v as _,
                    Self::Bool(v) => v as _,
                    Self::Char(v) => v as _,
                    Self::TypeInfo(Interned(Model::Unit(_, d), ..)) => *d as _,
                    _ => return self,
                };
                if i == min || i == max {
                    ConValue::Int(i)
                } else if signed {
                    ConValue::Int(i.wrapping_rem(max))
                } else {
                    ConValue::Int(i & max)
                }
            }
            Model::Float { size } => {
                let f = match self {
                    Self::Float(v) => v,
                    Self::Int(v) => v as _,
                    Self::Bool(v) => v as i32 as _,
                    Self::Char(v) => v as i32 as _,
                    Self::TypeInfo(Interned(Model::Unit(_, d), ..)) => *d as _,
                    _ => return self,
                };
                ConValue::Float(f)
            }
            Model::Bool => ConValue::Bool(self.truthy().unwrap_or(true)),
            Model::Char => {
                let c = match self {
                    Self::Int(v) => v as _,
                    Self::Float(v) => v as _,
                    Self::Bool(v) => v as _,
                    Self::Char(v) => return self,
                    Self::TypeInfo(Interned(Model::Unit(_, d), ..)) => *d as _,
                    _ => return self,
                };
                ConValue::Char(char::from_u32(c).unwrap_or('�'))
            }
            Model::Unit(None, _) => ConValue::Empty,
            Model::Unit(_, _) => ConValue::TypeInfo(ty.already_interned()),
            Model::Str => ConValue::String(self.to_string()),
            _ => self,
        }
    }

    pub fn dereference_in<'e>(&'e self, env: &'e Environment) -> IResult<&'e Self> {
        let mut value = self;
        while let ConValue::Ref(r) = value {
            value = r.get(env)?;
        }
        Ok(value)
    }

    #[allow(non_snake_case)]
    pub fn tuple_struct(id: Type, values: impl Into<Box<[ConValue]>>) -> Self {
        Self::TupleStruct(id, values.into())
    }
    #[allow(non_snake_case)]
    pub fn Struct(id: Type, values: HashMap<Symbol, ConValue>) -> Self {
        Self::Struct(id, Box::new(values))
    }

    pub fn index(self, index: &Self, _env: &Environment) -> IResult<ConValue> {
        let &Self::Int(index) = index else {
            Err(Error::TypeError("int", index.type_of()))?
        };
        match self {
            ConValue::Str(string) => string
                .chars()
                .nth(index as _)
                .map(ConValue::Char)
                .ok_or(Error::OobIndex(index as usize, string.chars().count())),
            ConValue::String(string) => string
                .chars()
                .nth(index as _)
                .map(ConValue::Char)
                .ok_or(Error::OobIndex(index as usize, string.chars().count())),
            ConValue::Array(arr) => arr
                .get(index as usize)
                .cloned()
                .ok_or(Error::OobIndex(index as usize, arr.len())),
            ConValue::Slice(place, start, len) => {
                if (index.unsigned_abs() as usize) < len {
                    Ok(ConValue::Ref(place.index(index as _, index < 0)))
                } else {
                    Err(Error::OobIndex(index.unsigned_abs() as _, len))
                }
            }
            ConValue::Ref(place) => Ok(ConValue::Ref(
                place.index(index.unsigned_abs() as _, index < 0),
            )),
            other => Err(Error::TypeError("type implements Index", other.type_of())),
        }
    }
    cmp! {
        lt: <;
        lt_eq: <=;
        eq: ==;
        neq: !=;
        gt_eq: >=;
        gt: >;
    }
    assign! {
        add_assign: +;
        bitand_assign: &;
        bitor_assign: |;
        bitxor_assign: ^;
        div_assign: /;
        mul_assign: *;
        rem_assign: %;
        shl_assign: <<;
        shr_assign: >>;
        sub_assign: -;
    }
}

impl Callable for ConValue {
    fn name(&self) -> Option<Symbol> {
        match self {
            ConValue::Function(func) => func.name(),
            // ConValue::Closure(func) => func.name(),
            ConValue::Builtin(func) => func.name(),
            _ => None,
        }
    }
    fn call(&self, env: &mut Environment, args: &[ConValue]) -> IResult<ConValue> {
        match self {
            Self::Function(func) => func.call(env, args),
            Self::Builtin(func) => func.call(env, args),
            Self::Module(m) => {
                if let Some(func) = m.get(&"call".into()) {
                    func.call(env, args)
                } else {
                    Err(Error::NotCallable(self.clone()))
                }
            }
            Self::Ref(ptr) => {
                // Move onto stack, and call
                let func = ptr.get(env)?.clone();
                func.call(env, args)
            }
            Self::TypeInfo(idx) => idx.call(env, args),
            _ => Err(Error::NotCallable(self.clone())),
        }
    }
}

/// Templates comparison functions for [ConValue]
macro cmp ($($fn:ident: $op:tt);*$(;)?) {$(
    /// TODO: Remove when functions are implemented:
    ///       Desugar into function calls
    pub fn $fn(&self, other: &Self) -> IResult<Self> {
        Ok(ConValue::Bool(self.compare(other)? $op 0))
    }
)*}

impl ConValue {
    pub fn compare(&self, other: &Self) -> IResult<isize> {
        use std::cmp::Ord;
        Ok(match (self, other) {
            (Self::Empty, Self::Empty) => 0,
            (Self::Int(a), Self::Int(b)) => a.cmp(b) as _,
            (Self::Float(a), Self::Float(b)) => {
                a.partial_cmp(b).unwrap_or_else(|| a.total_cmp(b)) as _
            }
            (Self::Bool(a), Self::Bool(b)) => a.cmp(b) as _,
            (Self::Char(a), Self::Char(b)) => a.cmp(b) as _,
            (Self::Str(a), Self::Str(b)) => a.cmp(b) as _,
            (Self::Str(a), Self::String(b)) => a.to_ref().cmp(b) as _,
            (Self::String(a), Self::Str(b)) => a.deref().cmp(b.to_ref()) as _,
            (Self::String(a), Self::String(b)) => a.cmp(b) as _,
            (Self::Array(a), Self::Array(b)) | (Self::Tuple(a), Self::Tuple(b)) => {
                if a.len() != b.len() {
                    return Ok(a.len().cmp(&b.len()) as _);
                };
                let mut res = 0;
                for (a, b) in a.iter().zip(b.iter()) {
                    res = a.compare(b)?;
                    if res != 0 {
                        break;
                    }
                }
                res
            }
            (Self::TypeInfo(a), Self::TypeInfo(b)) => a.cmp(b) as _,
            (a, b) => Err(Error::TypeError("Cmp", a.type_of()))?,
        })
    }
}

macro assign($( $fn: ident: $op: tt );*$(;)?) {$(
    pub fn $fn(&mut self, other: Self) -> IResult<()> {
        *self = (std::mem::take(self) $op other)?;
        Ok(())
    }
)*}
/// Implements [From] for an enum with 1-tuple variants
macro from ($($T:ty => $v:expr),*$(,)?) {
    $(impl From<$T> for ConValue {
        fn from(value: $T) -> Self { $v(value.into()) }
    })*
}
impl From<&Symbol> for ConValue {
    fn from(value: &Symbol) -> Self {
        ConValue::Str(*value)
    }
}
impl From<Rc<Symbol>> for ConValue {
    fn from(value: Rc<Symbol>) -> Self {
        ConValue::Str(value.0.into())
    }
}
from! {
    Integer => ConValue::Int,
    f64 => ConValue::Float,
    bool => ConValue::Bool,
    char => ConValue::Char,
    Symbol => ConValue::Str,
    &str => ConValue::Str,
    At<Expr> => ConValue::Quote,
    String => ConValue::String,
    Function => ConValue::Function,
    Vec<ConValue> => ConValue::Tuple,
    &'static Builtin => ConValue::Builtin,
}
impl From<()> for ConValue {
    fn from(_: ()) -> Self {
        Self::Empty
    }
}
impl From<&[ConValue]> for ConValue {
    fn from(value: &[ConValue]) -> Self {
        match value {
            [] => Self::Empty,
            [value] => value.clone(),
            _ => Self::Tuple(value.into()),
        }
    }
}

/// Implements binary [std::ops] traits for [ConValue]
///
/// TODO: Desugar operators into function calls
macro ops($($trait:ty: $fn:ident = [$($match:tt)*])*) {
    $(impl $trait for ConValue {
        type Output = IResult<Self>;
        /// TODO: Desugar operators into function calls
        fn $fn(self, rhs: Self) -> Self::Output {Ok(match (self, rhs) {$($match)*})}
    })*
}
ops! {
    Add: add = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_add(b)),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a + b),
        (ConValue::Str(a), ConValue::Str(b)) => (a.to_string() + &*b).into(),
        (ConValue::Str(a), ConValue::String(b)) => (a.to_string() + &*b).into(),
        (ConValue::String(a), ConValue::Str(b)) => (a + &*b).into(),
        (ConValue::String(a), ConValue::String(b)) => (a + &*b).into(),
        (ConValue::Str(s), ConValue::Char(c)) => { let mut s = s.to_string(); s.push(c); s.into() }
        (ConValue::String(s), ConValue::Char(c)) => { let mut s = s.to_string(); s.push(c); s.into() }
        (ConValue::Char(a), ConValue::Char(b)) => {
            ConValue::String([a, b].into_iter().collect::<String>())
        }
        (a, b) => Err(Error::TypeError(a.type_of(), b.type_of()))?
    ]
    BitAnd: bitand = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a & b),
        (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a & b),
        (a, b) => Err(Error::TypeError(a.type_of(), b.type_of()))?
    ]
    BitOr: bitor = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a | b),
        (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a | b),
        (a, b) => Err(Error::TypeError(a.type_of(), b.type_of()))?
    ]
    BitXor: bitxor = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a ^ b),
        (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a ^ b),
        (a, b) => Err(Error::TypeError(a.type_of(), b.type_of()))?
    ]
    Div: div = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.checked_div(b).unwrap_or_else(|| {
            eprintln!("Warning: Divide by zero in {a} / {b}"); a
        })),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a / b),
        (a, b) => Err(Error::TypeError(a.type_of(), b.type_of()))?
    ]
    Mul: mul = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_mul(b)),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a * b),
        (a, b) => Err(Error::TypeError(a.type_of(), b.type_of()))?
    ]
    Rem: rem = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.checked_rem(b).unwrap_or_else(|| {
            println!("Warning: Divide by zero in {a} % {b}"); a
        })),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a % b),
        (a, b) => Err(Error::TypeError(a.type_of(), b.type_of()))?
    ]
    Shl: shl = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_shl(b as _)),
        (a, ConValue::Int(_)) => Err(Error::TypeError("type implements Shl", a.type_of()))?,
        (_, b) => Err(Error::TypeError("int", b.type_of()))?
    ]
    Shr: shr = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_shr(b as _)),
        (a, ConValue::Int(_)) => Err(Error::TypeError("type implements Shr", a.type_of()))?,
        (_, b) => Err(Error::TypeError("int", b.type_of()))?
    ]
    Sub: sub = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_sub(b)),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a - b),
        (a, b) => Err(Error::TypeError(a.type_of(), b.type_of()))?
    ]
}
impl std::fmt::Display for ConValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConValue::Empty => "Empty".fmt(f),
            ConValue::Int(v) => v.fmt(f),
            ConValue::Float(v) => v.fmt(f),
            ConValue::Bool(v) => v.fmt(f),
            ConValue::Char(v) => v.fmt(f),
            ConValue::Str(v) => v.fmt(f),
            ConValue::String(v) => v.fmt(f),
            ConValue::Ref(v) => write!(f, "&<{}>", v),
            ConValue::Slice(id, start, len) => write!(f, "&<{id}>[{start}..{len}]"),
            ConValue::Array(array) => f.delimit('[', ']').list(array, ", "),
            ConValue::Tuple(tuple) => f.delimit('(', ')').list(tuple, ", "),
            ConValue::TupleStruct(id, tuple) => f
                .delimit(format_args!("{}(", id.name()), ")")
                .list(tuple, ", "),
            ConValue::Struct(id, map) => {
                use std::fmt::Write;
                write!(f, "{} ", id.name())?;
                let mut f = f.delimit_indented("{", "\n}");
                for (k, v) in map.iter() {
                    write!(f, "\n{k}: {v},")?;
                }
                Ok(())
            }
            ConValue::Module(module) => {
                use std::fmt::Write;
                let mut f = f.delimit("{", "\n}");
                for (k, v) in module.iter() {
                    write!(f, "\n{k}: {v},")?;
                }
                Ok(())
            }
            ConValue::Quote(q) => write!(f, "`{q}`"),
            ConValue::Function(func) => func.fmt(f),
            ConValue::Builtin(func) => func.fmt(f),
            ConValue::TypeInfo(ty) => ty.fmt(f),
        }
    }
}

pub macro cvstruct (
    $Name:ident {
        $($member:ident : $expr:expr),*
    }
) {{
    let mut members = HashMap::new();
    $(members.insert(stringify!($member).into(), ($expr).into());)*
    ConValue::Struct(Box::new((stringify!($Name).into(), members)))
}}
