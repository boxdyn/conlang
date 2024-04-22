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

pub mod key {
    use cl_structures::intern_pool::*;

    // define the index types
    make_intern_key! {
        /// Uniquely represents a [Def][1] in the [Def][1] [Pool]
        ///
        /// [1]: crate::definition::Def
        DefID,
    }

    impl std::fmt::Display for DefID {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }
}

pub mod definition {
    use crate::{key::DefID, module::Module};
    use cl_ast::{Item, Meta, Visibility};
    use std::{fmt::Debug, str::FromStr};

    mod display;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Def<'a> {
        pub name: &'a str,
        pub vis: Visibility,
        pub meta: &'a [Meta],
        pub kind: DefKind<'a>,
        pub source: Option<&'a Item>,
        pub module: Module<'a>,
    }

    mod builder_functions {
        use super::*;

        impl<'a> Def<'a> {
            pub fn set_name(&mut self, name: &'a str) -> &mut Self {
                self.name = name;
                self
            }
            pub fn set_vis(&mut self, vis: Visibility) -> &mut Self {
                self.vis = vis;
                self
            }
            pub fn set_meta(&mut self, meta: &'a [Meta]) -> &mut Self {
                self.meta = meta;
                self
            }
            pub fn set_kind(&mut self, kind: DefKind<'a>) -> &mut Self {
                self.kind = kind;
                self
            }
            pub fn set_source(&mut self, source: &'a Item) -> &mut Self {
                self.source = Some(source);
                self
            }
            pub fn set_module(&mut self, module: Module<'a>) -> &mut Self {
                self.module = module;
                self
            }
        }
    }

    impl Default for Def<'_> {
        fn default() -> Self {
            Self {
                name: Default::default(),
                vis: Visibility::Public,
                meta: Default::default(),
                kind: Default::default(),
                source: Default::default(),
                module: Default::default(),
            }
        }
    }

    #[derive(Clone, Default, Debug, PartialEq, Eq)]
    pub enum DefKind<'a> {
        /// An unevaluated definition
        #[default]
        Undecided,
        /// An impl block
        Impl(DefID),
        /// A use tree, and its parent
        Use(DefID),
        /// A type, such as a `type`, `struct`, or `enum`
        Type(TypeKind<'a>),
        /// A value, such as a `const`, `static`, or `fn`
        Value(ValueKind),
    }

    /// A [ValueKind] represents an item in the Value Namespace
    /// (a component of a [Project](crate::project::Project)).
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum ValueKind {
        Const(DefID),
        Static(DefID),
        Fn(DefID),
    }
    /// A [TypeKind] represents an item in the Type Namespace
    /// (a component of a [Project](crate::project::Project)).
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub enum TypeKind<'a> {
        /// An alias for an already-defined type
        Alias(Option<DefID>),
        /// A primitive type, built-in to the compiler
        Intrinsic(Intrinsic),
        /// A user-defined aromatic data type
        Adt(Adt<'a>),
        /// A reference to an already-defined type: &T
        Ref(u16, DefID),
        /// A contiguous view of dynamically sized memory
        Slice(DefID),
        /// A contiguous view of statically sized memory
        Array(DefID, usize),
        /// A tuple of existing types
        Tuple(Vec<DefID>),
        /// A function which accepts multiple inputs and produces an output
        FnSig { args: DefID, rety: DefID },
        /// The unit type
        Empty,
        /// The never type
        Never,
        /// The Self type
        SelfTy,
        /// An untyped module
        Module,
    }

    /// A user-defined Aromatic Data Type
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub enum Adt<'a> {
        /// A union-like enum type
        Enum(Vec<(&'a str, Option<DefID>)>),
        /// A C-like enum
        CLikeEnum(Vec<(&'a str, u128)>),
        /// An enum with no fields, which can never be constructed
        FieldlessEnum,

        /// A structural product type with named members
        Struct(Vec<(&'a str, Visibility, DefID)>),
        /// A structural product type with unnamed members
        TupleStruct(Vec<(Visibility, DefID)>),
        /// A structural product type of neither named nor unnamed members
        UnitStruct,

        /// A choose your own undefined behavior type
        /// TODO: should unions be a language feature?
        Union(Vec<(&'a str, DefID)>),
    }

    /// The set of compiler-intrinsic types.
    /// These primitive types have native implementations of the basic operations.
    #[allow(non_camel_case_types)]
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub enum Intrinsic {
        /// An 8-bit signed integer: `#[intrinsic = "i8"]`
        I8,
        /// A 16-bit signed integer: `#[intrinsic = "i16"]`
        I16,
        /// A 32-bit signed integer: `#[intrinsic = "i32"]`
        I32,
        /// A 64-bit signed integer: `#[intrinsic = "i32"]`
        I64,
        // /// A 128-bit signed integer: `#[intrinsic = "i32"]`
        // I128,
        /// An 8-bit unsigned integer: `#[intrinsic = "u8"]`
        U8,
        /// A 16-bit unsigned integer: `#[intrinsic = "u16"]`
        U16,
        /// A 32-bit unsigned integer: `#[intrinsic = "u32"]`
        U32,
        /// A 64-bit unsigned integer: `#[intrinsic = "u64"]`
        U64,
        // /// A 128-bit unsigned integer: `#[intrinsic = "u128"]`
        // U128,
        /// A boolean (`true` or `false`): `#[intrinsic = "bool"]`
        Bool,
        /// The unicode codepoint type: #[intrinsic = "char"]
        Char,
    }

    impl FromStr for Intrinsic {
        type Err = ();

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Ok(match s {
                "i8" => Intrinsic::I8,
                "i16" => Intrinsic::I16,
                "i32" => Intrinsic::I32,
                "i64" => Intrinsic::I64,
                "u8" => Intrinsic::U8,
                "u16" => Intrinsic::U16,
                "u32" => Intrinsic::U32,
                "u64" => Intrinsic::U64,
                "bool" => Intrinsic::Bool,
                "char" => Intrinsic::Char,
                _ => Err(())?,
            })
        }
    }
}

pub mod module {
    //! A [Module] is a node in the Module Tree (a component of a
    //! [Project](crate::project::Project))
    use cl_structures::intern_pool::InternKey;

    use crate::key::DefID;
    use std::collections::HashMap;

    /// A [Module] is a node in the Module Tree (a component of a
    /// [Project](crate::project::Project)).
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct Module<'a> {
        pub parent: Option<DefID>,
        pub types: HashMap<&'a str, DefID>,
        pub values: HashMap<&'a str, DefID>,
        pub imports: Vec<DefID>,
    }

    impl<'a> Module<'a> {
        pub fn new(parent: DefID) -> Self {
            Self { parent: Some(parent), ..Default::default() }
        }
        pub fn with_optional_parent(parent: Option<DefID>) -> Self {
            Self { parent, ..Default::default() }
        }

        pub fn get(&self, name: &'a str) -> (Option<DefID>, Option<DefID>) {
            (self.get_type(name), self.get_value(name))
        }
        pub fn get_type(&self, name: &'a str) -> Option<DefID> {
            self.types.get(name).copied()
        }
        pub fn get_value(&self, name: &'a str) -> Option<DefID> {
            self.values.get(name).copied()
        }

        /// Inserts a type with the provided [name](str) and [id](DefID)
        pub fn insert_type(&mut self, name: &'a str, id: DefID) -> Option<DefID> {
            self.types.insert(name, id)
        }

        /// Inserts a value with the provided [name](str) and [id](DefID)
        pub fn insert_value(&mut self, name: &'a str, id: DefID) -> Option<DefID> {
            self.values.insert(name, id)
        }
    }

    impl std::fmt::Display for Module<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { parent, types, values, imports } = self;
            if let Some(parent) = parent {
                writeln!(f, "Parent: {}", parent.get())?;
            }
            for (name, table) in [("Types", types), ("Values", values)] {
                if table.is_empty() {
                    continue;
                }
                writeln!(f, "{name}:")?;
                for (name, id) in table.iter() {
                    writeln!(f, "    {name} => {id}")?;
                }
            }
            if !imports.is_empty() {
                write!(f, "Imports:")?;
                for id in imports {
                    write!(f, "{id},")?;
                }
            }
            Ok(())
        }
    }
}

pub mod path {

    use cl_ast::{Path as AstPath, PathPart};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Path<'p> {
        pub absolute: bool,
        pub parts: &'p [PathPart],
    }

    impl<'p> Path<'p> {
        pub fn new(path: &'p AstPath) -> Self {
            let AstPath { absolute, parts } = path;
            Self { absolute: *absolute, parts }
        }
        pub fn relative(self) -> Self {
            Self { absolute: false, ..self }
        }
        pub fn pop_front(self) -> Option<Self> {
            let Self { absolute, parts } = self;
            Some(Self { absolute, parts: parts.get(1..)? })
        }
        pub fn is_empty(&self) -> bool {
            self.parts.is_empty()
        }
        pub fn len(&self) -> usize {
            self.parts.len()
        }
        pub fn front(&self) -> Option<&PathPart> {
            self.parts.first()
        }
    }

    impl<'p> From<&'p AstPath> for Path<'p> {
        fn from(value: &'p AstPath) -> Self {
            Self::new(value)
        }
    }
    impl AsRef<[PathPart]> for Path<'_> {
        fn as_ref(&self) -> &[PathPart] {
            self.parts
        }
    }

    impl std::fmt::Display for Path<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            const SEPARATOR: &str = "::";
            let Self { absolute, parts } = self;
            if *absolute {
                write!(f, "{SEPARATOR}")?
            }
            for (idx, part) in parts.iter().enumerate() {
                write!(f, "{}{part}", if idx > 0 { SEPARATOR } else { "" })?;
            }
            Ok(())
        }
    }
}

