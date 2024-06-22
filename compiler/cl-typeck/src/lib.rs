//! # The Conlang Type Checker
//!
//! As a statically typed language, Conlang requires a robust type checker to enforce correctness.
//!
//! This crate is a major work-in-progress.
//!
//! # The [Project](project::Project)™
//! Contains [item definitions](definition) and type expression information.
//!
//! *Every* definition is itself a module, and can contain arbitrarily nested items
//! as part of the [Module](module::Module) tree.
//!
//! The Project keeps track of a *global intern pool* of definitions, which are
//! trivially comparable by [DefID](key::DefID). Note that, for item definitions,
//! identical types in different modules DO NOT COMPARE EQUAL under this constraint.
//! However, so-called "anonymous" types *do not* follow this rule, as their
//! definitions are constructed dynamically and ensured to be unique.
// Note: it's a class invariant that named types are not added
// to the anon-types list. Please keep it that way. ♥  Thanks!
//!
//! # Namespaces
//! Within a Project, [definitions](definition::Def) are classified into two namespaces:
//!
//! ## Type Namespace:
//! - Modules
//! - Structs
//! - Enums
//! - Type aliases
//!
//! ## Value Namespace:
//! - Functions
//! - Constants
//! - Static variables
//!
//! There is a *key* distinction between the two namespaces when it comes to
//! [Path](path::Path) traversal: Only items in the Type Namespace will be considered
//! as an intermediate path target. This means items in the Value namespace are
//! entirely *opaque*, and form a one-way barrier. Items outside a Value can be
//! referred to within the Value, but items inside a Value *cannot* be referred to
//! outside the Value.
//!
//! # Order of operations:
//! Currently, the process of type resolution goes as follows:
//!
//! 1. Traverse the AST, [collecting all Items into item definitions](name_collector)
//! 2. Eagerly [resolve `use` imports](use_importer)
//! 3. Traverse all [definitions](definition::Def), and [resolve the types for every
//!    item](type_resolver)
//! 4. TODO: Construct a typed AST for expressions, and type-check them
#![warn(clippy::all)]

/*
How do I flesh out modules in an incremental way?

1. Visit all *modules* and create nodes in a module tree for them
- This can be done by holding mutable references to submodules!


Module:
    values: Map(name -> Value),
    types: Map(name -> Type),

Value: Either
    function: { signature: Type, body: FunctionBody }
    static: { Mutability, ty: Type, init: ConstEvaluationResult }
    const: { ty: Type, init: ConstEvaluationResult }

*/

pub mod key;

pub mod node;

pub mod definition;

pub mod module;

pub mod path;

pub mod project;

pub mod name_collector;

pub mod use_importer;

pub mod type_resolver;

pub mod inference;

/*

LET THERE BE NOTES:

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
