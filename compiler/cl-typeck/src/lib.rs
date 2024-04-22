//! # The Conlang Type Checker
//!
//! As a statically typed language, Conlang requires a robust type checker to enforce correctness.
#![feature(debug_closure_helpers)]
#![warn(clippy::all)]

/*

The type checker keeps track of a *global intern pool* for Definitions
References to the intern pool are held by DefID, and items cannot be freed from the pool EVER.

Definitions are classified into two namespaces:

Type Namespace:
    - Modules
    - Structs
    - Enums
    - Type aliases

Value Namespace:
    - Functions
    - Constants
    - Static variables

*/

pub mod key;

pub mod definition;

pub mod module;

pub mod path;

pub mod project;

pub mod name_collector;

pub mod use_importer;

pub mod type_resolver;

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
