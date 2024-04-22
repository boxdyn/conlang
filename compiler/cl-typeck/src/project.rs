//! A [Project] contains a tree of [Def]initions, referred to by their [Path]
use crate::{
    definition::{Def, DefKind, TypeKind},
    key::DefID,
    module,
    path::Path,
};
use cl_ast::{Identifier, PathPart, TyFn, TyKind, TyRef, TyTuple, Visibility};
use cl_structures::intern_pool::Pool;
use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

use self::evaluate::EvaluableTypeExpression;

#[derive(Clone, Debug)]
pub struct Project<'a> {
    pub pool: Pool<Def<'a>, DefID>,
    /// Stores anonymous tuples, function pointer types, etc.
    pub anon_types: HashMap<TypeKind<'a>, DefID>,
    pub root: DefID,
}

impl Project<'_> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Project<'_> {
    fn default() -> Self {
        let mut pool = Pool::default();
        let root = pool.insert(Def {
            name: "🌳 root 🌳",
            kind: DefKind::Type(TypeKind::Module),
            ..Default::default()
        });

        // Insert the Never(!) type
        let never = pool.insert(Def {
            name: "!",
            vis: Visibility::Public,
            kind: DefKind::Type(TypeKind::Never),
            module: module::Module::new(root),
            ..Default::default()
        });
        let empty = pool.insert(Def {
            name: "()",
            vis: Visibility::Public,
            kind: DefKind::Type(TypeKind::Empty),
            module: module::Module::new(root),
            ..Default::default()
        });
        // TODO: Self is not a real type!
        let selfty = pool.insert(Def {
            name: "Self",
            vis: Visibility::Public,
            kind: DefKind::Type(TypeKind::SelfTy),
            module: module::Module::new(root),
            ..Default::default()
        });

        let mut anon_types = HashMap::new();
        anon_types.insert(TypeKind::Empty, empty);
        anon_types.insert(TypeKind::Never, never);
        anon_types.insert(TypeKind::SelfTy, selfty);

        Self { pool, root, anon_types }
    }
}

impl<'a> Project<'a> {
    pub fn parent_of(&self, module: DefID) -> Option<DefID> {
        self[module].module.parent
    }
    pub fn root_of(&self, module: DefID) -> DefID {
        match self.parent_of(module) {
            Some(module) => self.root_of(module),
            None => module,
        }
    }

    pub fn get<'p>(
        &self,
        path: Path<'p>,
        within: DefID,
    ) -> Option<(Option<DefID>, Option<DefID>, Path<'p>)> {
        if path.absolute {
            return self.get(path.relative(), self.root_of(within));
        }
        match path.as_ref() {
            [] => Some((Some(within), None, path)),
            [PathPart::Ident(Identifier(name))] => {
                let (ty, val) = self[within].module.get(name);
                Some((ty, val, path.pop_front()?))
            }
            [PathPart::Ident(Identifier(name)), ..] => {
                let ty = self[within].module.get_type(name)?;
                self.get(path.pop_front()?, ty)
            }
            [PathPart::SelfKw, ..] => self.get(path.pop_front()?, within),
            [PathPart::SuperKw, ..] => self.get(path.pop_front()?, self.parent_of(within)?),
        }
    }

    /// Resolves a path within a module tree, finding the innermost module.
    /// Returns the remaining path parts.
    pub fn get_type<'p>(&self, path: Path<'p>, within: DefID) -> Option<(DefID, Path<'p>)> {
        if path.absolute {
            self.get_type(path.relative(), self.root_of(within))
        } else if let Some(front) = path.front() {
            let module = &self[within].module;
            match front {
                PathPart::SelfKw => self.get_type(path.pop_front()?, within),
                PathPart::SuperKw => self.get_type(path.pop_front()?, module.parent?),
                PathPart::Ident(Identifier(name)) => match module.types.get(name.as_str()) {
                    Some(&submodule) => self.get_type(path.pop_front()?, submodule),
                    None => Some((within, path)),
                },
            }
        } else {
            Some((within, path))
        }
    }

    pub fn get_value<'p>(&self, path: Path<'p>, within: DefID) -> Option<(DefID, Path<'p>)> {
        match path.front()? {
            PathPart::Ident(Identifier(name)) => Some((
                self[within].module.values.get(name.as_str()).copied()?,
                path.pop_front()?,
            )),
            _ => None,
        }
    }

    /// Inserts the type returned by the provided closure iff the TypeKind doesn't already exist
    ///
    /// Assumes `kind` uniquely identifies the type!
    pub fn insert_anonymous_type(
        &mut self,
        kind: TypeKind<'a>,
        def: impl FnOnce() -> Def<'a>,
    ) -> DefID {
        *(self
            .anon_types
            .entry(kind)
            .or_insert_with(|| self.pool.insert(def())))
    }

    pub fn evaluate<T>(&mut self, expr: &T, parent: DefID) -> Result<T::Out, String>
    where T: EvaluableTypeExpression {
        expr.evaluate(self, parent)
    }
}

