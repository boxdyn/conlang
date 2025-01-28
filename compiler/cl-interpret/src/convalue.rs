//! Values in the dynamically typed AST interpreter.
//!
//! The most permanent fix is a temporary one.
use cl_ast::{format::FmtAdapter, Sym};

use super::{
    error::{Error, IResult},
    function::Function,
    BuiltIn, Callable, Environment,
};
use std::{collections::HashMap, ops::*, rc::Rc};

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
    /// A string
    String(Sym),
    /// A reference
    Ref(Rc<ConValue>),
    /// An Array
    Array(Box<[ConValue]>),
    /// A tuple
    Tuple(Box<[ConValue]>),
    /// An exclusive range
    RangeExc(Integer, Integer),
    /// An inclusive range
    RangeInc(Integer, Integer),
    /// A value of a product type
    Struct(Box<(Sym, HashMap<Sym, ConValue>)>),
    /// An entire namespace
    Module(Box<HashMap<Sym, Option<ConValue>>>),
    /// A callable thing
    Function(Rc<Function>),
    /// A built-in function
    BuiltIn(&'static dyn BuiltIn),
}

impl ConValue {
    /// Gets whether the current value is true or false
    pub fn truthy(&self) -> IResult<bool> {
        match self {
            ConValue::Bool(v) => Ok(*v),
            _ => Err(Error::TypeError)?,
        }
    }
    pub fn range_exc(self, other: Self) -> IResult<Self> {
        let (Self::Int(a), Self::Int(b)) = (self, other) else {
            Err(Error::TypeError)?
        };
        Ok(Self::RangeExc(a, b))
    }
    pub fn range_inc(self, other: Self) -> IResult<Self> {
        let (Self::Int(a), Self::Int(b)) = (self, other) else {
            Err(Error::TypeError)?
        };
        Ok(Self::RangeInc(a, b))
    }
    pub fn index(&self, index: &Self) -> IResult<ConValue> {
        let Self::Int(index) = index else {
            Err(Error::TypeError)?
        };
        match self {
            ConValue::String(string) => string
                .chars()
                .nth(*index as _)
                .map(ConValue::Char)
                .ok_or(Error::OobIndex(*index as usize, string.chars().count())),
            ConValue::Array(arr) => arr
                .get(*index as usize)
                .cloned()
                .ok_or(Error::OobIndex(*index as usize, arr.len())),
            _ => Err(Error::TypeError),
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
    fn name(&self) -> Sym {
        match self {
            ConValue::Function(func) => func.name(),
            ConValue::BuiltIn(func) => func.name(),
            _ => "".into(),
        }
    }
    fn call(&self, interpreter: &mut Environment, args: &[ConValue]) -> IResult<ConValue> {
        match self {
            Self::Function(func) => func.call(interpreter, args),
            Self::BuiltIn(func) => func.call(interpreter, args),
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
            (Self::String(a), Self::String(b)) => Ok(Self::Bool(&**a $op &**b)),
            _ => Err(Error::TypeError)
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
impl From<&Sym> for ConValue {
    fn from(value: &Sym) -> Self {
        ConValue::String(*value)
    }
}
from! {
    Integer => ConValue::Int,
    f64 => ConValue::Float,
    bool => ConValue::Bool,
    char => ConValue::Char,
    Sym => ConValue::String,
    &str => ConValue::String,
    String => ConValue::String,
    Rc<str> => ConValue::String,
    Function => ConValue::Function,
    Vec<ConValue> => ConValue::Tuple,
    &'static dyn BuiltIn => ConValue::BuiltIn,
}
impl From<()> for ConValue {
    fn from(_: ()) -> Self {
        Self::Empty
    }
}
impl From<&[ConValue]> for ConValue {
    fn from(value: &[ConValue]) -> Self {
        match value.len() {
            0 => Self::Empty,
            1 => value[0].clone(),
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
        (ConValue::String(a), ConValue::String(b)) => (a.to_string() + &*b).into(),
        (ConValue::String(s), ConValue::Char(c)) => { let mut s = s.to_string(); s.push(c); s.into() }
        (ConValue::Char(a), ConValue::Char(b)) => {
            ConValue::String([a, b].into_iter().collect::<String>().into())
        }
        _ => Err(Error::TypeError)?
        ]
    BitAnd: bitand = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a & b),
        (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a & b),
        _ => Err(Error::TypeError)?
    ]
    BitOr: bitor = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a | b),
        (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a | b),
        _ => Err(Error::TypeError)?
    ]
    BitXor: bitxor = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a ^ b),
        (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a ^ b),
        _ => Err(Error::TypeError)?
    ]
    Div: div = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.checked_div(b).unwrap_or_else(|| {
            eprintln!("Warning: Divide by zero in {a} / {b}"); a
        })),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a / b),
        _ => Err(Error::TypeError)?
    ]
    Mul: mul = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_mul(b)),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a * b),
        _ => Err(Error::TypeError)?
    ]
    Rem: rem = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.checked_rem(b).unwrap_or_else(|| {
            println!("Warning: Divide by zero in {a} % {b}"); a
        })),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a % b),
        _ => Err(Error::TypeError)?
    ]
    Shl: shl = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_shl(b as _)),
        _ => Err(Error::TypeError)?
    ]
    Shr: shr = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_shr(b as _)),
        _ => Err(Error::TypeError)?
    ]
    Sub: sub = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_sub(b)),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a - b),
        _ => Err(Error::TypeError)?
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
            ConValue::String(v) => v.fmt(f),
            ConValue::Ref(v) => write!(f, "&{v}"),
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
            ConValue::RangeExc(a, b) => write!(f, "{a}..{}", b + 1),
            ConValue::RangeInc(a, b) => write!(f, "{a}..={b}"),
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
            ConValue::Struct(parts) => {
                let (name, map) = parts.as_ref();
                use std::fmt::Write;
                if !name.is_empty() {
                    write!(f, "{name}: ")?;
                }
                let mut f = f.delimit_with("{", "\n}");
                for (k, v) in map.iter() {
                    write!(f, "\n{k}: {v},")?;
                }
                Ok(())
            }
            ConValue::Module(module) => {
                use std::fmt::Write;
                let mut f = f.delimit_with("{", "\n}");
                for (k, v) in module.iter() {
                    write!(f, "\n{k}: ")?;
                    match v {
                        Some(v) => write!(f, "{v},"),
                        None => write!(f, "_,"),
                    }?
                }
                Ok(())
            }
            ConValue::Function(func) => {
                write!(f, "{}", func.decl())
            }
            ConValue::BuiltIn(func) => {
                write!(f, "{}", func.description())
            }
        }
    }
}
