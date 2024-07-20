//! A [Project] contains a tree of [Def]initions, referred to by their [Path]
use crate::{
    definition::{Def, DefKind, TypeKind},
    handle::DefID,
    module,
    node::{Node, NodeSource},
    path::Path,
};
use cl_ast::PathPart;
use cl_structures::index_map::IndexMap;
use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

use self::evaluate::EvaluableTypeExpression;

#[derive(Clone, Debug)]
pub struct Project<'a> {
    pub pool: IndexMap<DefID, Def<'a>>,
    /// Stores anonymous tuples, function pointer types, etc.
    pub anon_types: HashMap<TypeKind, DefID>,
    pub root: DefID,
}

impl Project<'_> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Project<'_> {
    fn default() -> Self {
        const ROOT_PATH: cl_ast::Path = cl_ast::Path { absolute: true, parts: Vec::new() };

        let mut pool = IndexMap::default();
        let root = pool.insert(Def {
            module: Default::default(),
            kind: DefKind::Type(TypeKind::Module),
            node: Node::new(ROOT_PATH, Some(NodeSource::Root)),
        });
        let never = pool.insert(Def {
            module: module::Module::new(root),
            kind: DefKind::Type(TypeKind::Never),
            node: Node::new(ROOT_PATH, None),
        });
        let empty = pool.insert(Def {
            module: module::Module::new(root),
            kind: DefKind::Type(TypeKind::Empty),
            node: Node::new(ROOT_PATH, None),
        });

        let mut anon_types = HashMap::new();
        anon_types.insert(TypeKind::Empty, empty);
        anon_types.insert(TypeKind::Never, never);

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

    /// Returns the DefID of the Self type within the given DefID's context
    pub fn selfty_of(&self, node: DefID) -> Option<DefID> {
        match self[node].kind {
            DefKind::Impl(id) => Some(id),
            DefKind::Type(_) => Some(node),
            _ => self.selfty_of(self.parent_of(node)?),
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
            [PathPart::Ident(name)] => {
                let (ty, val) = self[within].module.get(*name);
                Some((ty, val, path.pop_front()?))
            }
            [PathPart::Ident(name), ..] => {
                let ty = self[within].module.get_type(*name)?;
                self.get(path.pop_front()?, ty)
            }
            [PathPart::SelfTy, ..] => self.get(path.pop_front()?, self.selfty_of(within)?),
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
                PathPart::SelfTy => self.get_type(path.pop_front()?, self.selfty_of(within)?),
                PathPart::Ident(name) => match module.types.get(name) {
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
            PathPart::Ident(name) => Some((
                self[within].module.values.get(name).copied()?,
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
        kind: TypeKind,
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
    use crate::module;
    use cl_ast::{Sym, Ty, TyArray, TyFn, TyKind, TyRef, TySlice, TyTuple};

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
                // TyKind::Path must be looked up explicitly
                TyKind::Path(path) => path.evaluate(prj, parent)?,
                TyKind::Slice(slice) => slice.evaluate(prj, parent)?,
                TyKind::Array(array) => array.evaluate(prj, parent)?,
                TyKind::Tuple(tup) => tup.evaluate(prj, parent)?,
                TyKind::Ref(tyref) => tyref.evaluate(prj, parent)?,
                TyKind::Fn(tyfn) => tyfn.evaluate(prj, parent)?,
            };
            // println!("{self} => {id:?}");

            Ok(id)
        }
    }

    impl EvaluableTypeExpression for Sym {
        type Out = DefID;

        fn evaluate(&self, prj: &mut Project, parent: DefID) -> Result<Self::Out, String> {
            prj[parent]
                .module
                .types
                .get(self)
                .copied()
                .ok_or_else(|| format!("{self} is not a member of {:?}", prj[parent].name()))
        }
    }

    impl EvaluableTypeExpression for TySlice {
        type Out = DefID;

        fn evaluate(&self, prj: &mut Project, parent: DefID) -> Result<Self::Out, String> {
            let ty = self.ty.evaluate(prj, parent)?;
            let root = prj.root;
            let id = prj.insert_anonymous_type(TypeKind::Slice(ty), move || Def {
                module: module::Module::new(root),
                node: Node::new(Default::default(), None),
                kind: DefKind::Type(TypeKind::Slice(ty)),
            });

            Ok(id)
        }
    }

    impl EvaluableTypeExpression for TyArray {
        type Out = DefID;

        fn evaluate(&self, prj: &mut Project, parent: DefID) -> Result<Self::Out, String> {
            let kind = TypeKind::Array(self.ty.evaluate(prj, parent)?, self.count);
            let root = prj.root;
            let id = prj.insert_anonymous_type(kind.clone(), move || Def {
                module: module::Module::new(root),
                node: Node::new(Default::default(), None),
                kind: DefKind::Type(kind),
            });

            Ok(id)
        }
    }

    impl EvaluableTypeExpression for TyTuple {
        type Out = DefID;
        fn evaluate(&self, prj: &mut Project, parent: DefID) -> Result<DefID, String> {
            let types = self.types.evaluate(prj, parent)?;
            let root = prj.root;
            let id = prj.insert_anonymous_type(TypeKind::Tuple(types.clone()), move || Def {
                module: module::Module::new(root),
                node: Node::new(Default::default(), None),
                kind: DefKind::Type(TypeKind::Tuple(types)),
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
                module: module::Module::new(root),
                node: Node::new(Default::default(), None),
                kind: DefKind::Type(TypeKind::Ref(*count, to)),
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
                module: module::Module::new(root),
                node: Node::new(Default::default(), None),
                kind: DefKind::Type(TypeKind::FnSig { args, rety }),
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
                PathPart::SelfTy => prj
                    .selfty_of(parent)
                    .ok_or_else(|| "Attempt to get Self outside a Self-able context".into()),
                PathPart::Ident(name) => name.evaluate(prj, parent),
            }
        }
    }
    impl<'a> EvaluableTypeExpression for Path<'a> {
        type Out = DefID;
        fn evaluate(&self, prj: &mut Project, parent: DefID) -> Result<Self::Out, String> {
            let (tid, vid, path) = prj.get(*self, parent).ok_or("Failed to traverse path")?;
            if !path.is_empty() {
                Err(format!("Could not traverse past boundary: {path}"))?;
            }
            match (tid, vid) {
                (Some(ty), _) => Ok(ty),
                (None, Some(val)) => Ok(val),
                (None, None) => Err(format!("No type or value found at path {self}")),
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
