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

#[derive(Debug, PartialEq, Eq)]
pub struct Variable {
    pub instance: RefCell<Option<RcType>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Operator {
    name: Sym,
    types: RefCell<Vec<RcType>>,
}

/// A [Type::Variable] or [Type::Operator]:
/// - A [Type::Variable] can be either bound or unbound (instance: Some(_) | None)
/// - A [Type::Operator] has a name (used to identify the operator) and a list of types.
///
/// A type which contains unbound variables is considered "generic" (see
/// [`Type::is_generic()`]).
#[derive(Debug, PartialEq, Eq)]
pub enum Type {
    Variable(Variable),
    Operator(Operator),
}

impl Type {
    /// Creates a new unbound [type variable](Type::Variable)
    pub fn new_var() -> RcType {
        Rc::new(Self::Variable(Variable { instance: RefCell::new(None) }))
    }
    /// Creates a variable that is a new instance of another [Type]
    pub fn new_inst(of: &RcType) -> RcType {
        Rc::new(Self::Variable(Variable {
            instance: RefCell::new(Some(of.clone())),
        }))
    }
    /// Creates a new [type operator](Type::Operator)
    pub fn new_op(name: Sym, types: &[RcType]) -> RcType {
        Rc::new(Self::Operator(Operator {
            name,
            types: RefCell::new(types.to_vec()),
        }))
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
            Type::Operator(_) => unimplemented!("Cannot set instance of a type operator"),
            Type::Variable(Variable { instance }) => *instance.borrow_mut() = Some(of.clone()),
        }
    }
    /// Checks whether there are any unbound type variables in this type.
    /// ```rust
    /// # use cl_typeck::stage::infer::*;
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
            Type::Variable(Variable { instance }) => match instance.borrow().as_ref() {
                // base case: self is an unbound type variable (instance is none)
                None => true,
                // Variable is bound to a type which may be generic
                Some(instance) => instance.is_generic(),
            },
            Type::Operator(Operator { types, .. }) => {
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
            Type::Operator(Operator { name, types }) => Self::new_op(
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
    /// # use cl_typeck::stage::infer::*;
    /// let t_bool = Type::new_op("bool".into(), &[]);
    /// let t_nest = Type::new_inst(&Type::new_inst(&Type::new_inst(&t_bool)));
    /// let pruned = t_nest.prune();
    /// assert_eq!(pruned, t_bool);
    /// assert_eq!(t_nest, Type::new_inst(&t_bool));
    /// ```
    pub fn prune(self: &RcType) -> RcType {
        if let Type::Variable(Variable { instance }) = self.as_ref() {
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
            Type::Variable(Variable { instance }) => match instance.borrow().as_ref() {
                Some(t) => self.occurs_in(t),
                None => false,
            },
            Type::Operator(Operator { types, .. }) => {
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
                Type::Operator(Operator { name: a_name, types: a_types }),
                Type::Operator(Operator { name: b_name, types: b_types }),
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
            Type::Variable(Variable { instance }) => match instance.borrow().as_ref() {
                Some(instance) => write!(f, "{instance}"),
                None => write!(f, "_"),
            },
            Type::Operator(Operator { name, types }) => {
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