pub mod project {
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
}

pub mod name_collector {
    //! Performs step 1 of type checking: Collecting all the names of things into [Module] units
    use crate::{
        definition::{Def, DefKind},
        key::DefID,
        module::Module as Mod,
        project::Project as Prj,
    };
    use cl_ast::*;

    pub trait NameCollectable<'a> {
        /// Collects the identifiers within this node,
        /// returning a new [DefID] if any were allocated,
        /// else returning the parent [DefID]
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str>;

        fn collect_in_root(&'a self, c: &mut Prj<'a>) -> Result<DefID, &'static str> {
            self.collect(c, c.root)
        }
    }

    impl<'a> NameCollectable<'a> for File {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            for item in &self.items {
                item.collect(c, parent)?;
            }
            Ok(parent)
        }
    }
    impl<'a> NameCollectable<'a> for Item {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            let Item { attrs: Attrs { meta }, vis, kind, .. } = self;
            let id = match kind {
                ItemKind::Impl(i) => i.collect(c, parent),
                ItemKind::Module(i) => i.collect(c, parent),
                ItemKind::Alias(i) => i.collect(c, parent),
                ItemKind::Enum(i) => i.collect(c, parent),
                ItemKind::Struct(i) => i.collect(c, parent),
                ItemKind::Const(i) => i.collect(c, parent),
                ItemKind::Static(i) => i.collect(c, parent),
                ItemKind::Function(i) => i.collect(c, parent),
                ItemKind::Use(i) => i.collect(c, parent),
            }?;
            c[id].set_meta(meta).set_vis(*vis).set_source(self);

            Ok(id)
        }
    }
    impl<'a> NameCollectable<'a> for Module {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            let Self { name: Identifier(name), kind } = self;
            let module =
                c.pool
                    .insert(Def { name, module: Mod::new(parent), ..Default::default() });
            c[parent].module.types.insert(name, module);

            match kind {
                ModuleKind::Inline(file) => file.collect(c, module)?,
                ModuleKind::Outline => todo!("Out of line modules: {name}"),
            };
            Ok(module)
        }
    }
    impl<'a> NameCollectable<'a> for Impl {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            let Self { target: _, body } = self;
            let def = Def { module: Mod::new(parent), ..Default::default() };
            let id = c.pool.insert(def);

            // items will get reparented after name collection, when target is available
            body.collect(c, id)?;

            Ok(id)
        }
    }
    impl<'a> NameCollectable<'a> for Use {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            let def =
                Def { module: Mod::new(parent), kind: DefKind::Use(parent), ..Default::default() };

            let id = c.pool.insert(def);
            c[parent].module.imports.push(id);

            Ok(id)
        }
    }
    impl<'a> NameCollectable<'a> for Alias {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            let Alias { to: Identifier(name), .. } = self;

            let def = Def { name, module: Mod::new(parent), ..Default::default() };
            let id = c.pool.insert(def);

            c[parent].module.types.insert(name, id);
            Ok(id)
        }
    }
    impl<'a> NameCollectable<'a> for Enum {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            let Enum { name: Identifier(name), .. } = self;

            let def = Def { name, module: Mod::new(parent), ..Default::default() };
            let id = c.pool.insert(def);

            c[parent].module.types.insert(name, id);
            Ok(id)
        }
    }
    impl<'a> NameCollectable<'a> for Struct {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            let Struct { name: Identifier(name), .. } = self;

            let def = Def { name, module: Mod::new(parent), ..Default::default() };
            let id = c.pool.insert(def);

            c[parent].module.types.insert(name, id);
            Ok(id)
        }
    }
    impl<'a> NameCollectable<'a> for Const {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            let Self { name: Identifier(name), init, .. } = self;

            let kind = DefKind::Undecided;

            let def = Def { name, kind, module: Mod::new(parent), ..Default::default() };
            let id = c.pool.insert(def);

            c[parent].module.values.insert(name, id);
            init.collect(c, id)?;

            Ok(id)
        }
    }
    impl<'a> NameCollectable<'a> for Static {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            let Self { name: Identifier(name), init, .. } = self;

            let kind = DefKind::Undecided;

            let def = Def { name, kind, module: Mod::new(parent), ..Default::default() };
            let id = c.pool.insert(def);

            c[parent].module.values.insert(name, id);
            init.collect(c, id)?;

            Ok(id)
        }
    }
    impl<'a> NameCollectable<'a> for Function {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            let Self { name: Identifier(name), body, .. } = self;

            let kind = DefKind::Undecided;

            let def = Def { name, kind, module: Mod::new(parent), ..Default::default() };
            let id = c.pool.insert(def);

            c[parent].module.values.insert(name, id);
            body.collect(c, id)?;

            Ok(id)
        }
    }
    impl<'a> NameCollectable<'a> for Block {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            self.stmts.as_slice().collect(c, parent)
        }
    }
    impl<'a> NameCollectable<'a> for Stmt {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            self.kind.collect(c, parent)
        }
    }
    impl<'a> NameCollectable<'a> for StmtKind {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            match self {
                StmtKind::Empty => Ok(parent),
                StmtKind::Local(Let { init, .. }) => init.collect(c, parent),
                StmtKind::Item(item) => item.collect(c, parent),
                StmtKind::Expr(expr) => expr.collect(c, parent),
            }
        }
    }
    impl<'a> NameCollectable<'a> for Expr {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            self.kind.collect(c, parent)
        }
    }
    impl<'a> NameCollectable<'a> for ExprKind {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            match self {
                ExprKind::Assign(Assign { parts, .. }) | ExprKind::Binary(Binary { parts, .. }) => {
                    parts.0.collect(c, parent)?;
                    parts.1.collect(c, parent)
                }
                ExprKind::Unary(Unary { tail, .. }) => tail.collect(c, parent),
                ExprKind::Index(Index { head, indices }) => {
                    indices.collect(c, parent)?;
                    head.collect(c, parent)
                }
                ExprKind::Array(Array { values }) => values.collect(c, parent),
                ExprKind::ArrayRep(ArrayRep { value, repeat }) => {
                    value.collect(c, parent)?;
                    repeat.collect(c, parent)
                }
                ExprKind::AddrOf(AddrOf { expr, .. }) => expr.collect(c, parent),
                ExprKind::Block(block) => block.collect(c, parent),
                ExprKind::Group(Group { expr }) => expr.collect(c, parent),
                ExprKind::Tuple(Tuple { exprs }) => exprs.collect(c, parent),
                ExprKind::While(While { cond, pass, fail })
                | ExprKind::If(If { cond, pass, fail })
                | ExprKind::For(For { cond, pass, fail, .. }) => {
                    cond.collect(c, parent)?;
                    pass.collect(c, parent)?;
                    fail.body.collect(c, parent)
                }
                ExprKind::Break(Break { body }) | ExprKind::Return(Return { body }) => {
                    body.collect(c, parent)
                }
                _ => Ok(parent),
            }
        }
    }
    impl<'a, T: NameCollectable<'a>> NameCollectable<'a> for [T] {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            let mut last = parent;
            for expr in self {
                last = expr.collect(c, parent)?;
            }
            Ok(last)
        }
    }
    impl<'a, T: NameCollectable<'a>> NameCollectable<'a> for Option<T> {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            match self {
                Some(body) => body.collect(c, parent),
                None => Ok(parent),
            }
        }
    }
    impl<'a, T: NameCollectable<'a>> NameCollectable<'a> for Box<T> {
        fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
            (**self).collect(c, parent)
        }
    }
}

