//! [Conlang Values](ConValue) in the dynamically typed AST interpreter.
//!
//! > The most permanent fix is a temporary one.
use cl_ast::{At, Expr, fmt::FmtAdapter, types::Symbol};
use cl_structures::intern::interned::Interned;

use crate::{
    closurpreter::Clunktion,
    place::Place,
    typeinfo::{Model, Type},
};

use super::{
    Callable, Environment,
    builtin::Builtin,
    error::{Error, IResult},
    function::Function,
};
use std::{collections::HashMap, ops::*};

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

pub type Integer = i128;

/// A Conlang value stores data in the interpreter
#[derive(Clone, Debug, Default)]
pub enum ConValue {
    /// The empty/unit `()` type
    #[default]
    Unit,
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
    /// An Array
    Array(Vec<ConValue>),
    /// A tuple
    Tuple(Vec<ConValue>),
    // TODO: Instead of storing the identifier, store the index of the struct module
    /// A value of a product type
    Struct(Type, Box<HashMap<Symbol, ConValue>>),
    /// A value of a product type with anonymous members
    TupleStruct(Type, Vec<ConValue>),
    /// An entire namespace
    Module(Box<HashMap<Symbol, ConValue>>),
    /// A quoted expression
    Quote(Box<At<Expr>>),
    /// A callable thing
    Function(Function),
    /// A closure-function
    Clunktion(Clunktion),
    /// A built-in function
    Builtin(&'static Builtin),
    /// The definition of a type, by index
    TypeInfo(Type),
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
    fn call(&self, env: &mut Environment, args: Vec<ConValue>) -> IResult<ConValue> {
        let out = match self {
            Self::Function(func) => func.call(env, args),
            Self::Clunktion(clunk) => clunk.call(env, args),
            Self::Builtin(func) => func.call(env, args),
            Self::Struct(..) | Self::TupleStruct(..) => {
                let id = env.stack_alloc(self.clone())?;
                Place::from_index(id).call(env, args)
            }
            Self::Module(m) => match m.get(&env.kw.call) {
                Some(func) => func.call(env, args),
                None => Err(Error::NotCallable(self.clone())),
            },
            Self::Ref(ptr) => ptr.call(env, args),
            Self::TypeInfo(idx) => idx.call(env, args),
            _ => Err(Error::NotCallable(self.clone())),
        }?;
        Ok(out)
    }
}

impl ConValue {
    /// Gets whether the current value is true or false
    pub fn truthy(&self, env: &Environment) -> IResult<bool> {
        match self {
            ConValue::Bool(v) => Ok(*v),
            ConValue::Int(v) => Ok(*v != 0),
            _ => Err(Error::TypeError("type implements Truth", self.type_of(env)))?,
        }
    }

    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }

    /// Implements the overloadable `?` operator
    pub fn try_hard(self, env: &mut Environment) -> IResult<Self> {
        /// Special-case the builtin try-able data structures nominally, cuz it's fast
        fn be_lazy(value: ConValue) -> IResult<Result<ConValue, ConValue>> {
            let (ty, mut vs) = match value {
                ConValue::TupleStruct(ty, vs) if vs.len() == 1 => (ty, vs),
                ConValue::TypeInfo(ty) if let "None" = ty.name() => Err(Error::Return(value))?,
                _ => return Ok(Err(value)),
            };
            match ty.name() {
                // Tried::{Take, Return}
                "Take" => return Ok(Ok(vs[0].take())),
                "Return" => return Err(Error::Return(vs[0].take())),
                // Option::{Some, None}
                "Some" => return Ok(Ok(vs[0].take())),
                "None" => return Err(Error::Return(ConValue::TypeInfo(ty))),
                // Result::{Ok, Err}
                "Ok" => return Ok(Ok(vs[0].take())),
                "Err" => return Err(Error::Return(ConValue::TupleStruct(ty, vs))),
                _ => Ok(Err(ConValue::TupleStruct(ty, vs))),
            }
        }

        be_lazy(self)?.or_else(|e| {
            let ty = e.type_of(env);
            let try_harder = env.get_impl(ty, env.kw.kw_try)?;
            be_lazy(try_harder.call(env, vec![e])?)?
                .map_err(|e| Error::TypeError("enum Tried", e.type_of(env)))
        })
    }

