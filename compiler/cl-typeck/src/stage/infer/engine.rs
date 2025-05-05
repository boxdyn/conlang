use super::error::InferenceError;
use crate::{
    entry::Entry,
    handle::Handle,
    stage::infer::inference::Inference,
    table::{NodeKind, Table},
    type_expression::TypeExpression,
    type_kind::{Adt, Primitive, TypeKind},
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
    pub(super) table: &'table mut Table<'a>,
    /// The current working node
    pub(crate) at: Handle,
    /// The current breakset
    pub(crate) bset: Handle,
    /// The current returnset
    pub(crate) rset: Handle,
}

impl<'table, 'a> InferenceEngine<'table, 'a> {
    /// Infers the type of an object by deferring to [`Inference::infer()`]
    pub fn infer(&mut self, inferrable: &'a impl Inference<'a>) -> Result<Handle, InferenceError> {
        inferrable.infer(self)
    }

    /// Constructs a new [`InferenceEngine`], scoped around a [`Handle`] in a [`Table`].
    pub fn new(table: &'table mut Table<'a>, at: Handle) -> Self {
        let never = table.anon_type(TypeKind::Never);
        Self { at, table, bset: never, rset: never }
    }

    /// Constructs an [`InferenceEngine`] that borrows the same table as `self`,
    /// but with a shortened lifetime.
    pub fn scoped(&mut self) -> InferenceEngine<'_, 'a> {
        InferenceEngine { at: self.at, table: self.table, bset: self.bset, rset: self.rset }
    }

    pub fn infer_all(&mut self) -> Vec<(Handle, InferenceError)> {
        let iter = self.table.handle_iter();
        let mut res = Vec::new();
        for handle in iter {
            let mut eng = self.at(handle);
            // TODO: use sources instead of bodies, and infer the type globally
            let Some(body) = eng.table.body(handle) else {
                continue;
            };
            eprintln!("Evaluating body {body}");
            match body.infer(&mut eng) {
                Ok(ty) => println!("=> {}", eng.table.entry(ty)),
                Err(e) => {
                    match &e {
                        &InferenceError::Mismatch(a, b) => {
                            eprintln!(
                                "=> Mismatched types: {}, {}",
                                eng.table.entry(a),
                                eng.table.entry(b)
                            );
                        }
                        &InferenceError::Recursive(a, b) => {
                            eprintln!(
                                "=> Recursive types: {}, {}",
                                eng.table.entry(a),
                                eng.table.entry(b)
                            );
                        }
                        e => eprintln!("=> {e}"),
                    }
                    res.push((handle, e))
                }
            }
        }
        res
    }

    /// Constructs a new InferenceEngine with the
    pub fn at(&mut self, at: Handle) -> InferenceEngine<'_, 'a> {
        InferenceEngine { at, ..self.scoped() }
    }

    pub fn open_bset(&mut self) -> InferenceEngine<'_, 'a> {
        InferenceEngine { bset: self.new_var(), ..self.scoped() }
    }

    pub fn open_rset(&mut self) -> InferenceEngine<'_, 'a> {
        InferenceEngine { rset: self.new_var(), ..self.scoped() }
    }

    /// Constructs an [Entry] out of a [Handle], for ease of use
    pub fn entry(&self, of: Handle) -> Entry<'_, 'a> {
        self.table.entry(of)
    }

