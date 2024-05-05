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
//! 3. Traverse all [definitions](definition::Def),
//! and [resolve the types for every item](type_resolver)
//! 4. TODO: Construct a typed AST for expressions, and type-check them
#![warn(clippy::all)]

pub mod key;

pub mod node;

pub mod definition;

pub mod module;

pub mod path;

pub mod project;

pub mod name_collector;

pub mod use_importer;

pub mod type_resolver;

pub mod inference {
    //! Implements type unification, used by the Hindley-Milner type inference algorithm
    //!
    //! Inspired by [rust-hindley-milner][1] and [hindley-milner-python][2]
    //!
    //! [1]: https://github.com/tcr/rust-hindley-milner/
    //! [2]: https://github.com/rob-smallshire/hindley-milner-python

    use cl_ast::Sym;
    use core::fmt;
    use std::{cell::RefCell, rc::Rc};

    /*
        Types in Conlang:
        - Never type: !
          - type !
          - for<A> ! -> A
        - Primitive types: bool, i32, (), ...
          - type bool; ...
        - Reference types: &T, *T
          - for<T> type ref<T>; for<T> type ptr<T>
        - Slice type:      [T]
          - for<T> type slice<T>
        - Array type:      [T;usize]
          - for<T> type array<T, instanceof<usize>>
        - Tuple type:      (T, ...Z)
          - for<T, ..> type tuple<T, ..>    // on a per-case basis!
        - Funct type:      fn Tuple -> R
          - for<T, R> type T -> R           // on a per-case basis!
    */

    /// A refcounted [Type]
    pub type RcType = Rc<Type>;

    /// A [Type::Variable] or [Type::Operator]:
    /// - A [Type::Variable] can be either bound or unbound (instance: Some(_) | None)
    /// - A [Type::Operator] has a name (used to identify the operator) and a list of types.
    ///
    /// A type which contains unbound variables is considered "generic" (see
    /// [`Type::is_generic()`]).
    #[derive(Debug, PartialEq, Eq)]
    pub enum Type {
        Variable {
            instance: RefCell<Option<RcType>>,
        },
        Operator {
            name: Sym,
            types: RefCell<Vec<RcType>>,
        },
    }

    impl Type {
        /// Creates a new unbound [type variable](Type::Variable)
        pub fn new_var() -> RcType {
            Rc::new(Self::Variable { instance: RefCell::new(None) })
        }
        /// Creates a variable that is a new instance of another [Type]
        pub fn new_inst(of: &RcType) -> RcType {
            Rc::new(Self::Variable { instance: RefCell::new(Some(of.clone())) })
        }
        /// Creates a new [type operator](Type::Operator)
        pub fn new_op(name: Sym, types: &[RcType]) -> RcType {
            Rc::new(Self::Operator { name, types: RefCell::new(types.to_vec()) })
        }
        /// Creates a new [type operator](Type::Operator) representing a lambda
        pub fn new_fn(takes: &RcType, returns: &RcType) -> RcType {
            Self::new_op("fn".into(), &[takes.clone(), returns.clone()])
        }
        /// Creates a new [type operator](Type::Operator) representing a primitive type
        pub fn new_prim(name: Sym) -> RcType {
            Self::new_op(name, &[])
        }
        /// Creates a new [type operator](Type::Operator) representing a tuple
        pub fn new_tuple(members: &[RcType]) -> RcType {
            Self::new_op("tuple".into(), members)
        }