impl<'a> Index<DefID> for Project<'a> {
    type Output = Def<'a>;
    fn index(&self, index: DefID) -> &Self::Output {
        &self.pool[index]
    }
}
impl IndexMut<DefID> for Project<'_> {
    fn index_mut(&mut self, index: DefID) -> &mut Self::Output {
        &mut self.pool[index]
    }
}

pub mod evaluate {
    //! An [EvaluableTypeExpression] is a component of a type expression tree
    //! or an intermediate result of expression evaluation.

    use super::*;
    use cl_ast::Ty;

    /// Things that can be evaluated as a type expression
    pub trait EvaluableTypeExpression {
        /// The result of type expression evaluation
        type Out;
        fn evaluate(&self, prj: &mut Project, parent: DefID) -> Result<Self::Out, String>;
    }

    impl EvaluableTypeExpression for Ty {
        type Out = DefID;
        fn evaluate(&self, prj: &mut Project, id: DefID) -> Result<DefID, String> {
            self.kind.evaluate(prj, id)
        }
    }

    impl EvaluableTypeExpression for TyKind {
        type Out = DefID;
        fn evaluate(&self, prj: &mut Project, parent: DefID) -> Result<DefID, String> {
            let id = match self {
                // TODO: reduce duplication here
                TyKind::Never => prj.anon_types[&TypeKind::Never],
                TyKind::Empty => prj.anon_types[&TypeKind::Empty],
                TyKind::SelfTy => prj.anon_types[&TypeKind::SelfTy],
                // TyKind::Path must be looked up explicitly
                TyKind::Path(path) => {
                    let (id, path) = prj
                        .get_type(path.into(), parent)
                        .ok_or("Failed to get type")?;
                    if path.is_empty() {
                        id
                    } else {
                        let (id, path) =
                            prj.get_value(path, id).ok_or("Failed to get value")?;
                        path.is_empty()
                            .then_some(id)
                            .ok_or("Path not fully resolved")?
                    }
                }
                TyKind::Tuple(tup) => tup.evaluate(prj, parent)?,
                TyKind::Ref(tyref) => tyref.evaluate(prj, parent)?,
                TyKind::Fn(tyfn) => tyfn.evaluate(prj, parent)?,
            };
            // println!("{self} => {id:?}");

            Ok(id)
        }
    }

    impl EvaluableTypeExpression for str {
        type Out = DefID;

        fn evaluate(&self, prj: &mut Project, parent: DefID) -> Result<Self::Out, String> {
            prj[parent]
                .module
                .types
                .get(self)
                .copied()
                .ok_or_else(|| format!("{self} is not a member of {}", prj[parent].name))
        }
    }