pub mod use_importer {
    #![allow(unused)]
    use std::fmt::format;

    use cl_ast::*;

    use crate::{
        definition::{Def, DefKind},
        key::DefID,
        project::Project,
    };

    type UseResult = Result<(), String>;

    impl<'a> Project<'a> {
        pub fn resolve_imports(&mut self) -> UseResult {
            for id in self.pool.key_iter() {
                self.visit_def(id)?;
            }
            Ok(())
        }

        pub fn visit_def(&mut self, id: DefID) -> UseResult {
            let Def { name, vis, meta, kind, source, module } = &self.pool[id];
            if let (DefKind::Use(parent), Some(source)) = (kind, source) {
                let Item { kind: ItemKind::Use(u), .. } = source else {
                    Err(format!("Not a use item: {source}"))?
                };
                println!("Importing use item {u}");

                self.visit_use(u, *parent);
            }
            Ok(())
        }

        pub fn visit_use(&mut self, u: &'a Use, parent: DefID) -> UseResult {
            let Use { absolute, tree } = u;

            self.visit_use_tree(tree, parent, if *absolute { self.root } else { parent })
        }

        pub fn visit_use_tree(&mut self, tree: &'a UseTree, parent: DefID, c: DefID) -> UseResult {
            match tree {
                UseTree::Tree(trees) => {
                    for tree in trees {
                        self.visit_use_tree(tree, parent, c)?;
                    }
                }
                UseTree::Path(part, rest) => {
                    let c = self.evaluate(part, c)?;
                    self.visit_use_tree(rest, parent, c)?;
                }

                UseTree::Name(name) => self.visit_use_leaf(name, parent, c)?,
                UseTree::Alias(Identifier(from), Identifier(to)) => {
                    self.visit_use_alias(from, to, parent, c)?
                }
                UseTree::Glob => self.visit_use_glob(parent, c)?,
            }
            Ok(())
        }