    pub fn is_cheap_to_copy(&self) -> bool {
        match self {
            Self::Unit
            | Self::Int(_)
            | Self::Float(_)
            | Self::Bool(_)
            | Self::Char(_)
            | Self::Str(_) => true,
            Self::Ref(_) => true,
            Self::Quote(_) => true,
            Self::Function(_) => true,
            Self::Clunktion(_) => true,
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

    pub fn is_primitive(&self) -> bool {
        match self {
            ConValue::Unit => true,
            ConValue::Int(_) => true,
            ConValue::Float(_) => true,
            ConValue::Bool(_) => true,
            ConValue::Char(_) => true,
            ConValue::Str(_) => true,
            ConValue::String(_) => true,
            ConValue::Ref(_) => true,
            ConValue::Array(_) => true,
            ConValue::Tuple(_) => true,
            ConValue::Quote(_) => true,
            ConValue::Function(_) => true,
            ConValue::Clunktion(_) => true,
            ConValue::Builtin(_) => true,
            ConValue::Struct(_, _) => false,
            ConValue::TupleStruct(_, _) => false,
            ConValue::TypeInfo(_) => false,
            ConValue::Module(_) => false,
        }
    }

    pub fn type_of(&self, env: &Environment) -> Type {
        fn type_of_place(place: &Place, env: &Environment) -> Type {
            place
                .get(env)
                .map(|v| v.type_of(env))
                .unwrap_or_else(|_| Model::Any.already_interned())
        }

        match self {
            Self::Unit => Model::Unit { name: None, parent: None, value: 0 }.already_interned(),
            Self::Int(_) => Model::default_integer(),
            Self::Float(_) => Model::default_float(),
            Self::Bool(_) => Model::Bool.already_interned(),
            Self::Char(_) => Model::Char.already_interned(),
            Self::Str(_) => Model::Str.already_interned(),
            Self::String(_) => Model::Str.already_interned(),
            Self::Ref(place) => Model::Ref(type_of_place(place, env)).intern(),
            Self::Array(arr) if !arr.is_empty() => Model::Slice(arr[0].type_of(env)).intern(),
            Self::Array(_) => Model::Slice(Model::Any.already_interned()).intern(),
            Self::Tuple(vs) => Model::Tuple {
                name: None,
                parent: None,
                fields: vs.iter().map(|v| v.type_of(env)).collect(),
            }
            .intern(),
            Self::Struct(ty, _) => *ty,
            Self::TupleStruct(ty, _) => *ty,
            Self::Module(_) => Model::Any.already_interned(),
            Self::Quote(_) => Model::Any.already_interned(),
            Self::Function(_) => Model::Function.already_interned(),
            Self::Clunktion(_) => Model::Function.already_interned(),
            Self::Builtin(_) => Model::Function.already_interned(),
            Self::TypeInfo(ty) => *ty,
        }
    }

    pub fn cast(self, ty: &Model, env: &Environment) -> Self {
        match ty {
            &Model::Integer { signed, size: _, min, max } => {
                let i = match self {
                    Self::Int(v) => v,
                    Self::Float(v) => v as _,
                    Self::Bool(v) => v as _,
                    Self::Char(v) => v as _,
                    Self::TypeInfo(Interned(Model::Unit { name: _, parent: _, value: d }, ..)) => {
                        *d as _
                    }
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
            Model::Float { size: _ } => {
                let f = match self {
                    Self::Float(v) => v,
                    Self::Int(v) => v as _,
                    Self::Bool(v) => v as i32 as _,
                    Self::Char(v) => v as i32 as _,
                    Self::TypeInfo(Interned(Model::Unit { name: _, parent: _, value: d }, ..)) => {
                        *d as _
                    }
                    _ => return self,
                };
                ConValue::Float(f)
            }
            Model::Bool => ConValue::Bool(self.truthy(env).unwrap_or(true)),
            Model::Char => {
                let c = match self {
                    Self::Int(v) => v as _,
                    Self::Float(v) => v as _,
                    Self::Bool(v) => v as _,
                    Self::Char(_) => return self,
                    Self::TypeInfo(Interned(Model::Unit { name: _, parent: _, value: d }, ..)) => {
                        *d as _
                    }
                    _ => return self,
                };
                ConValue::Char(char::from_u32(c).unwrap_or('�'))
            }
            Model::Unit { name: None, .. } => ConValue::Unit,
            Model::Unit { .. } => ConValue::TypeInfo(ty.already_interned()),
            Model::Str => ConValue::String(self.to_string()),
            _ => self,
        }
    }

    pub fn dereference_in<'e>(&'e self, env: &'e Environment) -> IResult<&'e Self> {
        let mut value = self;
        while let ConValue::Ref(r) = value {
            value = r.get(env)?;
            // Should never happen, but just in case:
            if matches!(value, ConValue::Ref(r2) if r == r2) {
                break;
            }
        }
        Ok(value)
    }
}

/// Templates comparison functions for [ConValue]
macro cmp (with $env:ident; $($fn:ident: $op:tt);*$(;)?) {$(
    /// TODO: Remove when functions are implemented:
    ///       Desugar into function calls
    pub fn $fn(&self, other: &Self, $env: &mut Environment) -> IResult<Self> {
        Ok(ConValue::Bool(self.internal_compare(other, $env)? $op 0))
    }
)*}
impl ConValue {
    cmp! {
        with env;
        lt: <;
        lt_eq: <=;
        eq: ==;
        neq: !=;
        gt_eq: >=;
        gt: >;
    }
    pub fn internal_compare(&self, other: &Self, env: &mut Environment) -> IResult<Integer> {
        use std::cmp::Ord;
        Ok(match (self, other) {
            (Self::Unit, Self::Unit) => 0,
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
                for (a, b) in a.iter().zip(b.iter()) {
                    match a.internal_compare(b, env)? {
                        0 => continue,
                        res => return Ok(res),
                    }
                }
                a.len().cmp(&b.len()) as _
            }
            (Self::TypeInfo(a), Self::TypeInfo(b)) => a.cmp(b) as _,
            (a, b)
                if let ty = a.type_of(env)
                    && let Ok(f) = env.get_impl(ty, env.kw.cmp) =>
            {
                match f.call(env, vec![a.clone(), b.clone()])? {
                    ConValue::Int(i) => i,
                    other => Err(Error::TypeError("{integer}", other.type_of(env)))?,
                }
            }
            (ConValue::Ref(place), b) => match place.get(env)?.clone().cmp(b.clone(), env)? {
                ConValue::Int(i) => i,
                other => Err(Error::TypeError("{integer}", other.type_of(env)))?,
            },
            (a, b) => Err(Error::TypeError(
                format_args!("{a}.cmp({b})"),
                a.type_of(env),
            ))?,
        })
    }
}

/// Implements binary operators for [ConValue]
///
/// Performs autodereference on self, and dispatches by [Type] for ADTs
macro bin_ops(with $env:ident: Environment; $(
    $trait:ty: $fn:ident = [$($match:tt)*]
        $(else $other: pat => $expr: expr;)?
)*) {
    $(impl ConValue {
        #[doc = concat!["Implements the [`", stringify!($trait), "`] operator"]]
        pub fn $fn(self, rhs: Self, $env: &mut Environment) -> IResult<Self> {
            Ok(match (self, rhs) {
                (ConValue::Ref(place), b) => place.get($env)?.clone().$fn(b, $env)?,
                $($match)*
                #[allow(unreachable_patterns)]
                (a, b) if let ty = a.type_of($env)
                  && let Ok(f) = $env.get_impl(ty, $env.kw.$fn) => {
                    f.call($env, vec![a, b])?
                }
                $($other => $expr)?
            })
        }
    })*
}

bin_ops! {
    with env: Environment;
    Index: index = [
        (ConValue::Array(arr), ConValue::Int(index @ ..0)) => {
            arr.get(arr.len().wrapping_add_signed(index as _))
                .cloned()
                .ok_or(Error::OobIndex(arr.len().wrapping_add_signed(index as _), arr.len()))?
        }
        (ConValue::Array(arr), ConValue::Int(index)) => arr
            .get(index as usize)
            .cloned()
            .ok_or(Error::OobIndex(index as usize, arr.len()))?,
    ] else (a, idx) => Err(Error::TypeError(format!("{}.index({idx})", a.type_of(env)), idx.type_of(env)))?;
    Ord: cmp = [
        (a, b) if a.is_primitive() => ConValue::Int(a.internal_compare(&b, env)?),
    ] else (a, b) => ConValue::Int(a.internal_compare(&b, env)?);
    Mul: mul = [
        (ConValue::Unit, ConValue::Unit) => ConValue::Unit,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_mul(b)),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a * b),
    ] else (a, b) => Err(Error::TypeError(a.type_of(env), b.type_of(env)))?;
    Div: div = [
        (ConValue::Unit, ConValue::Unit) => ConValue::Unit,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.checked_div(b).unwrap_or_else(|| {
            eprintln!("Warning: Divide by zero in {a} / {b}"); a
        })),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a / b),
    ] else (a, b) => Err(Error::TypeError(a.type_of(env), b.type_of(env)))?;
    Rem: rem = [
        (ConValue::Unit, ConValue::Unit) => ConValue::Unit,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.checked_rem(b).unwrap_or_else(|| {
            println!("Warning: Divide by zero in {a} % {b}"); a
        })),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a % b),
    ] else (a, b) => Err(Error::TypeError(a.type_of(env), b.type_of(env)))?;
    Add: add = [
        (ConValue::Unit, ConValue::Unit) => ConValue::Unit,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_add(b)),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a + b),
        (ConValue::Str(a), ConValue::Str(b)) => (a.to_string() + b.to_ref()).into(),
        (ConValue::Str(a), ConValue::String(b)) => (a.to_string() + &b).into(),
        (ConValue::String(a), ConValue::Str(b)) => (a + b.to_ref()).into(),
        (ConValue::String(a), ConValue::String(b)) => (a + &b).into(),
        (ConValue::Str(s), ConValue::Char(c)) => { let mut s = s.to_string(); s.push(c); s.into() }
        (ConValue::String(mut s), ConValue::Char(c)) => { s.push(c); ConValue::String(s) }
        (ConValue::Char(a), ConValue::Char(b)) => {
            ConValue::String([a, b].into_iter().collect::<String>())
        }
    ] else (a, b) => Err(Error::TypeError(a.type_of(env), b.type_of(env)))?;
    Sub: sub = [
        (ConValue::Unit, ConValue::Unit) => ConValue::Unit,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_sub(b)),
        (ConValue::Float(a), ConValue::Float(b)) => ConValue::Float(a - b),
    ] else (a, b) => Err(Error::TypeError(a.type_of(env), b.type_of(env)))?;
    Shl: shl = [
        (ConValue::Unit, ConValue::Unit) => ConValue::Unit,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_shl(b as _)),
        (ConValue::Int(_), b) => Err(Error::TypeError("i32", b.type_of(env)))?,
    ] else (a, _) => Err(Error::TypeError("Shl", a.type_of(env)))?;
    Shr: shr = [
        (ConValue::Unit, ConValue::Unit) => ConValue::Unit,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_shr(b as _)),
        (ConValue::Int(_), b) => Err(Error::TypeError("i32", b.type_of(env)))?,
    ] else (a, _) => Err(Error::TypeError("Shr", a.type_of(env)))?;
    BitAnd: and = [
        (ConValue::Unit, ConValue::Unit) => ConValue::Unit,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a & b),
        (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a & b),
    ] else (a, b) => Err(Error::TypeError(a.type_of(env), b.type_of(env)))?;
    BitOr: or = [
        (ConValue::Unit, ConValue::Unit) => ConValue::Unit,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a | b),
        (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a | b),
    ] else (a, b) => Err(Error::TypeError(a.type_of(env), b.type_of(env)))?;
    BitXor: xor = [
        (ConValue::Unit, ConValue::Unit) => ConValue::Unit,
        (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a ^ b),
        (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a ^ b),
    ] else (a, b) => Err(Error::TypeError(a.type_of(env), b.type_of(env)))?;
}

