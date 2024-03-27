//! # The Conlang Type Checker
//!
//! As a statically typed language, Conlang requires a robust type checker to enforce correctness.

#![warn(clippy::all)]

/*

The type checker keeps track of a *global intern pool* for Types and Values
References to the intern pool are held by ID, and items cannot be freed from the pool EVER.

Items are inserted into their respective pools,


*/

pub mod key {
    use cl_structures::intern_pool::*;

    // define the index types
    make_intern_key! {
        /// Uniquely represents a TypeDef in the TypeDef [Pool]
        TypeID,
        /// Uniquely represents a ValueDef in the ValueDef [Pool]
        ValueID,
        /// Uniquely represents a Module in the Module [Pool]
        ModuleID,
    }
}

pub mod typedef {
    //! A TypeDef represents an item in the Type Namespace (a component of a
    //! [Project](crate::project::Project)).

    use crate::key::{TypeID, ValueID};
    use cl_ast::{Item, Visibility};

    /// A TypeDef represents an item in the Type Namespace (a component of a
    /// [Project](crate::project::Project)).
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct TypeDef {
        pub name: String,
        /// The expanded form of the definition, with all fields properly typed.
        /// This is filled in once all types are been enumerated.
        pub kind: Option<TypeKind>,
        pub definition: Item,
        pub associated_values: Vec<ValueID>,
        // TODO: Generic type parameters
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum TypeKind {
        /// A primitive type, built-in to the compiler
        Intrinsic(Intrinsic),
        /// A user-defined structural product type
        Struct(Vec<(String, Visibility, TypeID)>),
        /// A user-defined union-like enum type
        Enum(Vec<(String, TypeID)>),
        /// An alias for an already-defined type
        Alias(TypeID),
        /// The unit type
        Empty,
        /// The Self type
        SelfTy,
        // TODO: union types, tuples, tuple structs maybe?
    }

    /// The set of compiler-intrinsic types.
    /// These primitive types have native implementations of the basic operations.
    #[allow(non_camel_case_types)]
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Intrinsic {
        /// A 32-bit two's complement integer
        i32,
        /// A 32-bit unsigned integer
        u32,
        /// A boolean (`true` or `false`)
        bool,
    }
}

pub mod valdef {
    //! A [ValueDef] represents an item in the Value Namespace (a component of a
    //! [Project](crate::project::Project)).

    use crate::typeref::TypeRef;
    use cl_ast::{Block, Item};

    /// A [ValueDef] represents an item in the Value Namespace (a component of a
    /// [Project](crate::project::Project)).
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ValueDef {
        pub name: String,
        /// The expanded form of the definition, with all fields properly typed
        pub kind: Option<ValueKind>,
        pub definition: Item,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum ValueKind {
        Const(TypeRef),
        Static(TypeRef),
        Fn {
            args: Vec<TypeRef>,
            rety: TypeRef,
            body: Block,
        },
    }
}

pub mod module {
    //! A [Module] is a node in the Module Tree (a component of a
    //! [Project](crate::project::Project))
    use crate::key::{ModuleID, TypeID, ValueID};
    use std::collections::HashMap;

    /// A [Module] is a node in the Module Tree (a component of a
    /// [Project](crate::project::Project)).
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct Module {
        pub parent: Option<ModuleID>,
        pub types: HashMap<String, TypeID>,
        pub values: HashMap<String, ValueID>,
        pub submodules: HashMap<String, ModuleID>,
    }
}

pub mod project {
    use cl_structures::intern_pool::Pool;

    use crate::{
        key::{ModuleID, TypeID, ValueID},
        module::Module,
        typedef::TypeDef,
        valdef::ValueDef,
    };

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct Project {
        pub types: Pool<TypeDef, TypeID>,
        pub values: Pool<ValueDef, ValueID>,
        pub modules: Pool<Module, ModuleID>,
    }
}

pub mod typeref {
    //! Stores type and referencce info

    use crate::key::TypeID;

    /// The Type struct represents all valid types, and can be trivially equality-compared
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct TypeRef {
        /// You can only have a pointer chain 65535 pointers long.
        ref_depth: u16,
        /// Types can be [Generic](TKind::Generic) or [Concrete](TKind::Concrete)
        kind: RefKind,
    }

    /// Types can be [Generic](TKind::Generic) or [Concrete](TKind::Concrete)
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum RefKind {
        /// A Concrete type has an associated [TypeDef](super::typedef::TypeDef)
        Concrete(TypeID),
        /// A Generic type is a *locally unique* comparable value,
        /// valid only until the end of its typing context.
        /// This is usually the surrounding function.
        Generic(usize),
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
