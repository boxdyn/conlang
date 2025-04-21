use super::error::InferenceError;
use crate::{
    entry::Entry,
    handle::Handle,
    table::{NodeKind, Table},
    type_expression::TypeExpression,
    type_kind::{Adt, TypeKind},
};
use cl_ast::Sym;

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

pub struct InferenceEngine<'table, 'a> {
    pub(crate) at: Handle,
    pub(super) table: &'table mut Table<'a>,
    pub(crate) bset: Handle,
    pub(crate) rset: Handle,
}

impl<'table, 'a> InferenceEngine<'table, 'a> {
    pub fn new(table: &'table mut Table<'a>, at: Handle) -> Self {
        let never = table.anon_type(TypeKind::Never);
        Self { at, table, bset: never, rset: never }
    }

    pub fn at(&mut self, at: Handle) -> InferenceEngine<'_, 'a> {
        InferenceEngine { at, table: self.table, bset: self.bset, rset: self.rset }
    }

    pub fn open_bset(&mut self) -> InferenceEngine<'_, 'a> {
        let bset = self.from_type_kind(TypeKind::Empty);
        InferenceEngine { at: self.at, table: self.table, bset, rset: self.rset }
    }

    pub fn open_rset(&mut self) -> InferenceEngine<'_, 'a> {
        let rset = self.new_var();
        InferenceEngine { at: self.at, table: self.table, bset: self.bset, rset }
    }

    pub fn entry(&self, of: Handle) -> Entry<'_, 'a> {
        self.table.entry(of)
    }

    pub fn from_type_kind(&mut self, kind: TypeKind) -> Handle {
        self.table.anon_type(kind)
    }

    pub fn by_name<Out, N: TypeExpression<Out>>(
        &mut self,
        name: &N,
    ) -> Result<Out, crate::type_expression::Error> {
        name.evaluate(self.table, self.at)
    }

    /// Creates a new unbound [type variable](Handle)
    pub fn new_var(&mut self) -> Handle {
        self.table.type_variable()
    }

    /// Creates a variable that is a new instance of another [Type](Handle)
    pub fn new_inst(&mut self, of: Handle) -> Handle {
        self.table.anon_type(TypeKind::Instance(of))
    }

    pub fn de_inst(&self, to: Handle) -> Handle {
        match self.table.entry(to).ty() {
            Some(TypeKind::Instance(id)) => self.de_inst(*id),
            _ => to,
        }
    }

    /// Creates a new type variable representing a function (signature)
    pub fn new_fn(&mut self, args: Handle, rety: Handle) -> Handle {
        self.table.anon_type(TypeKind::FnSig { args, rety })
    }

    /// Creates a new type variable representing a tuple
    pub fn new_tuple(&mut self, tys: Vec<Handle>) -> Handle {
        self.table.anon_type(TypeKind::Tuple(tys))
    }

    /// Creates a new type variable representing an array
    pub fn new_array(&mut self, ty: Handle, size: usize) -> Handle {
        self.table.anon_type(TypeKind::Array(ty, size))
    }

    /// All primitives must be predefined in the standard library.
    pub fn primitive(&self, name: Sym) -> Option<Handle> {
        self.table.get_by_sym(self.table.root(), &name)
    }

    /// Enters a new scope
    pub fn local_scope(&mut self) {
        let scope = self.table.new_entry(self.at, NodeKind::Local);
        self.at = scope;
    }

    /// Creates a new locally-scoped InferenceEngine.
    pub fn block_scope(&mut self) -> InferenceEngine<'_, 'a> {
        let scope = self.table.new_entry(self.at, NodeKind::Local);
        self.at(scope)
    }

    /// Sets this type variable `to` be an instance `of` the other
    /// # Panics
    /// Panics if `to` is not a type variable
    pub fn set_instance(&mut self, to: Handle, of: Handle) {
        let mut e = self.table.entry_mut(to);
        match e.as_ref().ty() {
            Some(TypeKind::Variable) => e.set_ty(TypeKind::Instance(of)),
            other => todo!("Cannot set {} to instance of: {other:?}", e.as_ref()),
        };
    }