/// Implements unary operators for [ConValue]
///
/// Performs autodereference on self, and dispatches by [Type] for ADTs
macro un_ops(with $env:ident: Environment; $(
    $trait:ty: $fn:ident = [$($match:tt)*]
        $(else $other: pat => $expr: expr;)?
)*) {
    $(impl ConValue {
        #[doc = concat!["Implements the [`", stringify!($trait), "`] operator"]]
        pub fn $fn(self, $env: &mut Environment) -> IResult<Self> {
            Ok(match self {
                ConValue::Ref(place) => place.get($env)?.clone().$fn($env)?,
                $($match)*
                v if let ty = v.type_of($env)
                  && let Ok(f) = $env.get_impl(ty, $env.kw.$fn) => {
                    f.call($env, vec![v])?
                }
                $($other => $expr)?
            })
        }
    })*
}

un_ops! {
    with env: Environment;
    Neg: neg = [
        ConValue::Unit => ConValue::Unit,
        ConValue::Int(v) => ConValue::Int(-v),
        ConValue::Float(v) => ConValue::Float(-v),
    ] else other => Err(Error::TypeError("Neg", other.type_of(env)))?;
    Not: not = [
        ConValue::Unit => ConValue::Unit,
        ConValue::Int(v) => ConValue::Int(!v),
        ConValue::Bool(v) => ConValue::Bool(!v),
    ] else other => Err(Error::TypeError("Not", other.type_of(env)))?;
}