        /// Sets this type variable to be an instance `of` the other
        /// # Panics
        /// Panics if `self` is not a type variable
        pub fn set_instance(self: &RcType, of: &RcType) {
            match self.as_ref() {
                Type::Operator { .. } => unimplemented!("Cannot set instance of a type operator"),
                Type::Variable { instance } => *instance.borrow_mut() = Some(of.clone()),
            }
        }
        /// Checks whether there are any unbound type variables in this type.
        /// ```rust
        /// # use cl_typeck::inference::*;
        /// let bool = Type::new_op("bool".into(), &[]);
        /// let true_v = Type::new_inst(&bool);
        /// let unbound = Type::new_var();
        /// let id_fun = Type::new_fn(&unbound, &unbound);
        /// let truthy = Type::new_fn(&unbound, &bool);
        /// assert!(!bool.is_generic());   // bool contains no unbound type variables
        /// assert!(!true_v.is_generic()); // true_v is bound to `bool`
        /// assert!(unbound.is_generic()); // unbound is an unbound type variable
        /// assert!(id_fun.is_generic());  // id_fun is a function with unbound type variables
        /// assert!(truthy.is_generic());  // truthy is a function with one unbound type variable
        /// ```
        pub fn is_generic(self: &RcType) -> bool {
            match self.as_ref() {
                Type::Variable { instance } => match instance.borrow().as_ref() {
                    // base case: self is an unbound type variable (instance is none)
                    None => true,
                    // Variable is bound to a type which may be generic
                    Some(instance) => instance.is_generic(),
                },
                Type::Operator { types, .. } => {
                    // Operator may have generic args
                    types.borrow().iter().any(Self::is_generic)
                }
            }
        }
        /// Makes a deep copy of a type expression.
        ///
        /// Bound variables are shared, unbound variables are duplicated.
        pub fn deep_clone(self: &RcType) -> RcType {
            // If there aren't any unbound variables, it's fine to clone the entire expression
            if !self.is_generic() {
                return self.clone();
            }
            // There are unbound type variables, so we make a new one
            match self.as_ref() {
                Type::Variable { .. } => Self::new_var(),
                Type::Operator { name, types } => Self::new_op(
                    *name,
                    &types
                        .borrow()
                        .iter()
                        .map(Self::deep_clone)
                        .collect::<Vec<_>>(),
                ),
            }
        }
        /// Returns the defining instance of `self`,
        /// collapsing type instances along the way.
        /// # May panic
        /// Panics if this type variable's instance field is already borrowed.
        /// # Examples
        /// ```rust
        /// # use cl_typeck::inference::*;
        /// let t_bool = Type::new_op("bool".into(), &[]);
        /// let t_nest = Type::new_inst(&Type::new_inst(&Type::new_inst(&t_bool)));
        /// let pruned = t_nest.prune();
        /// assert_eq!(pruned, t_bool);
        /// assert_eq!(t_nest, Type::new_inst(&t_bool));
        /// ```
        pub fn prune(self: &RcType) -> RcType {
            if let Type::Variable { instance } = self.as_ref() {
                if let Some(old_inst) = instance.borrow_mut().as_mut() {
                    let new_inst = old_inst.prune(); // get defining instance
                    *old_inst = new_inst.clone(); // collapse
                    return new_inst;
                }
            }
            self.clone()
        }

        /// Checks whether a type expression occurs in another type expression
        ///
        /// # Note:
        /// - Since the test uses strict equality, `self` should be pruned prior to testing.
        /// - The test is *not guaranteed to terminate* for recursive types.
        pub fn occurs_in(self: &RcType, other: &RcType) -> bool {
            if self == other {
                return true;
            }
            match other.as_ref() {
                Type::Variable { instance } => match instance.borrow().as_ref() {
                    Some(t) => self.occurs_in(t),
                    None => false,
                },
                Type::Operator { types, .. } => {
                    // Note: this might panic.
                    // Think about whether it panics for only recursive types?
                    types.borrow().iter().any(|other| self.occurs_in(other))
                }
            }
        }

        /// Unifies two type expressions, propagating changes via interior mutability
        pub fn unify(self: &RcType, other: &RcType) -> Result<(), InferenceError> {
            let (a, b) = (self.prune(), other.prune()); // trim the hedges
            match (a.as_ref(), b.as_ref()) {
                (Type::Variable { .. }, _) if !a.occurs_in(&b) => a.set_instance(&b),
                (Type::Variable { .. }, _) => Err(InferenceError::Recursive(a, b))?,
                (Type::Operator { .. }, Type::Variable { .. }) => b.unify(&a)?,
                (
                    Type::Operator { name: a_name, types: a_types },
                    Type::Operator { name: b_name, types: b_types },
                ) => {
                    let (a_types, b_types) = (a_types.borrow(), b_types.borrow());
                    if a_name != b_name || a_types.len() != b_types.len() {
                        Err(InferenceError::Mismatch(a.clone(), b.clone()))?
                    }
                    for (a, b) in a_types.iter().zip(b_types.iter()) {
                        a.unify(b)?
                    }
                }
            }
            Ok(())
        }
    }

    impl fmt::Display for Type {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Type::Variable { instance } => match instance.borrow().as_ref() {
                    Some(instance) => write!(f, "{instance}"),
                    None => write!(f, "_"),
                },
                Type::Operator { name, types } => {
                    write!(f, "({name}")?;
                    for ty in types.borrow().iter() {
                        write!(f, " {ty}")?;
                    }
                    f.write_str(")")
                }
            }
        }
    }

    /// An error produced during type inference
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum InferenceError {
        Mismatch(RcType, RcType),
        Recursive(RcType, RcType),
    }

    impl fmt::Display for InferenceError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                InferenceError::Mismatch(a, b) => write!(f, "Type mismatch: {a:?} != {b:?}"),
                InferenceError::Recursive(_, _) => write!(f, "Recursive type!"),
            }
        }
    }
}

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
