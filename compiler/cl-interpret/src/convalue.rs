//! Values in the dynamically typed AST interpreter.
//!
//! The most permanent fix is a temporary one.
use cl_ast::{Expr, fmt::FmtAdapter, types::Symbol};

use crate::{constructor::Constructor, place::Place};

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

type Integer = isize;

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
    Slice(Place, usize),
    /// An Array
    Array(Box<[ConValue]>),
    /// A tuple
    Tuple(Box<[ConValue]>),
    // TODO: Instead of storing the identifier, store the index of the struct module
    /// A value of a product type
    Struct(Symbol, Box<HashMap<Symbol, ConValue>>),
    /// A value of a product type with anonymous members
    TupleStruct(Symbol, Box<[ConValue]>),
    /// An entire namespace
    Module(Box<HashMap<Symbol, ConValue>>),
    /// A quoted expression
    Quote(Rc<Expr>),
    /// A callable thing
    Function(Rc<Function>),
    /// A tuple constructor
    TupleConstructor(Constructor),
    // /// A closure, capturing by reference
    // Closure(Rc<Closure>),
    /// A built-in function
    Builtin(&'static Builtin),
}

impl ConValue {
    /// Gets whether the current value is true or false
    pub fn truthy(&self) -> IResult<bool> {
        match self {
            ConValue::Bool(v) => Ok(*v),
            ConValue::Int(v) => Ok(*v != 0),
            _ => Err(Error::TypeError("type implements Truth", self.typename()))?,
        }
    }