        pub fn visit_use_path(&mut self) -> UseResult {
            Ok(())
        }

        pub fn visit_use_leaf(
            &mut self,
            part: &'a Identifier,
            parent: DefID,
            c: DefID,
        ) -> UseResult {
            let Identifier(name) = part;
            self.visit_use_alias(name, name, parent, c)
        }

        pub fn visit_use_alias(
            &mut self,
            from: &'a str,
            name: &'a str,
            parent: DefID,
            c: DefID,
        ) -> UseResult {
            let mut imported = false;
            let c_mod = &self[c].module;
            let (tid, vid) = (
                c_mod.types.get(from).copied(),
                c_mod.values.get(from).copied(),
            );
            let parent = &mut self[parent].module;

            if let Some(tid) = tid {
                parent.types.insert(name, tid);
                imported = true;
            }

            if let Some(vid) = vid {
                parent.values.insert(name, vid);
                imported = true;
            }

            if imported {
                Ok(())
            } else {
                Err(format!("Identifier {name} not found in module {c}"))
            }
        }

        pub fn visit_use_glob(&mut self, parent: DefID, c: DefID) -> UseResult {
            // Loop over all the items in c, and add them as items in the parent
            if parent == c {
                return Ok(());
            }
            let [parent, c] = self
                .pool
                .get_many_mut([parent, c])
                .expect("parent and c are not the same");

            for (k, v) in &c.module.types {
                parent.module.types.entry(*k).or_insert(*v);
            }
            for (k, v) in &c.module.values {
                parent.module.values.entry(*k).or_insert(*v);
            }
            Ok(())
        }
    }
}

