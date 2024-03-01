//! # The Conlang Type Checker
//!
//! As a statically typed language, Conlang requires a robust type checker to enforce correctness.

#![warn(clippy::all)]
#![allow(unused)]
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use cl_ast::*;

pub mod intern {
    //! Trivially-copyable, easily comparable typed indices for type system constructs

    /// Creates newtype indices over [`usize`] for use elsewhere in the type checker
    macro_rules! def_id {($($(#[$meta:meta])* $name:ident),*$(,)?) => {$(
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(usize);

        impl $name {
            #[doc = concat!("Constructs a [`", stringify!($name), "`] from a [`usize`] without checking bounds.")]
            /// # Safety
            /// The provided value should be within the bounds of its associated container
            pub unsafe fn from_raw_unchecked(value: usize) -> Self {
                Self(value)
            }
            /// Gets the index of the type by-value
            pub fn value(&self) -> usize {
                self.0
            }
        }

        impl From< $name > for usize {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    )*}}

    // define the index types
    def_id! {
        /// Uniquely represents a Type
        TypeID,
        /// Uniquely represents a Value
        ValueID,
    }
}

pub mod typedef {
    //! Representations of type definitions
    // use std::collections::HashMap;

    use crate::intern::TypeID;
    use cl_ast::{Item, Visibility};

    /// The definition of a type
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct TypeDef {
        name: String,
        kind: Option<TypeKind>,
        definition: Item,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum TypeKind {
        /// A primitive type, built-in to the compiler
        Intrinsic,
        /// A user-defined structural product type
        Struct(Vec<(String, Visibility, TypeID)>),
        /// A user-defined union-like enum type
        Enum(Vec<(String, TypeID)>),
        /// A type alias
        Alias(TypeID),
        /// The unit type
        Empty,
        /// The Self type
        SelfTy,
        // TODO: other types
    }
}

pub mod valdef {
    //! Representations of value definitions

    use crate::intern::{TypeID, ValueID};
    use cl_ast::Block;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ValueDef {
        name: String,
        kind: Option<ValueKind>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum ValueKind {
        Const(),
        Static(),
        Fn {
            args: Vec<TypeID>,
            rety: TypeID,
            body: Block,
        },
    }
}

pub mod typeinfo {
    //! Stores typeck-time type inference info

    use crate::intern::TypeID;

    /// The Type struct represents all valid types, and can be trivially equality-compared
    pub struct Type {
        /// You can only have a pointer chain 65535 pointers long.
        ref_depth: u16,
        /// Types can be [Generic](TKind::Generic) or [Concrete](TKind::Concrete)
        kind: TKind,
    }

    /// Types can be [Generic](TKind::Generic) or [Concrete](TKind::Concrete)
    pub enum TKind {
        /// A Concrete type has an associated [TypeDef](super::typedef::TypeDef)
        Concrete(TypeID),
        /// A Generic type is a *locally unique* comparable value,
        /// valid only until the end of its inference context
        Generic(usize),
    }
}

pub mod type_context {
    //! A type context stores a map from names to TypeIDs

    use std::collections::HashMap;

    use crate::intern::TypeID;

    pub struct TypeCtx {
        parent: Option<Box<TypeCtx>>,
        concrete: HashMap<String, TypeID>,
    }
}

/*
/// What is an inference rule?
/// An inference rule is a specification with a set of predicates and a judgement

/// Let's give every type an ID
struct TypeID(usize);

/// Let's give every type some data:

struct TypeDef<'def> {
    name: String,
    definition: &'def Item,
}

and store them in a big vector of type descriptions:

struct TypeMap<'def> {
    types: Vec<TypeDef<'def>>,
}
// todo: insertion of a type should yield a TypeID
// todo: impl index with TypeID

Let's store type information as either a concrete type or a generic type:

/// The Type struct represents all valid types, and can be trivially equality-compared
pub struct Type {
    /// You can only have a pointer chain 65535 pointers long.
    ref_depth: u16,
    kind: TKind,
}
pub enum TKind {
    Concrete(TypeID),
    Generic(usize),
}

And assume I can specify a rule based on its inputs and outputs:

Rule {
    operation: If,
    /// The inputs field is populated by
    inputs: [Concrete(BOOL), Generic(0), Generic(0)],
    outputs: Generic(0),
    /// This rule is compiler-intrinsic!
    through: None,
}

Rule {
    operation: Add,
    inputs: [Concrete(I32), Concrete(I32)],
    outputs: Concrete(I32),
    /// This rule is not compiler-intrinsic (it is overloaded!)
    through: Some(&ImplAddForI32::Add),
}



These rules can be stored in some kind of rule database:

let rules: Hashmap<Operation, Vec<Rule>> {

}

*/

/*
    Potential solution:
    Store reference to type field of each type expression in the AST
*/