    /// Checks whether there are any unbound type variables in this type
    pub fn is_generic(&self, ty: Handle) -> bool {
        let entry = self.table.entry(ty);
        let Some(ty) = entry.ty() else {
            return false;
        };
        match ty {
            TypeKind::Uninferred => false,
            TypeKind::Variable => true,
            &TypeKind::Array(h, _) => self.is_generic(h),
            &TypeKind::Instance(h) => self.is_generic(h),
            TypeKind::Intrinsic(_) => false,
            TypeKind::Adt(Adt::Enum(tys)) => tys
                .iter()
                .any(|(_, ty)| ty.is_some_and(|ty| self.is_generic(ty))),
            TypeKind::Adt(Adt::Struct(tys)) => tys.iter().any(|&(_, _, ty)| self.is_generic(ty)),
            TypeKind::Adt(Adt::TupleStruct(tys)) => tys.iter().any(|&(_, ty)| self.is_generic(ty)),
            TypeKind::Adt(Adt::UnitStruct) => false,
            TypeKind::Adt(Adt::Union(tys)) => tys.iter().any(|&(_, ty)| self.is_generic(ty)),
            &TypeKind::Ref(h) => self.is_generic(h),
            &TypeKind::Slice(h) => self.is_generic(h),
            TypeKind::Tuple(handles) => handles.iter().any(|&ty| self.is_generic(ty)),
            &TypeKind::FnSig { args, rety } => self.is_generic(args) || self.is_generic(rety),
            TypeKind::Empty | TypeKind::Never | TypeKind::Module => false,
        }
    }

    /// Makes a deep copy of a type expression.
    ///
    /// Bound variables are shared, unbound variables are duplicated.
    pub fn deep_clone(&mut self, ty: Handle) -> Handle {
        if !self.is_generic(ty) {
            return ty;
        };
        let entry = self.table.entry(ty);
        let Some(ty) = entry.ty().cloned() else {
            return ty;
        };
        match ty {
            TypeKind::Variable => self.new_var(),
            TypeKind::Array(h, s) => {
                let ty = self.deep_clone(h);
                self.table.anon_type(TypeKind::Array(ty, s))
            }
            TypeKind::Instance(h) => {
                let ty = self.deep_clone(h);
                self.table.anon_type(TypeKind::Instance(ty))
            }
            TypeKind::Adt(Adt::Enum(tys)) => {
                let tys = tys
                    .into_iter()
                    .map(|(name, ty)| (name, ty.map(|ty| self.deep_clone(ty))))
                    .collect();
                self.table.anon_type(TypeKind::Adt(Adt::Enum(tys)))
            }
            TypeKind::Adt(Adt::Struct(tys)) => {
                let tys = tys
                    .into_iter()
                    .map(|(n, v, ty)| (n, v, self.deep_clone(ty)))
                    .collect();
                self.table.anon_type(TypeKind::Adt(Adt::Struct(tys)))
            }
            TypeKind::Adt(Adt::TupleStruct(tys)) => {
                let tys = tys
                    .into_iter()
                    .map(|(v, ty)| (v, self.deep_clone(ty)))
                    .collect();
                self.table.anon_type(TypeKind::Adt(Adt::TupleStruct(tys)))
            }
            TypeKind::Adt(Adt::Union(tys)) => {
                let tys = tys
                    .into_iter()
                    .map(|(n, ty)| (n, self.deep_clone(ty)))
                    .collect();
                self.table.anon_type(TypeKind::Adt(Adt::Union(tys)))
            }
            TypeKind::Ref(h) => {
                let ty = self.deep_clone(h);
                self.table.anon_type(TypeKind::Ref(ty))
            }
            TypeKind::Slice(h) => {
                let ty = self.deep_clone(h);
                self.table.anon_type(TypeKind::Slice(ty))
            }
            TypeKind::Tuple(tys) => {
                let tys = tys.into_iter().map(|ty| self.deep_clone(ty)).collect();
                self.table.anon_type(TypeKind::Tuple(tys))
            }
            TypeKind::FnSig { args, rety } => {
                let args = self.deep_clone(args);
                let rety = self.deep_clone(rety);
                self.table.anon_type(TypeKind::FnSig { args, rety })
            }
            _ => self.table.anon_type(ty),
        }
    }

    /// Returns the defining instance of `self`,
    /// collapsing type instances along the way.
    pub fn prune(&mut self, ty: Handle) -> Handle {
        if let Some(TypeKind::Instance(new_ty)) = self.table.ty(ty) {
            let new_ty = self.prune(*new_ty);
            self.table.set_ty(ty, TypeKind::Instance(new_ty));
            new_ty
        } else {
            ty
        }
    }

    /// Checks whether a type occurs in another type
    ///
    /// # Note:
    /// - Since the test uses strict equality, `self` should be pruned prior to testing.
    /// - The test is *not guaranteed to terminate* for recursive types.
    pub fn occurs_in(&self, this: Handle, other: Handle) -> bool {
        if this == other {
            return true;
        }
        let Some(ty) = self.table.ty(other) else {
            return false;
        };
        match ty {
            TypeKind::Instance(other) => self.occurs_in(this, *other),
            TypeKind::Adt(Adt::Enum(items)) => items
                .iter()
                .any(|(_, i)| i.is_some_and(|other| self.occurs_in(this, other))),
            TypeKind::Adt(Adt::Struct(items)) => items
                .iter()
                .any(|(_, _, other)| self.occurs_in(this, *other)),
            TypeKind::Adt(Adt::TupleStruct(items)) => {
                items.iter().any(|(_, other)| self.occurs_in(this, *other))
            }
            TypeKind::Adt(Adt::Union(items)) => {
                items.iter().any(|(_, other)| self.occurs_in(this, *other))
            }
            TypeKind::Ref(other) => self.occurs_in(this, *other),
            TypeKind::Slice(other) => self.occurs_in(this, *other),
            TypeKind::Array(other, _) => self.occurs_in(this, *other),
            TypeKind::Tuple(handles) => handles.iter().any(|&other| self.occurs_in(this, other)),
            TypeKind::FnSig { args, rety } => {
                self.occurs_in(this, *args) || self.occurs_in(this, *rety)
            }
            TypeKind::Uninferred
            | TypeKind::Variable
            | TypeKind::Adt(Adt::UnitStruct)
            | TypeKind::Intrinsic(_)
            | TypeKind::Empty
            | TypeKind::Never
            | TypeKind::Module => false,
        }
    }