impl std::fmt::Display for ConValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::sync::LazyLock;
        static INT_FORMAT: LazyLock<String> =
            LazyLock::new(|| std::env::var("CONLANG_INT").unwrap_or_default());
        match self {
            ConValue::Unit => "Unit".fmt(f),
            ConValue::Int(v) => match INT_FORMAT.as_str() {
                "b" => write!(f, "{v:b}"),
                "o" => write!(f, "{v:o}"),
                "x" => write!(f, "{v:x}"),
                "#b" => write!(f, "{v:#08b}"),
                "#o" => write!(f, "{v:#03o}"),
                "#x" => write!(f, "{v:#02x}"),
                _ => v.fmt(f),
            },
            ConValue::Float(v) => v.fmt(f),
            ConValue::Bool(v) => v.fmt(f),
            ConValue::Char(v) => v.fmt(f),
            ConValue::Str(v) => v.fmt(f),
            ConValue::String(v) => v.fmt(f),
            ConValue::Ref(v) => write!(f, "&<{}>", v),
            ConValue::Array(array) => f.delimit('[', ']').list(array, ", "),
            ConValue::Tuple(tuple) => f.delimit('(', ')').list(tuple, ", "),
            ConValue::TupleStruct(id, tuple) => f
                .delimit(format_args!("{}(", id.name()), ")")
                .list(tuple, ", "),
            ConValue::Struct(id, map) => {
                use std::fmt::Write;
                match id.name() {
                    "" => {}
                    name => write!(f, "{name} ")?,
                }
                let mut f = f.delimit_indented("{", "\n}");
                for (k, v) in map.iter() {
                    write!(f, "\n{k}: {v},")?;
                }
                Ok(())
            }
            ConValue::Module(module) => {
                use std::fmt::Write;
                let mut f = f.delimit_indented("mod {", "\n}");
                let mut items = module.iter().collect::<Vec<_>>();
                items.sort_by_key(|i| i.0);
                for (k, v) in items {
                    writeln!(f)?;
                    write!(f.indent(), "{k}: {v},")?;
                }
                Ok(())
            }
            ConValue::Quote(q) => write!(f, "`{q}`"),
            ConValue::Clunktion(func) => func.fmt(f),
            ConValue::Function(func) => func.fmt(f),
            ConValue::Builtin(func) => func.fmt(f),
            ConValue::TypeInfo(ty) => ty.fmt(f),
        }
    }
}

/// Implements [From] for an enum with 1-tuple variants
macro from ($($T:ty => $v:expr),*$(,)?) {
    $(impl From<$T> for ConValue {
        fn from(value: $T) -> Self { $v(value.into()) }
    })*
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
    Type => ConValue::TypeInfo,
}
impl From<()> for ConValue {
    fn from(_: ()) -> Self {
        Self::Unit
    }
}
impl From<&[ConValue]> for ConValue {
    fn from(value: &[ConValue]) -> Self {
        match value {
            [] => Self::Unit,
            [value] => value.clone(),
            _ => Self::Tuple(value.into()),
        }
    }
}
