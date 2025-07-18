//! Values in the dynamically typed AST interpreter.
//!
//! The most permanent fix is a temporary one.
use cl_ast::{Expr, Sym, format::FmtAdapter};

use crate::{closure::Closure, constructor::Constructor};

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
    /// A string
    String(Sym),
    /// A reference
    Ref(usize),
    /// A reference to an array
    Slice(usize, usize),
    /// An Array
    Array(Box<[ConValue]>),
    /// A tuple
    Tuple(Box<[ConValue]>),
    /// A value of a product type
    Struct(Box<(&'static str, HashMap<Sym, ConValue>)>),
    /// A value of a product type with anonymous members
    TupleStruct(Box<(&'static str, Box<[ConValue]>)>),
    /// An entire namespace
    Module(Box<HashMap<Sym, ConValue>>),
    /// A namespace, sans storage
    Module2(HashMap<Sym, usize>),
    /// A quoted expression
    Quote(Box<Expr>),
    /// A callable thing
    Function(Rc<Function>),
    /// A tuple constructor
    TupleConstructor(Constructor),
    /// A closure, capturing by reference
    Closure(Rc<Closure>),
    /// A built-in function
    Builtin(&'static Builtin),
}

impl ConValue {
    /// Gets whether the current value is true or false
    pub fn truthy(&self) -> IResult<bool> {
        match self {
            ConValue::Bool(v) => Ok(*v),
            _ => Err(Error::TypeError())?,
        }
    }

    pub fn typename(&self) -> IResult<&'static str> {
        Ok(match self {
            ConValue::Empty => "Empty",
            ConValue::Int(_) => "i64",
            ConValue::Float(_) => "f64",
            ConValue::Bool(_) => "bool",
            ConValue::Char(_) => "char",
            ConValue::String(_) => "String",
            ConValue::Ref(_) => "Ref",
            ConValue::Slice(_, _) => "Slice",
            ConValue::Array(_) => "Array",
            ConValue::Tuple(_) => "Tuple",
            ConValue::Struct(_) => "Struct",
            ConValue::TupleStruct(_) => "TupleStruct",
            ConValue::Module(_) => "",
            ConValue::Module2(_) => "",
            ConValue::Quote(_) => "Quote",
            ConValue::Function(_) => "Fn",
            ConValue::TupleConstructor(_) => "Fn",
            ConValue::Closure(_) => "Fn",
            ConValue::Builtin(_) => "Fn",
        })
    }

    #[allow(non_snake_case)]
    pub fn TupleStruct(id: Sym, values: Box<[ConValue]>) -> Self {
        Self::TupleStruct(Box::new((id.to_ref(), values)))
    }
    #[allow(non_snake_case)]
    pub fn Struct(id: Sym, values: HashMap<Sym, ConValue>) -> Self {
        Self::Struct(Box::new((id.to_ref(), values)))
    }

    pub fn index(&self, index: &Self, _env: &Environment) -> IResult<ConValue> {
        let &Self::Int(index) = index else {
            Err(Error::TypeError())?
        };
        match self {
            ConValue::String(string) => string
                .chars()
                .nth(index as _)
                .map(ConValue::Char)
                .ok_or(Error::OobIndex(index as usize, string.chars().count())),
            ConValue::Array(arr) => arr
                .get(index as usize)
                .cloned()
                .ok_or(Error::OobIndex(index as usize, arr.len())),
            &ConValue::Slice(id, len) => {
                let index = if index < 0 {
                    len.wrapping_add_signed(index)
                } else {
                    index as usize
                };
                if index < len {
                    Ok(ConValue::Ref(id + index))
                } else {
                    Err(Error::OobIndex(index, len))
                }
            }
            _ => Err(Error::TypeError()),
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
            ConValue::Closure(func) => func.name(),
            ConValue::Builtin(func) => func.name(),
            _ => "".into(),
        }
    }
    fn call(&self, env: &mut Environment, args: &[ConValue]) -> IResult<ConValue> {
        match self {
            Self::Function(func) => func.call(env, args),
            Self::TupleConstructor(func) => func.call(env, args),
            Self::Closure(func) => func.call(env, args),
            Self::Builtin(func) => func.call(env, args),
            Self::Module(m) => {
                if let Some(func) = m.get(&"call".into()) {
                    func.call(env, args)
                } else {
                    Err(Error::NotCallable(self.clone()))
                }
            }
            &Self::Ref(ptr) => {
                // Move onto stack, and call
                let func = env.get_id(ptr).ok_or(Error::StackOverflow(ptr))?.clone();
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
            (Self::String(a), Self::String(b)) => Ok(Self::Bool(&**a $op &**b)),
            _ => Err(Error::TypeError())
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
    Expr => ConValue::Quote,
    String => ConValue::String,
    Rc<str> => ConValue::String,
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
        (ConValue::String(a), ConValue::String(b)) => (a.to_string() + &*b).into(),
        (ConValue::String(s), ConValue::Char(c)) => { let mut s = s.to_string(); s.push(c); s.into() }
        (ConValue::Char(a), ConValue::Char(b)) => {
            ConValue::String([a, b].into_iter().collect::<String>().into())
        }
        _ => Err(Error::TypeError())?
        ]
    BitAnd: bitand = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a & b),
        (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a & b),
        _ => Err(Error::TypeError())?
    ]
    BitOr: bitor = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a | b),
        (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a | b),
        _ => Err(Error::TypeError())?
    ]
    BitXor: bitxor = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a ^ b),
        (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a ^ b),
        _ => Err(Error::TypeError())?
    ]
    Div: div = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.checked_div(b).unwrap_or_else(|| {
            eprintln!("Warning: Divide by zero in {a} / {b}"); a
        })),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a / b),
        _ => Err(Error::TypeError())?
    ]
    Mul: mul = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_mul(b)),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a * b),
        _ => Err(Error::TypeError())?
    ]
    Rem: rem = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.checked_rem(b).unwrap_or_else(|| {
            println!("Warning: Divide by zero in {a} % {b}"); a
        })),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a % b),
        _ => Err(Error::TypeError())?
    ]
    Shl: shl = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_shl(b as _)),
        _ => Err(Error::TypeError())?
    ]
    Shr: shr = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_shr(b as _)),
        _ => Err(Error::TypeError())?
    ]
    Sub: sub = [
        (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_sub(b)),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a - b),
        _ => Err(Error::TypeError())?
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
            ConValue::TupleStruct(parts) => {
                let (id, tuple) = parts.as_ref();
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
            ConValue::Struct(parts) => {
                let (id, map) = parts.as_ref();
                use std::fmt::Write;
                write!(f, "{id} ")?;
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
                    write!(f, "\n{k}: {v},")?;
                }
                Ok(())
            }
            ConValue::Module2(module) => {
                use std::fmt::Write;
                let mut f = f.delimit_with("{", "\n}");
                for (k, v) in module.iter() {
                    write!(f, "\n{k}: <{v}>,")?;
                }
                Ok(())
            }
            ConValue::Quote(q) => {
                write!(f, "`{q}`")
            }
            ConValue::Function(func) => {
                write!(f, "{}", func.decl())
            }
            ConValue::TupleConstructor(Constructor { name: index, arity }) => {
                write!(f, "{index}(..{arity})")
            }
            ConValue::Closure(func) => {
                write!(f, "{}", func.as_ref())
            }
            ConValue::Builtin(func) => {
                write!(f, "{}", func.description())
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