    pub fn typename(&self) -> &'static str {
        match self {
            ConValue::Empty => "Empty",
            ConValue::Int(_) => "i64",
            ConValue::Float(_) => "f64",
            ConValue::Bool(_) => "bool",
            ConValue::Char(_) => "char",
            ConValue::Str(_) => "str",
            ConValue::String(_) => "String",
            ConValue::Ref(_) => "Ref",
            ConValue::Slice(_, _) => "Slice",
            ConValue::Array(_) => "Array",
            ConValue::Tuple(_) => "Tuple",
            ConValue::Struct(sym, _) => sym.to_ref(),
            ConValue::TupleStruct(sym, _) => sym.to_ref(),
            ConValue::Module(_) => "",
            ConValue::Quote(_) => "Quote",
            ConValue::Function(_) => "Fn",
            ConValue::TupleConstructor(_) => "Fn",
            // ConValue::Closure(_) => "Fn",
            ConValue::Builtin(_) => "Fn",
        }
    }

    pub fn cast(self, to_type: &str) -> Self {
        if to_type.starts_with("f") {
            let f = match self {
                Self::Float(v) => v,
                Self::Int(v) => v as _,
                Self::Bool(v) => v as i32 as _,
                Self::Char(v) => v as i32 as _,
                _ => return self,
            };
            return ConValue::Float(f);
        }
        let i = match self {
            Self::Int(v) => v,
            Self::Float(v) => v as _,
            Self::Bool(v) => v as _,
            Self::Char(v) => v as _,
            _ => return self,
        };
        match to_type {
            "i8" => ConValue::Int(i as i8 as _),
            "i16" => ConValue::Int(i as i16 as _),
            "i32" => ConValue::Int(i as i32 as _),
            "i64" => ConValue::Int(i),
            "isize" | "int" => ConValue::Int(i),
            "u8" => ConValue::Int(i % 0x100),
            "u16" => ConValue::Int(i % 0x10000),
            "u32" => ConValue::Int(i % 0x100000000),
            "u64" => ConValue::Int(i),
            "usize" | "uint" => ConValue::Int(i),
            "bool" => ConValue::Bool(i != 0),
            "char" => ConValue::Char(char::from_u32(i as _).unwrap_or('�')),
            _ => self,
        }
    }

    #[allow(non_snake_case)]
    pub fn tuple_struct(id: impl Into<Symbol>, values: impl Into<Box<[ConValue]>>) -> Self {
        Self::TupleStruct(id.into(), values.into())
    }
    #[allow(non_snake_case)]
    pub fn Struct(id: impl Into<Symbol>, values: HashMap<Symbol, ConValue>) -> Self {
        Self::Struct(id.into(), Box::new(values))
    }

    pub fn index(&self, index: &Self, _env: &Environment) -> IResult<ConValue> {
        let &Self::Int(index) = index else {
            Err(Error::TypeError("int", index.typename()))?
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
            ConValue::Slice(place, len) => {
                let index = if index < 0 {
                    len.wrapping_add_signed(index)
                } else {
                    index as usize
                };

                if index < *len {
                    Ok(ConValue::Ref(place.clone().index(index)))
                } else {
                    Err(Error::OobIndex(index, *len))
                }
            }
            other => Err(Error::TypeError("type implements Index", other.typename())),
        }
    }
    cmp! {
        lt: false, <;
        lt_eq: true, <=;
        eq: true, ==;
        neq: false, !=;
        gt_eq: true, >=;
        gt: false, >;
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
            Self::TupleConstructor(func) => func.call(env, args),
            // Self::Closure(func) => func.call(env, args),
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
                let func = ptr.get_mut(env)?.clone();
                func.call(env, args)
            }
            _ => Err(Error::NotCallable(self.clone())),
        }
    }
}
/// Templates comparison functions for [ConValue]
macro cmp ($($fn:ident: $empty:literal, $op:tt);*$(;)?) {$(
    /// TODO: Remove when functions are implemented:
    ///       Desugar into function calls
    pub fn $fn(&self, other: &Self) -> IResult<Self> {
        match (self, other) {
            (Self::Empty, Self::Empty) => Ok(Self::Bool($empty)),
            (Self::Int(a), Self::Int(b)) => Ok(Self::Bool(a $op b)),
            (Self::Float(a), Self::Float(b)) => Ok(Self::Bool(a $op b)),
            (Self::Bool(a), Self::Bool(b)) => Ok(Self::Bool(a $op b)),
            (Self::Char(a), Self::Char(b)) => Ok(Self::Bool(a $op b)),
            (Self::Str(a), Self::Str(b)) => Ok(Self::Bool(&**a $op &**b)),
            (Self::Str(a), Self::String(b)) => Ok(Self::Bool(&**a $op &**b)),
            (Self::String(a), Self::Str(b)) => Ok(Self::Bool(&**a $op &**b)),
            (Self::String(a), Self::String(b)) => Ok(Self::Bool(&**a $op &**b)),
            (a, _) => Err(Error::TypeError("type implements Cmp", a.typename()))?,
        }
    }
)*}
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
    Expr => ConValue::Quote,
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
        (ConValue::String(a), ConValue::Str(b)) => (a.to_string() + &*b).into(),
        (ConValue::String(a), ConValue::String(b)) => (a.to_string() + &*b).into(),
        (ConValue::Str(s), ConValue::Char(c)) => { let mut s = s.to_string(); s.push(c); s.into() }
        (ConValue::String(s), ConValue::Char(c)) => { let mut s = s.to_string(); s.push(c); s.into() }
        (ConValue::Char(a), ConValue::Char(b)) => {
            ConValue::String([a, b].into_iter().collect::<String>())
        }
        (a, _) => Err(Error::TypeError("type implements add", a.typename()))?
    ]
    BitAnd: bitand = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a & b),
        (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a & b),
        (a, _) => Err(Error::TypeError("int or bool",a.typename()))?
    ]
    BitOr: bitor = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a | b),
        (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a | b),
        (a, _) => Err(Error::TypeError("int or bool", a.typename()))?
    ]
    BitXor: bitxor = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a ^ b),
        (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a ^ b),
        (a, _) => Err(Error::TypeError("int or bool", a.typename()))?
    ]
    Div: div = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.checked_div(b).unwrap_or_else(|| {
            eprintln!("Warning: Divide by zero in {a} / {b}"); a
        })),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a / b),
        (a, _) => Err(Error::TypeError("type implements Div", a.typename()))?
    ]
    Mul: mul = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_mul(b)),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a * b),
        (a, _) => Err(Error::TypeError("type implements Mul", a.typename()))?
    ]
    Rem: rem = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.checked_rem(b).unwrap_or_else(|| {
            println!("Warning: Divide by zero in {a} % {b}"); a
        })),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a % b),
        (a, _) => Err(Error::TypeError("type implements Rem", a.typename()))?
    ]
    Shl: shl = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_shl(b as _)),
        (a, ConValue::Int(_)) => Err(Error::TypeError("type implements Shl", a.typename()))?,
        (_, b) => Err(Error::TypeError("int", b.typename()))?
    ]
    Shr: shr = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_shr(b as _)),
        (a, ConValue::Int(_)) => Err(Error::TypeError("type implements Shr", a.typename()))?,
        (_, b) => Err(Error::TypeError("int", b.typename()))?
    ]
    Sub: sub = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_sub(b)),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a - b),
        (a, _) => Err(Error::TypeError("type implements Sub", a.typename()))?
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
            ConValue::Slice(id, len) => write!(f, "&<{id}>[{len}..]"),
            ConValue::Array(array) => {
                '['.fmt(f)?;
                for (idx, element) in array.iter().enumerate() {
                    if idx > 0 {
                        ", ".fmt(f)?
                    }
                    element.fmt(f)?
                }
                ']'.fmt(f)
            }
            ConValue::Tuple(tuple) => {
                '('.fmt(f)?;
                for (idx, element) in tuple.iter().enumerate() {
                    if idx > 0 {
                        ", ".fmt(f)?
                    }
                    element.fmt(f)?
                }
                ')'.fmt(f)
            }
            ConValue::TupleStruct(id, tuple) => {
                write!(f, "{id}")?;
                '('.fmt(f)?;
                for (idx, element) in tuple.iter().enumerate() {
                    if idx > 0 {
                        ", ".fmt(f)?
                    }
                    element.fmt(f)?
                }
                ')'.fmt(f)
            }
            ConValue::Struct(id, map) => {
                use std::fmt::Write;
                write!(f, "{id} ")?;
                let mut f = f.delimit("{", "\n}");
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
            ConValue::Quote(q) => {
                write!(f, "`{q}`")
            }
            ConValue::Function(func) => {
                let (pat, body) = func.decl();
                write!(f, "fn {pat} {body}")
            }
            ConValue::TupleConstructor(Constructor { name: index, arity }) => {
                write!(f, "{index}(..{arity})")
            }
            // ConValue::Closure(func) => {
            //     write!(f, "{}", func.as_ref())
            // }
            ConValue::Builtin(func) => {
                write!(f, "{}", func)
            }
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