    #[deprecated = "Use dedicated methods instead."]
    pub fn from_type_kind(&mut self, kind: TypeKind) -> Handle {
        // TODO: preserve type heirarchy (for, i.e., reference types)
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

    /// Gets the defining usage of a type without collapsing intermediates
    pub fn def_usage(&self, to: Handle) -> Handle {
        match self.table.entry(to).ty() {
            Some(TypeKind::Instance(id)) => self.def_usage(*id),
            _ => to,
        }
    }

    pub fn get_fn(&self, at: Handle, name: Sym) -> Option<(Handle, Handle)> {
        use cl_ast::PathPart;
        if let Some(&TypeKind::FnSig { args, rety }) = self
            .entry(at)
            .nav(&[PathPart::Ident(name)])
            .as_ref()
            .and_then(Entry::ty)
        {
            Some((args, rety))
        } else {
            None
        }
    }

    /// Creates a new type variable representing a tuple
    pub fn new_tuple(&mut self, tys: Vec<Handle>) -> Handle {
        self.table.anon_type(TypeKind::Tuple(tys))
    }

    /// Creates a new type variable representing an array
    pub fn new_array(&mut self, ty: Handle, size: usize) -> Handle {
        self.table.anon_type(TypeKind::Array(ty, size))
    }

    /// Creates a new type variable representing a slice of contiguous memory
    pub fn new_slice(&mut self, ty: Handle) -> Handle {
        self.table.anon_type(TypeKind::Slice(ty))
    }

    /// Creates a new reference to a type
    pub fn new_ref(&mut self, to: Handle) -> Handle {
        self.table.anon_type(TypeKind::Ref(to))
    }

    /// All primitives must be predefined in the standard library.
    pub fn primitive(&self, name: Sym) -> Option<Handle> {
        // TODO: keep a map of primitives in the table root
        self.table.get_by_sym(self.table.root(), &name)
    }

    pub fn never(&mut self) -> Handle {
        self.table.anon_type(TypeKind::Never)
    }

    pub fn empty(&mut self) -> Handle {
        self.table.anon_type(TypeKind::Empty)
    }

    pub fn bool(&self) -> Handle {
        self.primitive("bool".into())
            .expect("There should be a type named bool.")
    }

    pub fn char(&self) -> Handle {
        self.primitive("char".into())
            .expect("There should be a type named char.")
    }

    pub fn str(&self) -> Handle {
        self.primitive("str".into())
            .expect("There should be a type named str.")
    }

    pub fn u32(&self) -> Handle {
        self.primitive("u32".into())
            .expect("There should be a type named u32.")
    }

    pub fn usize(&self) -> Handle {
        self.primitive("usize".into())
            .expect("There should be a type named usize.")
    }

    /// Creates a new inferred-integer literal
    pub fn integer_literal(&mut self) -> Handle {
        let h = self.table.new_entry(self.at, NodeKind::Local);
        self.table
            .set_ty(h, TypeKind::Primitive(Primitive::Integer));
        h
    }

    /// Creates a new inferred-float literal
    pub fn float_literal(&mut self) -> Handle {
        let h = self.table.new_entry(self.at, NodeKind::Local);
        self.table.set_ty(h, TypeKind::Primitive(Primitive::Float));
        h
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
            Some(TypeKind::Inferred) => {
                if let Some(ty) = self.table.ty(of) {
                    self.table.set_ty(to, ty.clone());
                }
                None
            }
            Some(TypeKind::Variable)
            | Some(TypeKind::Primitive(Primitive::Float | Primitive::Integer)) => {
                e.set_ty(TypeKind::Instance(of))
            }
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
            TypeKind::Inferred => false,
            TypeKind::Variable => true,
            &TypeKind::Array(h, _) => self.is_generic(h),
            &TypeKind::Instance(h) => self.is_generic(h),
            TypeKind::Primitive(_) => false,
            TypeKind::Adt(Adt::Enum(tys)) => tys.iter().any(|(_, ty)| self.is_generic(*ty)),
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
                    .map(|(name, ty)| (name, self.deep_clone(ty)))
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
            TypeKind::Adt(Adt::Enum(items)) => {
                items.iter().any(|(_, other)| self.occurs_in(this, *other))
            }
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
            TypeKind::Inferred
            | TypeKind::Variable
            | TypeKind::Adt(Adt::UnitStruct)
            | TypeKind::Primitive(_)
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
            (TypeKind::Inferred, _) => {
                self.set_instance(ah, bh);
                Ok(())
            }
            (_, TypeKind::Inferred) => self.unify(bh, ah),

            (TypeKind::Variable, _) => {
                self.set_instance(ah, bh);
                Ok(())
            }
            (TypeKind::Instance(a), TypeKind::Instance(b)) if !self.occurs_in(*a, *b) => {
                self.set_instance(*a, *b);
                Ok(())
            }
            (TypeKind::Instance(_), _) => Err(InferenceError::Recursive(ah, bh)),

            (TypeKind::Primitive(Primitive::Float), TypeKind::Primitive(Primitive::Integer))
            | (TypeKind::Primitive(Primitive::Integer), TypeKind::Primitive(Primitive::Float)) => {
                Err(InferenceError::Mismatch(ah, bh))
            }

            // Primitives have their own set of vars which only unify with primitives.
            (TypeKind::Primitive(Primitive::Integer), TypeKind::Primitive(i)) if i.is_integer() => {
                self.set_instance(ah, bh);
                Ok(())
            }
            (TypeKind::Primitive(Primitive::Float), TypeKind::Primitive(f)) if f.is_float() => {
                self.set_instance(ah, bh);
                Ok(())
            }

            (_, TypeKind::Variable)
            | (_, TypeKind::Instance(_))
            | (TypeKind::Primitive(_), TypeKind::Primitive(Primitive::Integer))
            | (TypeKind::Primitive(_), TypeKind::Primitive(Primitive::Float)) => self.unify(bh, ah),
            (TypeKind::Adt(Adt::Enum(ia)), TypeKind::Adt(Adt::Enum(ib)))
                if ia.len() == ib.len() =>
            {
                for ((na, a), (nb, b)) in ia.clone().into_iter().zip(ib.clone().into_iter()) {
                    if na != nb {
                        return Err(InferenceError::Mismatch(ah, bh));
                    }
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
            // Slice unifies with array
            (TypeKind::Array(a, _), TypeKind::Slice(b)) => self.unify(*a, *b),
            (TypeKind::Slice(_), TypeKind::Array(_, _)) => self.unify(bh, ah),
            (TypeKind::Array(a, sa), TypeKind::Array(b, sb)) if sa == sb => self.unify(*a, *b),
            (TypeKind::Tuple(a), TypeKind::Tuple(b)) => {
                if a.len() != b.len() {
                    return Err(InferenceError::Mismatch(ah, bh));
                }
                let (a, b) = (a.clone(), b.clone());
                for (a, b) in a.iter().zip(b.iter()) {
                    self.unify(*a, *b)?;
                }
                Ok(())
            }
            (&TypeKind::FnSig { args: a1, rety: r1 }, &TypeKind::FnSig { args: a2, rety: r2 }) => {
                self.unify(a1, a2)?;
                self.unify(r1, r2)
            }
            (TypeKind::Empty, TypeKind::Tuple(t)) | (TypeKind::Tuple(t), TypeKind::Empty)
                if t.is_empty() =>
            {
                Ok(())
            }
            (TypeKind::Never, _) | (_, TypeKind::Never) => Ok(()),
            (a, b) if a == b => Ok(()),
            _ => Err(InferenceError::Mismatch(ah, bh)),
        }
    }
}
