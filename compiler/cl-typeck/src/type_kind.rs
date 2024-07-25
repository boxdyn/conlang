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
    Ref(u16, Handle),
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
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Intrinsic {
    /// An 8-bit signed integer: `#[intrinsic = "i8"]`
    I8,
    /// A 16-bit signed integer: `#[intrinsic = "i16"]`
    I16,
    /// A 32-bit signed integer: `#[intrinsic = "i32"]`
    I32,
    /// A 64-bit signed integer: `#[intrinsic = "i32"]`
    I64,
    // /// A 128-bit signed integer: `#[intrinsic = "i32"]`
    // I128,
    /// A ptr-len signed integer: `#[intrinsic = "isize"]`
    Isize,
    /// An 8-bit unsigned integer: `#[intrinsic = "u8"]`
    U8,
    /// A 16-bit unsigned integer: `#[intrinsic = "u16"]`
    U16,
    /// A 32-bit unsigned integer: `#[intrinsic = "u32"]`
    U32,
    /// A 64-bit unsigned integer: `#[intrinsic = "u64"]`
    U64,
    // /// A 128-bit unsigned integer: `#[intrinsic = "u128"]`
    // U128,
    /// A ptr-len unsigned integer: `#[intrinsic = "isize"]`
    Usize,
    /// A boolean (`true` or `false`): `#[intrinsic = "bool"]`
    Bool,
    /// The unicode codepoint type: #[intrinsic = "char"]
    Char,
}

impl FromStr for Intrinsic {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "i8" => Intrinsic::I8,
            "i16" => Intrinsic::I16,
            "i32" => Intrinsic::I32,
            "i64" => Intrinsic::I64,
            "isize" => Intrinsic::Isize,
            "u8" => Intrinsic::U8,
            "u16" => Intrinsic::U16,
            "u32" => Intrinsic::U32,
            "u64" => Intrinsic::U64,
            "usize" => Intrinsic::Usize,
            "bool" => Intrinsic::Bool,
            "char" => Intrinsic::Char,
            _ => Err(())?,
        })
    }
}