    /// Unifies two types
    pub fn unify(&mut self, this: Handle, other: Handle) -> Result<(), InferenceError> {
        let (ah, bh) = (self.prune(this), self.prune(other));
        let (a, b) = (self.table.entry(ah), self.table.entry(bh));
        let (Some(a), Some(b)) = (a.ty(), b.ty()) else {
            return Err(InferenceError::Mismatch(ah, bh));
        };

        match (a, b) {
            (TypeKind::Uninferred, _) => {
                self.set_instance(ah, bh);
                Ok(())
            }
            (_, TypeKind::Uninferred) => self.unify(bh, ah),

            (TypeKind::Variable, _) => {
                self.set_instance(ah, bh);
                Ok(())
            }
            (TypeKind::Instance(a), TypeKind::Instance(b)) if !self.occurs_in(*a, *b) => {
                self.set_instance(*a, *b);
                Ok(())
            }
            (TypeKind::Instance(_), _) => Err(InferenceError::Recursive(ah, bh)),
            (_, TypeKind::Variable) | (_, TypeKind::Instance(_)) => self.unify(bh, ah),

            (TypeKind::Intrinsic(ia), TypeKind::Intrinsic(ib)) if ia == ib => Ok(()),
            (TypeKind::Adt(Adt::Enum(ia)), TypeKind::Adt(Adt::Enum(ib)))
                if ia.len() == ib.len() =>
            {
                for ((na, a), (nb, b)) in ia.clone().into_iter().zip(ib.clone().into_iter()) {
                    if na != nb || a.is_some() != b.is_some() {
                        return Err(InferenceError::Mismatch(ah, bh));
                    }
                    let (Some(a), Some(b)) = (a, b) else { continue };
                    self.unify(a, b)?;
                }
                Ok(())
            }
            (TypeKind::Adt(Adt::Struct(ia)), TypeKind::Adt(Adt::Struct(ib)))
                if ia.len() == ib.len() =>
            {
                for ((na, va, a), (nb, vb, b)) in ia.clone().into_iter().zip(ib.clone().into_iter())
                {
                    if na != nb || va != vb {
                        return Err(InferenceError::Mismatch(ah, bh));
                    }
                    self.unify(a, b)?;
                }
                Ok(())
            }
            (TypeKind::Adt(Adt::TupleStruct(ia)), TypeKind::Adt(Adt::TupleStruct(ib)))
                if ia.len() == ib.len() =>
            {
                for ((va, a), (vb, b)) in ia.clone().into_iter().zip(ib.clone().into_iter()) {
                    if va != vb {
                        return Err(InferenceError::Mismatch(ah, bh));
                    }
                    self.unify(a, b)?;
                }
                Ok(())
            }
            (TypeKind::Adt(Adt::Union(ia)), TypeKind::Adt(Adt::Union(ib)))
                if ia.len() == ib.len() =>
            {
                todo!()
            }
            (TypeKind::Ref(a), TypeKind::Ref(b)) => self.unify(*a, *b),
            (TypeKind::Slice(a), TypeKind::Slice(b)) => self.unify(*a, *b),
            (TypeKind::Array(a, sa), TypeKind::Array(b, sb)) if sa == sb => self.unify(*a, *b),
            (TypeKind::Tuple(a), TypeKind::Tuple(b)) => {
                if a.len() != b.len() {
                    return Err(InferenceError::Mismatch(ah, bh));
                }
                Ok(())
            }
            (&TypeKind::FnSig { args: a1, rety: r1 }, &TypeKind::FnSig { args: a2, rety: r2 }) => {
                self.unify(a1, a2)?;
                self.unify(r1, r2)
            }
            (TypeKind::Empty, TypeKind::Empty) => Ok(()),
            (TypeKind::Never, _) | (_, TypeKind::Never) => Ok(()),
            (TypeKind::Module, TypeKind::Module) => Ok(()),
            _ => Err(InferenceError::Mismatch(ah, bh)),
        }
    }
}
