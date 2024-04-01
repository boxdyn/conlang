//! # The Conlang Type Checker
//!
//! As a statically typed language, Conlang requires a robust type checker to enforce correctness.
#![feature(debug_closure_helpers)]
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
        /// Uniquely represents a [Def][1] in the [Def][1] [Pool]
        ///
        /// [1]: crate::definition::Def
        DefID,
    }
}

pub mod definition {
    use crate::{key::DefID, module::Module, type_kind::TypeKind, value_kind::ValueKind};
    use cl_ast::{format::FmtPretty, Item, Meta, Visibility};
    use std::fmt::Write;

    #[derive(Clone, PartialEq, Eq)]
    pub struct Def {
        pub name: String,
        pub vis: Visibility,
        pub meta: Vec<Meta>,
        pub kind: DefKind,
        pub source: Option<Item>,
        pub module: Module,
    }

    impl Default for Def {
        fn default() -> Self {
            Self {
                name: Default::default(),
                vis: Default::default(),
                meta: Default::default(),
                kind: DefKind::Type(TypeKind::Module),
                source: Default::default(),
                module: Default::default(),
            }
        }
    }

    impl Def {
        pub fn new_module(
            name: String,
            vis: Visibility,
            meta: Vec<Meta>,
            parent: Option<DefID>,
        ) -> Self {
            Self {
                name,
                vis,
                meta,
                kind: DefKind::Type(TypeKind::Module),
                source: None,
                module: Module { parent, ..Default::default() },
            }
        }
    }

    impl std::fmt::Debug for Def {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { name, vis, meta, kind, source, module } = self;
            f.debug_struct("Def")
                .field("name", &name)
                .field("vis", &vis)
                .field_with("meta", |f| write!(f, "{meta:?}"))
                .field("kind", &kind)
                .field_with("source", |f| match source {
                    Some(item) => write!(f.pretty(), "{{\n{item}\n}}"),
                    None => todo!(),
                })
                .field("module", &module)
                .finish()
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum DefKind {
        /// A type, such as a ``
        Type(TypeKind),
        /// A value, such as a `const`, `static`, or `fn`
        Value(ValueKind),
    }

    impl DefKind {
        pub fn is_type(&self) -> bool {
            matches!(self, Self::Type(_))
        }
        pub fn ty(&self) -> Option<&TypeKind> {
            match self {
                DefKind::Type(t) => Some(t),
                _ => None,
            }
        }
        pub fn is_value(&self) -> bool {
            matches!(self, Self::Value(_))
        }
        pub fn value(&self) -> Option<&ValueKind> {
            match self {
                DefKind::Value(v) => Some(v),
                _ => None,
            }
        }
    }
}

pub mod type_kind {
    //! A [TypeKind] represents an item in the Type Namespace
    //! (a component of a [Project](crate::project::Project)).

    use cl_ast::Visibility;
    use std::{fmt::Debug, str::FromStr};

    use crate::key::DefID;

    /// The kind of a type
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum TypeKind {
        /// A type which has not yet been resolved
        Undecided,
        /// An alias for an already-defined type
        Alias(Option<DefID>),
        /// A primitive type, built-in to the compiler
        Intrinsic(Intrinsic),
        /// A user-defined abstract data type
        Adt(Adt),
        /// A reference to an already-defined type: &T
        Ref(DefID),
        /// A contiguous view of dynamically sized memory
        Slice(DefID),
        /// A function pointer which accepts multiple inputs and produces an output
        FnPtr { args: Vec<DefID>, rety: DefID },
        /// The unit type
        Empty,
        /// The never type
        Never,
        /// The Self type
        SelfTy,
        /// An untyped module
        Module,
    }

    /// A user-defined Abstract Data Type
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Adt {
        /// A union-like enum type
        Enum(Vec<(String, DefID)>),
        CLikeEnum(Vec<(String, u128)>),
        /// An enum with no fields, which can never be constructed
        FieldlessEnum,

        /// A structural product type with named members
        Struct(Vec<(String, Visibility, DefID)>),
        /// A structural product type with unnamed members
        TupleStruct(Vec<(Visibility, DefID)>),
        /// A structural product type of neither named nor unnamed members
        UnitStruct,

        /// A choose your own undefined behavior type
        /// TODO: should unions be a language feature?
        Union(Vec<(String, DefID)>),
    }