pub mod type_resolver {
    //! Performs step 2 of type checking: Evaluating type definitions

    use crate::{
        definition::{Adt, Def, DefKind, TypeKind, ValueKind},
        key::DefID,
        module,
        project::{evaluate::EvaluableTypeExpression, Project as Prj},
    };
    use cl_ast::*;

    /// Evaluate a single ID
    pub fn resolve(prj: &mut Prj, id: DefID) -> Result<(), &'static str> {
        let (DefKind::Undecided, Some(source)) = (&prj[id].kind, prj[id].source) else {
            return Ok(());
        };
        let kind = match &source.kind {
            ItemKind::Alias(_) => "type",
            ItemKind::Module(_) => "mod",
            ItemKind::Enum(_) => "enum",
            ItemKind::Struct(_) => "struct",
            ItemKind::Const(_) => "const",
            ItemKind::Static(_) => "static",
            ItemKind::Function(_) => "fn",
            ItemKind::Impl(_) => "impl",
            ItemKind::Use(_) => "use",
        };
        eprintln!(
            "Resolver: \x1b[32mEvaluating\x1b[0m \"\x1b[36m{kind} {}\x1b[0m\" (`{id:?}`)",
            prj[id].name
        );

        prj[id].kind = source.resolve_type(prj, id)?;

