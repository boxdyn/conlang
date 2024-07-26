//! A [TypeKind] is a node in the [Table](crate::table::Table)'s type graph

use crate::handle::Handle;
use cl_ast::{Sym, Visibility};
use std::{fmt::Debug, str::FromStr};

mod display;

/// A [TypeKind] represents an item
/// (a component of a [Table](crate::table::Table))
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeKind {
    /// An alias for an already-defined type
    Instance(Handle),
    /// A primitive type, built-in to the compiler
    Intrinsic(Intrinsic),
    /// A user-defined aromatic data type
    Adt(Adt),
    /// A reference to an already-defined type: &T
    Ref(Handle),
    /// A contiguous view of dynamically sized memory
    Slice(Handle),
    /// A contiguous view of statically sized memory
    Array(Handle, usize),
    /// A tuple of existing types
    Tuple(Vec<Handle>),
    /// A function which accepts multiple inputs and produces an output
    FnSig { args: Handle, rety: Handle },
    /// The unit type
    Empty,
    /// The never type
    Never,
    /// An untyped module
    Module,
}

/// A user-defined Aromatic Data Type
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Adt {
    /// A union-like enum type
    Enum(Vec<(Sym, Option<Handle>)>),

    /// A structural product type with named members
    Struct(Vec<(Sym, Visibility, Handle)>),
    /// A structural product type with unnamed members
    TupleStruct(Vec<(Visibility, Handle)>),
    /// A structural product type of neither named nor unnamed members
    UnitStruct,

    /// A choose your own undefined behavior type
    /// TODO: should unions be a language feature?
    Union(Vec<(Sym, Handle)>),
}

/// The set of compiler-intrinsic types.
/// These primitive types have native implementations of the basic operations.
#[rustfmt::skip]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Intrinsic {
    I8, I16, I32, I64, I128, Isize, // Signed integers
    U8, U16, U32, U64, U128, Usize, // Unsigned integers
    F8, F16, F32, F64, F128, Fsize, // Floating point numbers
    Bool,                           // boolean value
    Char,                           // Unicode codepoint
}

// Author's note: the fsize type is a meme

impl FromStr for Intrinsic {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "i8" => Intrinsic::I8,
            "i16" => Intrinsic::I16,
            "i32" => Intrinsic::I32,
            "i64" => Intrinsic::I64,
            "i128" => Intrinsic::I128,
            "isize" => Intrinsic::Isize,
            "u8" => Intrinsic::U8,
            "u16" => Intrinsic::U16,
            "u32" => Intrinsic::U32,
            "u64" => Intrinsic::U64,
            "u128" => Intrinsic::U128,
            "usize" => Intrinsic::Usize,
            "f8" => Intrinsic::F8,
            "f16" => Intrinsic::F16,
            "f32" => Intrinsic::F32,
            "f64" => Intrinsic::F64,
            "f128" => Intrinsic::F128,
            "fsize" => Intrinsic::Fsize,
            "bool" => Intrinsic::Bool,
            "char" => Intrinsic::Char,
            _ => Err(())?,
        })
    }
}