    /// The set of compiler-intrinsic types.
    /// These primitive types have native implementations of the basic operations.
    #[allow(non_camel_case_types)]
    #[derive(Clone, Debug, PartialEq, Eq)]
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
                _ => Err(())?,
            })
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Float {
        F32 = 0x20,
        F64,
    }
}

pub mod value_kind {
    //! A [ValueKind] represents an item in the Value Namespace
    //! (a component of a [Project](crate::project::Project)).

    use crate::typeref::TypeRef;
    use cl_ast::Block;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum ValueKind {
        Undecided,
        Const(TypeRef),
        Static(TypeRef),
        Fn {
            // TODO: Store the variable bindings here!
            args: Vec<TypeRef>,
            rety: TypeRef,
            body: Block,
        },
    }
}

pub mod module {
    //! A [Module] is a node in the Module Tree (a component of a
    //! [Project](crate::project::Project))
    use crate::key::DefID;
    use std::collections::HashMap;

    /// A [Module] is a node in the Module Tree (a component of a
    /// [Project](crate::project::Project)).
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct Module {
        pub parent: Option<DefID>,
        pub types: HashMap<String, DefID>,
        pub values: HashMap<String, DefID>,
    }

    impl Module {
        pub fn new(parent: DefID) -> Self {
            Self { parent: Some(parent), ..Default::default() }
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
        definition::{Def, DefKind},
        key::DefID,
        path::Path,
        type_kind::TypeKind,
    };
    use cl_ast::{Identifier, PathPart, Visibility};
    use cl_structures::intern_pool::Pool;
    use std::ops::{Index, IndexMut};

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Project {
        pub pool: Pool<Def, DefID>,
        pub module_root: DefID,
    }

    impl Project {
        pub fn new() -> Self {
            Self::default()
        }
    }
    impl Default for Project {
        fn default() -> Self {
            let mut pool = Pool::default();
            let module_root = pool.insert(Def::default());
            // Insert the Never(!) type
            let never = pool.insert(Def {
                name: String::from("!"),
                vis: Visibility::Public,
                kind: DefKind::Type(TypeKind::Never),
                ..Default::default()
            });
            pool[module_root]
                .module
                .types
                .insert(String::from("!"), never);
            Self { pool, module_root }
        }
    }

    impl Project {
        pub fn parent_of(&self, module: DefID) -> Option<DefID> {
            self[module].module.parent
        }
        pub fn root_of(&self, module: DefID) -> DefID {
            match self.parent_of(module) {
                Some(module) => self.root_of(module),
                None => module,
            }
        }
        /// Resolves a path within a module tree, finding the innermost module.
        /// Returns the remaining path parts.
        pub fn get_type<'a>(&self, path: Path<'a>, within: DefID) -> Option<(DefID, Path<'a>)> {
            // TODO: Cache module lookups
            if path.absolute {
                self.get_type(path.relative(), self.root_of(within))
            } else if let Some(front) = path.front() {
                let module = &self[within].module;
                match front {
                    PathPart::SelfKw => self.get_type(path.pop_front()?, within),
                    PathPart::SuperKw => self.get_type(path.pop_front()?, module.parent?),
                    PathPart::Ident(Identifier(name)) => match module.types.get(name) {
                        Some(&submodule) => self.get_type(path.pop_front()?, submodule),
                        None => Some((within, path)),
                    },
                }
            } else {
                Some((within, path))
            }
        }

        pub fn get_value<'a>(&self, path: Path<'a>, within: DefID) -> Option<(DefID, Path<'a>)> {
            match path.front()? {
                PathPart::Ident(Identifier(name)) => Some((
                    self[within].module.values.get(name).copied()?,
                    path.pop_front()?,
                )),
                _ => None,
            }
        }

