//! A [TypeKind] is a node in the [Table](crate::table::Table)'s type graph

use crate::table::Scope;
use cl_ast::types::Symbol;
use std::{fmt::Debug, str::FromStr};

mod display;

/// A [TypeKind] represents an item
/// (a component of a [Table](crate::table::Table))
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeKind {
    /// A type that is yet to be inferred!
    Inferred,
    /// A type variable, to be monomorphized
    Variable,
    /// An alias for an already-defined type
    Instance(Scope),
    /// A primitive type, built-in to the compiler
    Primitive(Primitive),
    /// A user-defined aromatic data type
    Adt(Adt),
    /// A reference to an already-defined type: &T
    Ref(Scope),
    /// A raw pointer to an already-defined type: &T
    Ptr(Scope),
    /// A contiguous view of dynamically sized memory
    Slice(Scope),
    /// A contiguous view of statically sized memory
    Array(Scope, usize),
    /// A tuple of existing types
    Tuple(Vec<Scope>),
    /// A function which accepts multiple inputs and produces an output
    FnSig { args: Scope, rety: Scope },
    /// An untyped module
    Module,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Visibility {
    Public,
    Private,
}

/// A user-defined Aromatic Data Type
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Adt {
    /// A union-like enum type
    Enum(Vec<(Symbol, Scope)>),

    /// A structural product type with named members
    Struct(Vec<(Symbol, Visibility, Scope)>),
    /// A structural product type with unnamed members
    TupleStruct(Vec<(Visibility, Scope)>),
    /// A structural product type of neither named nor unnamed members
    UnitStruct,

    /// A choose your own undefined behavior type
    /// TODO: should unions be a language feature?
    Union(Vec<(Symbol, Scope)>),
}

/// The set of compiler-intrinsic types.
/// These primitive types have native implementations of the basic operations.
#[rustfmt::skip]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Primitive {
    I8, I16, I32, I64, I128, Isize, // Signed integers
    U8, U16, U32, U64, U128, Usize, // Unsigned integers
    F8, F16, F32, F64, F128, Fsize, // Floating point numbers
    Integer, Float,                 // Inferred int and float
    Bool,                           // boolean value
    Char,                           // Unicode codepoint
    Str,                            // UTF-8 string
    Never,                          // The never type
}

#[rustfmt::skip]
impl Primitive {
    /// Checks whether self is an integer
    pub fn is_integer(self) -> bool {
        matches!(
            self, 
            | Self::I8 | Self::I16 | Self::I32 | Self::I64 | Self::I128 | Self::Isize
            | Self::U8 | Self::U16 | Self::U32 | Self::U64 | Self::U128 | Self::Usize
            | Self::Integer
        )
    }
    /// Checks whether self is a floating point number
    pub fn is_float(self) -> bool {
        matches!(
            self, 
            | Self::F8 | Self::F16 | Self::F32 | Self::F64 | Self::F128 | Self::Fsize
            | Self::Float
        )
    }
}

// Author's note: the fsize type is a meme

impl FromStr for Primitive {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "i8" => Primitive::I8,
            "i16" => Primitive::I16,
            "i32" => Primitive::I32,
            "i64" => Primitive::I64,
            "i128" => Primitive::I128,
            "isize" => Primitive::Isize,
            "u8" => Primitive::U8,
            "u16" => Primitive::U16,
            "u32" => Primitive::U32,
            "u64" => Primitive::U64,
            "u128" => Primitive::U128,
            "usize" => Primitive::Usize,
            "f8" => Primitive::F8,
            "f16" => Primitive::F16,
            "f32" => Primitive::F32,
            "f64" => Primitive::F64,
            "f128" => Primitive::F128,
            "fsize" => Primitive::Fsize,
            "bool" => Primitive::Bool,
            "char" => Primitive::Char,
            "str" => Primitive::Str,
            "never" => Primitive::Never,
            _ => Err(())?,
        })
    }
}