    impl EvaluableTypeExpression for TyTuple {
        type Out = DefID;
        fn evaluate(&self, prj: &mut Project, parent: DefID) -> Result<DefID, String> {
            let types = self.types.evaluate(prj, parent)?;
            let root = prj.root;
            let id = prj.insert_anonymous_type(TypeKind::Tuple(types.clone()), move || Def {
                kind: DefKind::Type(TypeKind::Tuple(types)),
                module: module::Module::new(root),
                ..Default::default()
            });

            Ok(id)
        }
    }
    impl EvaluableTypeExpression for TyRef {
        type Out = DefID;
        fn evaluate(&self, prj: &mut Project, parent: DefID) -> Result<DefID, String> {
            let TyRef { count, mutable: _, to } = self;
            let to = to.evaluate(prj, parent)?;

            let root = prj.root;
            let id = prj.insert_anonymous_type(TypeKind::Ref(*count, to), move || Def {
                kind: DefKind::Type(TypeKind::Ref(*count, to)),
                module: module::Module::new(root),
                ..Default::default()
            });
            Ok(id)
        }
    }
    impl EvaluableTypeExpression for TyFn {
        type Out = DefID;
        fn evaluate(&self, prj: &mut Project, parent: DefID) -> Result<DefID, String> {
            let TyFn { args, rety } = self;

            let args = args.evaluate(prj, parent)?;
            let rety = match rety {
                Some(rety) => rety.evaluate(prj, parent)?,
                _ => TyKind::Empty.evaluate(prj, parent)?,
            };

            let root = prj.root;
            let id = prj.insert_anonymous_type(TypeKind::FnSig { args, rety }, || Def {
                kind: DefKind::Type(TypeKind::FnSig { args, rety }),
                module: module::Module::new(root),
                ..Default::default()
            });
            Ok(id)
        }
    }

    impl EvaluableTypeExpression for cl_ast::Path {
        type Out = DefID;

        fn evaluate(&self, prj: &mut Project, parent: DefID) -> Result<Self::Out, String> {
            Path::from(self).evaluate(prj, parent)
        }
    }
    impl EvaluableTypeExpression for PathPart {
        type Out = DefID;
        fn evaluate(&self, prj: &mut Project, parent: DefID) -> Result<Self::Out, String> {
            match self {
                PathPart::SuperKw => prj
                    .parent_of(parent)
                    .ok_or_else(|| "Attempt to get super of root".into()),
                PathPart::SelfKw => Ok(parent),
                PathPart::Ident(Identifier(name)) => name.as_str().evaluate(prj, parent),
            }
        }
    }
    impl<'a> EvaluableTypeExpression for Path<'a> {
        type Out = DefID;
        fn evaluate(&self, prj: &mut Project, parent: DefID) -> Result<Self::Out, String> {
            let (id, path) = prj.get_type(*self, parent).ok_or("Failed to get type")?;

            if path.is_empty() {
                Ok(id)
            } else {
                let (id, path) = prj.get_value(path, id).ok_or("Failed to get value")?;
                path.is_empty()
                    .then_some(id)
                    .ok_or(String::from("Path not fully resolved"))
            }
        }
    }
    impl<T: EvaluableTypeExpression> EvaluableTypeExpression for [T] {
        type Out = Vec<T::Out>;

        fn evaluate(&self, prj: &mut Project, parent: DefID) -> Result<Self::Out, String> {
            let mut types = vec![];
            for value in self {
                types.push(value.evaluate(prj, parent)?)
            }

            Ok(types)
        }
    }
    impl<T: EvaluableTypeExpression> EvaluableTypeExpression for Option<T> {
        type Out = Option<T::Out>;

        fn evaluate(&self, prj: &mut Project, parent: DefID) -> Result<Self::Out, String> {
            Ok(match self {
                Some(v) => Some(v.evaluate(prj, parent)?),
                None => None,
            })
        }
    }
    impl<T: EvaluableTypeExpression> EvaluableTypeExpression for Box<T> {
        type Out = T::Out;

        fn evaluate(&self, prj: &mut Project, parent: DefID) -> Result<Self::Out, String> {
            self.as_ref().evaluate(prj, parent)
        }
    }
}