        #[rustfmt::skip]
        pub fn insert_type(&mut self, name: String, value: Def, parent: DefID) -> Option<DefID> {
            let id = self.pool.insert(value);
            self[parent].module.types.insert(name, id)
        }
        #[rustfmt::skip]
        pub fn insert_value(&mut self, name: String, value: Def, parent: DefID) -> Option<DefID> {
            let id = self.pool.insert(value);
            self[parent].module.values.insert(name, id)
        }
    }

    /// Implements [Index] and [IndexMut] for [Project]: `self.table[ID] -> Definition`
    macro_rules! impl_index {
        ($(self.$table:ident[$idx:ty] -> $out:ty),*$(,)?) => {$(
            impl Index<$idx> for Project {
                type Output = $out;
                fn index(&self, index: $idx) -> &Self::Output {
                    &self.$table[index]
                }
            }
            impl IndexMut<$idx> for Project {
                fn index_mut(&mut self, index: $idx) -> &mut Self::Output {
                    &mut self.$table[index]
                }
            }
        )*};
    }
    impl_index! {
        self.pool[DefID] -> Def,
        // self.types[TypeID] -> TypeDef,
        // self.values[ValueID] -> ValueDef,
    }
}

pub mod name_collector {
    //! Performs step 1 of type checking: Collecting all the names of things into [Module] units

    use crate::{
        definition::{Def, DefKind},
        key,
        project::Project,
        type_kind::{Adt, TypeKind},
        value_kind::ValueKind,
    };
    use cl_ast::*;
    use std::ops::{Deref, DerefMut};