        eprintln!("\x1b[33m=> {}\x1b[0m", prj[id].kind);

        Ok(())
    }

    /// Resolves a given node
    pub trait TypeResolvable<'a> {
        /// The return type upon success
        type Out;
        /// Resolves type expressions within this node
        fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str>;
    }

    impl<'a> TypeResolvable<'a> for Item {
        type Out = DefKind<'a>;
        fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
            let Self { attrs: Attrs { meta }, kind, .. } = self;
            for meta in meta {
                if let Ok(def) = meta.resolve_type(prj, id) {
                    return Ok(def);
                }
            }
            kind.resolve_type(prj, id)
        }
    }

    impl<'a> TypeResolvable<'a> for Meta {
        type Out = DefKind<'a>;

        #[allow(unused_variables)]
        fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
            let Self { name: Identifier(name), kind } = self;
            match (name.as_str(), kind) {
                ("intrinsic", MetaKind::Equals(Literal::String(intrinsic))) => Ok(DefKind::Type(
                    TypeKind::Intrinsic(intrinsic.parse().map_err(|_| "unknown intrinsic type")?),
                )),
                (_, MetaKind::Plain) => Ok(DefKind::Type(TypeKind::Intrinsic(
                    name.parse().map_err(|_| "Unknown intrinsic type")?,
                ))),
                _ => Err("Unknown meta attribute"),
            }
        }
    }

    impl<'a> TypeResolvable<'a> for ItemKind {
        type Out = DefKind<'a>;
        fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
            if prj[id].source.map(|s| &s.kind as *const _) != Some(self as *const _) {
                return Err("id is not self!");
            }
            match self {
                ItemKind::Module(i) => i.resolve_type(prj, id),
                ItemKind::Impl(i) => i.resolve_type(prj, id),
                ItemKind::Alias(i) => i.resolve_type(prj, id),
                ItemKind::Enum(i) => i.resolve_type(prj, id),
                ItemKind::Struct(i) => i.resolve_type(prj, id),
                ItemKind::Const(i) => i.resolve_type(prj, id),
                ItemKind::Static(i) => i.resolve_type(prj, id),
                ItemKind::Function(i) => i.resolve_type(prj, id),
                ItemKind::Use(i) => i.resolve_type(prj, id),
            }
        }
    }

    impl<'a> TypeResolvable<'a> for Module {
        type Out = DefKind<'a>;
        #[allow(unused_variables)]
        fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
            Ok(DefKind::Type(TypeKind::Module))
        }
    }

    impl<'a> TypeResolvable<'a> for Impl {
        type Out = DefKind<'a>;

        fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
            let parent = prj.parent_of(id).unwrap_or(id);

            let target = match &self.target {
                ImplKind::Type(t) => t.evaluate(prj, parent),
                ImplKind::Trait { for_type, .. } => for_type.evaluate(prj, parent),
            }
            .map_err(|_| "Unresolved type in impl target")?;

            prj[id].module.parent = Some(target);

            Ok(DefKind::Impl(target))
        }
    }

    impl<'a> TypeResolvable<'a> for Use {
        type Out = DefKind<'a>;

        fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
            todo!("Resolve types for {self} with ID {id} in {prj:?}")
        }
    }

    impl<'a> TypeResolvable<'a> for Alias {
        type Out = DefKind<'a>;

        fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
            let parent = prj.parent_of(id).unwrap_or(id);
            let alias = if let Some(ty) = &self.from {
                Some(
                    ty.evaluate(prj, parent)
                        .or_else(|_| ty.evaluate(prj, id))
                        .map_err(|_| "Unresolved type in alias")?,
                )
            } else {
                None
            };

            Ok(DefKind::Type(TypeKind::Alias(alias)))
        }
    }

    impl<'a> TypeResolvable<'a> for Enum {
        type Out = DefKind<'a>;

        fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
            let Self { name: _, kind } = self;
            let EnumKind::Variants(v) = kind else {
                return Ok(DefKind::Type(TypeKind::Adt(Adt::FieldlessEnum)));
            };
            let mut fields = vec![];
            for v @ Variant { name: Identifier(name), kind: _ } in v {
                let id = v.resolve_type(prj, id)?;
                fields.push((name.as_str(), id))
            }
            Ok(DefKind::Type(TypeKind::Adt(Adt::Enum(fields))))
        }
    }

    impl<'a> TypeResolvable<'a> for Variant {
        type Out = Option<DefID>;

        fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
            let parent = prj.parent_of(id).unwrap_or(id);
            let Self { name: Identifier(name), kind } = self;

            let adt = match kind {
                VariantKind::Plain => return Ok(None),
                VariantKind::CLike(_) => todo!("Resolve variant info for C-like enums"),
                VariantKind::Tuple(ty) => {
                    return ty
                        .evaluate(prj, parent)
                        .map_err(|_| "Unresolved type in enum tuple variant")
                        .map(Some)
                }
                VariantKind::Struct(members) => Adt::Struct(members.resolve_type(prj, id)?),
            };

            let def = Def {
                name,
                kind: DefKind::Type(TypeKind::Adt(adt)),
                module: module::Module::new(id),
                ..Default::default()
            };

            let new_id = prj.pool.insert(def);
            // Insert the struct variant type into the enum's namespace
            prj[id].module.types.insert(name, new_id);

            Ok(Some(new_id))
        }
    }

    impl<'a> TypeResolvable<'a> for Struct {
        type Out = DefKind<'a>;
        fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
            let parent = prj.parent_of(id).unwrap_or(id);
            let Self { name: _, kind } = self;
            Ok(match kind {
                StructKind::Empty => DefKind::Type(TypeKind::Empty),
                StructKind::Tuple(types) => DefKind::Type(TypeKind::Adt(Adt::TupleStruct({
                    let mut out = vec![];
                    for ty in types {
                        out.push((
                            Visibility::Public,
                            ty.evaluate(prj, parent)
                                .map_err(|_| "Unresolved type in tuple-struct member")?,
                        ));
                    }
                    out
                }))),
                StructKind::Struct(members) => {
                    DefKind::Type(TypeKind::Adt(Adt::Struct(members.resolve_type(prj, id)?)))
                }
            })
        }
    }

    impl<'a> TypeResolvable<'a> for StructMember {
        type Out = (&'a str, Visibility, DefID);

        fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
            let parent = prj.parent_of(id).unwrap_or(id);
            let Self { name: Identifier(name), vis, ty } = self;

            let ty = ty
                .evaluate(prj, parent)
                .map_err(|_| "Invalid type while resolving StructMember")?;

            Ok((name, *vis, ty))
        }
    }

    impl<'a> TypeResolvable<'a> for Const {
        type Out = DefKind<'a>;

        fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
            let Self { ty, .. } = self;
            let ty = ty
                .evaluate(prj, id)
                .map_err(|_| "Invalid type while resolving const")?;
            Ok(DefKind::Value(ValueKind::Const(ty)))
        }
    }
    impl<'a> TypeResolvable<'a> for Static {
        type Out = DefKind<'a>;

        fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
            let parent = prj.parent_of(id).unwrap_or(id);
            let Self { ty, .. } = self;
            let ty = ty
                .evaluate(prj, parent)
                .map_err(|_| "Invalid type while resolving static")?;
            Ok(DefKind::Value(ValueKind::Static(ty)))
        }
    }

    impl<'a> TypeResolvable<'a> for Function {
        type Out = DefKind<'a>;

        fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
            let parent = prj.parent_of(id).unwrap_or(id);
            let Self { sign, .. } = self;
            let sign = sign
                .evaluate(prj, parent)
                .map_err(|_| "Invalid type in function signature")?;
            Ok(DefKind::Value(ValueKind::Fn(sign)))
        }
    }

    impl<'a, T: TypeResolvable<'a>> TypeResolvable<'a> for [T] {
        type Out = Vec<T::Out>;

        fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
            let mut members = vec![];
            for member in self {
                members.push(member.resolve_type(prj, id)?);
            }
            Ok(members)
        }
    }
    impl<'a, T: TypeResolvable<'a>> TypeResolvable<'a> for Option<T> {
        type Out = Option<T::Out>;

        fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
            match self {
                Some(t) => Some(t.resolve_type(prj, id)).transpose(),
                None => Ok(None),
            }
        }
    }
}

