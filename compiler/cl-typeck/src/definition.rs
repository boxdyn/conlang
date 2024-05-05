use crate::{
    key::DefID,
    module::Module,
    node::{Node, NodeSource},
};
use cl_ast::{Meta, Sym, Visibility};
use std::{fmt::Debug, str::FromStr};

mod display;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Def<'a> {
    pub node: Node<'a>,
    pub kind: DefKind,
    pub module: Module,
}

impl<'a> Def<'a> {
    pub fn with_node(node: Node<'a>) -> Self {
        Self { node, kind: DefKind::Undecided, module: Default::default() }
    }
}

impl Def<'_> {
    pub fn name(&self) -> Option<Sym> {
        match self.node.kind {
            Some(source) => source.name(),
            None => None,
        }
    }
}

mod builder_functions {
    use super::*;

    impl<'a> Def<'a> {
        pub fn set_vis(&mut self, vis: Visibility) -> &mut Self {
            self.node.vis = vis;
            self
        }
        pub fn set_meta(&mut self, meta: &'a [Meta]) -> &mut Self {
            self.node.meta = meta;
            self
        }
        pub fn set_kind(&mut self, kind: DefKind) -> &mut Self {
            self.kind = kind;
            self
        }
        pub fn set_source(&mut self, source: NodeSource<'a>) -> &mut Self {
            self.node.kind = Some(source);
            self
        }
        pub fn set_module(&mut self, module: Module) -> &mut Self {
            self.module = module;
            self
        }
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub enum DefKind {
    /// An unevaluated definition
    #[default]
    Undecided,
    /// An impl block
    Impl(DefID),
    /// A use tree, and its parent
    Use(DefID),
    /// A type, such as a `type`, `struct`, or `enum`
    Type(TypeKind),
    /// A value, such as a `const`, `static`, or `fn`
    Value(ValueKind),
}

/// A [ValueKind] represents an item in the Value Namespace
/// (a component of a [Project](crate::project::Project)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueKind {
    Const(DefID),
    Static(DefID),
    Local(DefID),
    Fn(DefID),
}
/// A [TypeKind] represents an item in the Type Namespace
/// (a component of a [Project](crate::project::Project)).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeKind {
    /// An alias for an already-defined type
    Alias(Option<DefID>),
    /// A primitive type, built-in to the compiler
    Intrinsic(Intrinsic),
    /// A user-defined aromatic data type
    Adt(Adt),
    /// A reference to an already-defined type: &T
    Ref(u16, DefID),
    /// A contiguous view of dynamically sized memory
    Slice(DefID),
    /// A contiguous view of statically sized memory
    Array(DefID, usize),
    /// A tuple of existing types
    Tuple(Vec<DefID>),
    /// A function which accepts multiple inputs and produces an output
    FnSig { args: DefID, rety: DefID },
    /// The unit type
    Empty,
    /// The never type
    Never,
    /// The Self type
    SelfTy,
    /// An untyped module
    Module,
}

/// A user-defined Aromatic Data Type
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Adt {
    /// A union-like enum type
    Enum(Vec<(Sym, Option<DefID>)>),
    /// A C-like enum
    CLikeEnum(Vec<(Sym, u128)>),
    /// An enum with no fields, which can never be constructed
    FieldlessEnum,

    /// A structural product type with named members
    Struct(Vec<(Sym, Visibility, DefID)>),
    /// A structural product type with unnamed members
    TupleStruct(Vec<(Visibility, DefID)>),
    /// A structural product type of neither named nor unnamed members
    UnitStruct,

    /// A choose your own undefined behavior type
    /// TODO: should unions be a language feature?
    Union(Vec<(Sym, DefID)>),
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