    /// Collects types for future use
    #[derive(Debug, PartialEq, Eq)]
    pub struct NameCollector<'prj> {
        /// A stack of the current modules
        pub mod_stack: Vec<key::DefID>,
        /// The [Project], the type checker and resolver's central data store
        pub project: &'prj mut Project,
    }

    impl<'prj> NameCollector<'prj> {
        pub fn new(project: &'prj mut Project) -> Self {
            // create a root module
            Self { mod_stack: vec![project.module_root], project }
        }

        /// Gets the currently traversed parent module
        pub fn parent(&self) -> Option<key::DefID> {
            self.mod_stack.last().copied()
        }
    }

    impl Deref for NameCollector<'_> {
        type Target = Project;
        fn deref(&self) -> &Self::Target {
            self.project
        }
    }
    impl DerefMut for NameCollector<'_> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.project
        }
    }

    impl NameCollector<'_> {
        pub fn file(&mut self, f: &File) -> Result<(), &'static str> {
            let parent = self.parent().ok_or("No parent to add item to")?;

            for item in &f.items {
                let def = match &item.kind {
                    // modules
                    // types
                    ItemKind::Module(_) => {
                        self.module(item)?;
                        continue;
                    }
                    ItemKind::Enum(_) => Some(self.ty_enum(item)?),
                    ItemKind::Alias(_) => Some(self.ty_alias(item)?),
                    ItemKind::Struct(_) => Some(self.ty_struct(item)?),
                    // values processed by the value collector
                    ItemKind::Const(_) => Some(self.val_const(item)?),
                    ItemKind::Static(_) => Some(self.val_static(item)?),
                    ItemKind::Function(_) => Some(self.val_function(item)?),
                    ItemKind::Impl(_) => None,
                };
                let Some(def) = def else { continue };
                match def.kind {
                    DefKind::Type(_) => {
                        if let Some(v) = self.insert_type(def.name.clone(), def, parent) {
                            panic!("Redefinition of type {} ({v:?})!", self[v].name)
                        }
                    }
                    DefKind::Value(_) => {
                        if let Some(v) = self.insert_value(def.name.clone(), def, parent) {
                            panic!("Redefinition of value {} ({v:?})!", self[v].name)
                        }
                    }
                }
            }
            Ok(())
        }

        /// Collects a [Module]
        pub fn module(&mut self, m: &Item) -> Result<(), &'static str> {
            let Item { kind: ItemKind::Module(Module { name, kind }), vis, attrs, .. } = m else {
                Err("module called on Item which was not an ItemKind::Module")?
            };
            let ModuleKind::Inline(kind) = kind else {
                Err("Out-of-line modules not yet supported")?
            };
            let parent = self.parent().ok_or("No parent to add module to")?;

            let module = self.pool.insert(Def::new_module(
                name.0.clone(),
                *vis,
                attrs.meta.clone(),
                Some(parent),
            ));

            self[parent]
                .module
                .types
                .insert(name.0.clone(), module)
                .is_some()
                .then(|| panic!("Error: redefinition of module {name}"));

            self.mod_stack.push(module);
            let out = self.file(kind);
            self.mod_stack.pop();

            out
        }
    }
    /// Type collection
    impl NameCollector<'_> {
        /// Collects an [Item] of type [ItemKind::Enum]
        pub fn ty_enum(&mut self, item: &Item) -> Result<Def, &'static str> {
            let Item { kind: ItemKind::Enum(Enum { name, kind }), vis, attrs, .. } = item else {
                Err("Enum called on item which was not ItemKind::Enum")?
            };
            let kind = match kind {
                EnumKind::NoVariants => DefKind::Type(TypeKind::Adt(Adt::FieldlessEnum)),
                EnumKind::Variants(_) => DefKind::Type(TypeKind::Undecided),
            };

            Ok(Def {
                name: name.0.clone(),
                vis: *vis,
                meta: attrs.meta.clone(),
                kind,
                source: Some(item.clone()),
                module: Default::default(),
            })
        }

        /// Collects an [Item] of type [ItemKind::Alias]
        pub fn ty_alias(&mut self, item: &Item) -> Result<Def, &'static str> {
            let Item { kind: ItemKind::Alias(Alias { to: name, from }), vis, attrs, .. } = item
            else {
                Err("Alias called on Item which was not ItemKind::Alias")?
            };

            let mut kind = match from {
                Some(_) => DefKind::Type(TypeKind::Undecided),
                None => DefKind::Type(TypeKind::Alias(None)),
            };

            for meta in &attrs.meta {
                let Meta { name: meta_name, kind: meta_kind } = meta;
                match (meta_name.0.as_str(), meta_kind) {
                    ("intrinsic", MetaKind::Equals(Literal::String(intrinsic))) => {
                        kind = DefKind::Type(TypeKind::Intrinsic(
                            intrinsic.parse().map_err(|_| "unknown intrinsic type")?,
                        ));
                    }
                    ("intrinsic", MetaKind::Plain) => {
                        kind = DefKind::Type(TypeKind::Intrinsic(
                            name.0.parse().map_err(|_| "Unknown intrinsic type")?,
                        ))
                    }
                    _ => {}
                }
            }

            Ok(Def {
                name: name.0.clone(),
                vis: *vis,
                meta: attrs.meta.clone(),
                kind,
                source: Some(item.clone()),
                module: Default::default(),
            })
        }

        /// Collects an [Item] of type [ItemKind::Struct]
        pub fn ty_struct(&mut self, item: &Item) -> Result<Def, &'static str> {
            let Item { kind: ItemKind::Struct(Struct { name, kind }), vis, attrs, .. } = item
            else {
                Err("Struct called on item which was not ItemKind::Struct")?
            };
            let kind = match kind {
                StructKind::Empty => DefKind::Type(TypeKind::Adt(Adt::UnitStruct)),
                StructKind::Tuple(_) => DefKind::Type(TypeKind::Undecided),
                StructKind::Struct(_) => DefKind::Type(TypeKind::Undecided),
            };

            Ok(Def {
                name: name.0.clone(),
                vis: *vis,
                meta: attrs.meta.clone(),
                kind,
                source: Some(item.clone()),
                module: Default::default(),
            })
        }
    }
    /// Value collection
    impl NameCollector<'_> {
        pub fn val_const(&mut self, item: &Item) -> Result<Def, &'static str> {
            let Item { kind: ItemKind::Const(Const { name, .. }), vis, attrs, .. } = item else {
                Err("Const called on Item which was not ItemKind::Const")?
            };

            Ok(Def {
                name: name.0.clone(),
                vis: *vis,
                meta: attrs.meta.clone(),
                kind: DefKind::Value(ValueKind::Undecided),
                source: Some(item.clone()),
                module: Default::default(),
            })
        }

        pub fn val_static(&mut self, item: &Item) -> Result<Def, &'static str> {
            let Item { kind: ItemKind::Static(Static { name, .. }), vis, attrs, .. } = item else {
                Err("Static called on Item which was not ItemKind::Static")?
            };
            Ok(Def {
                name: name.0.clone(),
                vis: *vis,
                meta: attrs.meta.clone(),
                kind: DefKind::Type(TypeKind::Undecided),
                source: Some(item.clone()),
                module: Default::default(),
            })
        }

        pub fn val_function(&mut self, item: &Item) -> Result<Def, &'static str> {
            // TODO: treat function bodies like modules with internal items
            let Item { kind: ItemKind::Function(Function { name, .. }), vis, attrs, .. } = item
            else {
                Err("val_function called on Item which was not ItemKind::Function")?
            };
            Ok(Def {
                name: name.0.clone(),
                vis: *vis,
                meta: attrs.meta.clone(),
                kind: DefKind::Value(ValueKind::Undecided),
                source: Some(item.clone()),
                module: Default::default(),
            })
        }
    }
}