pub mod typeref {
    //! Stores type and inference info

    use crate::key::DefID;

    /// The Type struct represents all valid types, and can be trivially equality-compared
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct TypeRef {
        /// Types can be [Generic](RefKind::Generic) or [Concrete](RefKind::Concrete)
        kind: RefKind,
    }

    /// Types can be [Generic](RefKind::Generic) or [Concrete](RefKind::Concrete)
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum RefKind {
        /// A Concrete type has an associated [Def](super::definition::Def)
        Concrete(DefID),
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

pub mod rule {
    use crate::key::DefID;

    pub struct Rule {
        /// What is this Rule for?
        pub operation: (),
        /// What inputs does it take?
        pub inputs: Vec<DefID>,
        /// What output does it produce?
        pub output: DefID,
        /// Where did this rule come from?
        pub through: Origin,
    }

    // TODO: Genericize
    pub enum Operation {
        Mul,
        Div,
        Rem,
        Add,
        Sub,

        Deref,
        Neg,
        Not,
        At,
        Tilde,

        Index,

        If,
        While,
        For,
    }

    pub enum Origin {
        /// This rule is built into the compiler
        Intrinsic,
        /// This rule is derived from an implementation on a type
        Extrinsic(DefID),
    }
}

pub mod typeck {
    #![allow(unused)]
    use cl_ast::*;

    pub struct Context {
        rules: (),
    }

    trait TypeCheck {}
}

//