pub mod type_resolver {
    //! Performs step 2 of type checking: Evaluating type definitions
    #![allow(unused)]
    use std::ops::{Deref, DerefMut};

    use cl_ast::*;

    use crate::{definition::Def, key::DefID, project::Project};

    pub struct TypeResolver<'prj> {
        pub project: &'prj mut Project,
    }

    impl Deref for TypeResolver<'_> {
        type Target = Project;
        fn deref(&self) -> &Self::Target {
            self.project
        }
    }
    impl DerefMut for TypeResolver<'_> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.project
        }
    }

    impl TypeResolver<'_> {
        pub fn resolve(&mut self) -> Result<bool, &str> {
            #![allow(unused)]
            for typedef in self.pool.iter_mut().filter(|v| v.kind.is_type()) {
                let Def { name, vis, meta: attr, kind, source: Some(ref definition), module: _ } =
                    typedef
                else {
                    continue;
                };
                match &definition.kind {
                    ItemKind::Alias(Alias { to: _, from: Some(from) }) => match &from.kind {
                        TyKind::Never => todo!(),
                        TyKind::Empty => todo!(),
                        TyKind::SelfTy => todo!(),
                        TyKind::Path(_) => todo!(),
                        TyKind::Tuple(_) => todo!(),
                        TyKind::Ref(_) => todo!(),
                        TyKind::Fn(_) => todo!(),
                    },
                    ItemKind::Alias(_) => {}
                    ItemKind::Const(_) => todo!(),
                    ItemKind::Static(_) => todo!(),
                    ItemKind::Module(_) => todo!(),
                    ItemKind::Function(_) => {}
                    ItemKind::Struct(_) => {}
                    ItemKind::Enum(_) => {}
                    ItemKind::Impl(_) => {}
                }
            }
            Ok(true)
        }

        pub fn get_type(&self, kind: &TyKind) -> Option<DefID> {
            match kind {
                TyKind::Never => todo!(),
                TyKind::Empty => todo!(),
                TyKind::SelfTy => todo!(),
                TyKind::Path(_) => todo!(),
                TyKind::Tuple(_) => todo!(),
                TyKind::Ref(_) => todo!(),
                TyKind::Fn(_) => todo!(),
            }
            None
        }
    }
}

pub mod typeref {
    //! Stores type and reference info

    use crate::key::DefID;

    /// The Type struct represents all valid types, and can be trivially equality-compared
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct TypeRef {
        /// You can only have a pointer chain 65535 pointers long.
        ref_depth: u16,
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
    use crate::{key::DefID, typeref::TypeRef};

    pub struct Rule {
        /// What is this Rule for?
        pub operation: (),
        /// What inputs does it take?
        pub inputs: Vec<TypeRef>,
        /// What output does it produce?
        pub output: TypeRef,
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
