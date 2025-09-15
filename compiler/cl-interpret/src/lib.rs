//! Walks a Conlang AST, interpreting it as a program.
#![warn(clippy::all)]
#![feature(decl_macro)]

use cl_ast::Sym;
use convalue::ConValue;
use env::Environment;
use error::{Error, ErrorKind, IResult};
use interpret::Interpret;

/// Callable types can be called from within a Conlang program
pub trait Callable {
    /// Calls this [Callable] in the provided [Environment], with [ConValue] args  \
    /// The Callable is responsible for checking the argument count and validating types
    fn call(&self, interpreter: &mut Environment, args: &[ConValue]) -> IResult<ConValue>;
    /// Returns the common name of this identifier.
    fn name(&self) -> Sym;
}

pub mod convalue;

pub mod interpret;

pub mod function;

pub mod constructor {
    use cl_ast::Sym;

    use crate::{
        Callable,
        convalue::ConValue,
        env::Environment,
        error::{Error, IResult},
    };

    #[derive(Clone, Copy, Debug)]
    pub struct Constructor {
        pub name: Sym,
        pub arity: u32,
    }

    impl Callable for Constructor {
        fn call(&self, _env: &mut Environment, args: &[ConValue]) -> IResult<ConValue> {
            let &Self { name, arity } = self;
            if arity as usize == args.len() {
                Ok(ConValue::TupleStruct(Box::new((
                    name.to_ref(),
                    args.into(),
                ))))
            } else {
                Err(Error::ArgNumber(arity as usize, args.len()))
            }
        }

        fn name(&self) -> cl_ast::Sym {
            "tuple-constructor".into()
        }
    }
}

pub mod closure;

pub mod builtin;

pub mod pattern;

pub mod env;

pub mod modules {
    use crate::env::StackBinds;
    use cl_ast::{PathPart, Sym};
    use std::collections::HashMap;

    /// Immutable object-oriented interface to a [ModuleTree]
    #[derive(Clone, Copy, Debug)]
    pub struct ModuleNode<'tree> {
        tree: &'tree ModuleTree,
        index: usize,
    }

    /// Mutable object-oriented interface to a [ModuleTree]
    #[derive(Debug)]
    pub struct ModuleNodeMut<'tree> {
        tree: &'tree mut ModuleTree,
        index: usize,
    }

    macro_rules! module_node_impl {
        () => {
            /// Gets the index from this node
            pub fn index(self) -> usize {
                self.index
            }
            /// Gets this node's parent
            pub fn parent(self) -> Option<Self> {
                let parent = self.tree.parent(self.index)?;
                Some(Self { index: parent, ..self })
            }
            /// Gets the node's "encompassing Type"
            pub fn selfty(self) -> Option<Self> {
                let selfty = self.tree.selfty(self.index)?;
                Some(Self { index: selfty, ..self })
            }
            /// Gets the child of this node with the given name
            pub fn child(self, name: &Sym) -> Option<Self> {
                let child = self.tree.child(self.index, name)?;
                Some(Self { index: child, ..self })
            }
            /// Gets a stack value in this node with the given name
            pub fn item(self, name: &Sym) -> Option<usize> {
                self.tree.items(self.index)?.get(name).copied()
            }
            /// Returns true when this node represents type information
            pub fn is_ty(self) -> Option<bool> {
                self.tree.is_ty.get(self.index).copied()
            }
            /// Returns a reference to this node's children, if present
            pub fn children(&self) -> Option<&HashMap<Sym, usize>> {
                self.tree.children(self.index)
            }
            /// Returns a reference to this node's items, if present
            pub fn items(&self) -> Option<&StackBinds> {
                self.tree.items(self.index)
            }
            /// Traverses a path starting at this node
            ///
            /// Returns a new node, and the unconsumed path portion.
            pub fn find(self, path: &[PathPart]) -> (Self, &[PathPart]) {
                let (index, path) = self.tree.find(self.index, path);
                (Self { index, ..self }, path)
            }
            /// Traverses a path starting at this node
            ///
            /// Returns an item address if the path terminated in an item.
            pub fn find_item(&self, path: &[PathPart]) -> Option<usize> {
                self.tree.find_item(self.index, path)
            }
        };
    }

    impl ModuleNode<'_> {
        module_node_impl! {}
    }

    impl ModuleNodeMut<'_> {
        module_node_impl! {}
        /// Creates a new child in this node
        pub fn add_child(self, name: Sym, is_ty: bool) -> Self {
            let node = self.tree.add_child(self.index, name, is_ty);
            self.tree.get_mut(node)
        }
        /// Creates an arbitrary edge in the module graph
        pub fn add_import(&mut self, name: Sym, child: usize) {
            self.tree.add_import(self.index, name, child)
        }
        pub fn add_imports(&mut self, binds: HashMap<Sym, usize>) {
            self.tree.add_imports(self.index, binds)
        }
        /// Binds a new item in this node
        pub fn add_item(&mut self, name: Sym, stack_index: usize) {
            self.tree.add_item(self.index, name, stack_index)
        }
        /// Binds an entire stack frame in this node
        pub fn add_items(&mut self, binds: StackBinds) {
            self.tree.add_items(self.index, binds)
        }
        /// Constructs a borrowing [ModuleNode]
        pub fn as_ref(&self) -> ModuleNode<'_> {
            let Self { tree, index } = self;
            ModuleNode { tree, index: *index }
        }
    }

    #[derive(Clone, Debug)]
    pub struct ModuleTree {
        parents: Vec<usize>,
        children: Vec<HashMap<Sym, usize>>,
        items: Vec<StackBinds>,
        is_ty: Vec<bool>,
    }

    impl ModuleTree {
        /// Constructs a new ModuleTree with a single root module
        pub fn new() -> Self {
            Self {
                parents: vec![0],
                children: vec![HashMap::new()],
                items: vec![HashMap::new()],
                is_ty: vec![false],
            }
        }

        /// Gets a borrowed handle to the node at `index`
        pub fn get(&self, index: usize) -> ModuleNode<'_> {
            ModuleNode { tree: self, index }
        }

        /// Gets a mutable handle to the node at `index`
        pub fn get_mut(&mut self, index: usize) -> ModuleNodeMut<'_> {
            ModuleNodeMut { tree: self, index }
        }

        /// Creates a new child in this node
        pub fn add_child(&mut self, parent: usize, name: Sym, is_ty: bool) -> usize {
            let index = self.parents.len();
            self.children[parent].insert(name, index);
            self.parents.push(parent);
            self.children.push(HashMap::new());
            self.is_ty.push(is_ty);
            index
        }

        /// Binds a new item in this node
        pub fn add_item(&mut self, node: usize, name: Sym, stack_index: usize) {
            self.items[node].insert(name, stack_index);
        }

        /// Creates an arbitrary child edge
        pub fn add_import(&mut self, parent: usize, name: Sym, child: usize) {
            self.children[parent].insert(name, child);
        }

        /// Binds an entire stack frame in this node
        pub fn add_items(&mut self, node: usize, binds: StackBinds) {
            self.items[node].extend(binds);
        }

        /// Binds an arbitrary set of child edges
        pub fn add_imports(&mut self, node: usize, binds: HashMap<Sym, usize>) {
            self.children[node].extend(binds);
        }

        /// Gets this node's parent
        pub fn parent(&self, node: usize) -> Option<usize> {
            if node == 0 {
                return None;
            }
            self.parents.get(node).copied()
        }

        /// Gets the node's "encompassing Type"
        pub fn selfty(&self, node: usize) -> Option<usize> {
            if self.is_ty[node] {
                return Some(node);
            }
            self.selfty(self.parent(node)?)
        }

        /// Gets the child of this node with the given name
        pub fn child(&self, node: usize, id: &Sym) -> Option<usize> {
            self.children[node].get(id).copied()
        }

        /// Gets a stack value in this node with the given name
        pub fn item(&self, node: usize, name: &Sym) -> Option<usize> {
            self.items.get(node).and_then(|map| map.get(name).copied())
        }

        /// Returns a reference to this node's children, if present
        pub fn children(&self, node: usize) -> Option<&HashMap<Sym, usize>> {
            self.children.get(node)
        }

        /// Returns a reference to this node's items, if present
        pub fn items(&self, node: usize) -> Option<&StackBinds> {
            self.items.get(node)
        }

        /// Traverses a path starting at this node
        ///
        /// Returns a new node, and the unconsumed path portion.
        pub fn find<'p>(&self, node: usize, path: &'p [PathPart]) -> (usize, &'p [PathPart]) {
            match path {
                [PathPart::SuperKw, tail @ ..] => match self.parent(node) {
                    Some(node) => self.find(node, tail),
                    None => (node, path),
                },
                [PathPart::Ident(name), tail @ ..] => match self.child(node, name) {
                    Some(node) => self.find(node, tail),
                    None => (node, path),
                },
                [PathPart::SelfTy, tail @ ..] => match self.selfty(node) {
                    Some(node) => self.find(node, tail),
                    None => (node, path),
                },
                [] => (node, path),
            }
        }

        /// Traverses a path starting at this node
        ///
        /// Returns an item address if the path terminated in an item.
        pub fn find_item(&self, node: usize, path: &[PathPart]) -> Option<usize> {
            let (node, [PathPart::Ident(name)]) = self.find(node, path) else {
                return None;
            };
            self.item(node, name)
        }
    }

    impl Default for ModuleTree {
        fn default() -> Self {
            Self::new()
        }
    }
}

pub mod collector {
    use std::ops::{Deref, DerefMut};

    use crate::{
        convalue::ConValue,
        env::Environment,
        modules::{ModuleNode, ModuleNodeMut},
    };
    use cl_ast::{
        ast_visitor::{Visit, Walk},
        *,
    };

    pub struct Collector<'env> {
        module: usize,
        env: &'env mut Environment,
    }

    impl Collector<'_> {
        pub fn as_node(&self) -> ModuleNode<'_> {
            self.env.modules().get(self.module)
        }

        pub fn as_node_mut(&mut self) -> ModuleNodeMut<'_> {
            self.env.modules_mut().get_mut(self.module)
        }

        pub fn scope(&mut self, name: Sym, is_ty: bool, f: impl Fn(&mut Collector<'_>)) {
            let module = match self.as_node_mut().child(&name) {
                Some(m) => m,
                None => self.as_node_mut().add_child(name, is_ty),
            }
            .index();

            let mut frame = self.env.frame(name.to_ref());
            f(&mut Collector { env: &mut frame, module });
            let binds = frame.into_binds().unwrap_or_default();

            self.modules_mut().add_items(module, binds);
        }

        pub fn in_foreign_scope<F, T>(&mut self, path: &[PathPart], f: F) -> Option<T>
        where F: Fn(&mut Collector<'_>) -> T {
            let (module, []) = self.env.modules_mut().find(self.module, path) else {
                return None;
            };

            let mut frame = self.env.frame("impl");
            let out = f(&mut Collector { env: &mut frame, module });
            let binds = frame.into_binds().unwrap_or_default();

            self.env.modules_mut().add_items(module, binds);
            Some(out)
        }
    }

    impl<'env> Deref for Collector<'env> {
        type Target = Environment;
        fn deref(&self) -> &Self::Target {
            self.env
        }
    }

    impl DerefMut for Collector<'_> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.env
        }
    }

    impl<'a, 'env> Visit<'a> for Collector<'env> {
        fn visit_file(&mut self, value: &'a File) {
            let mut sorter = ItemSorter::default();
            sorter.visit(value);
            sorter.visit_all(self);
        }
        fn visit_block(&mut self, value: &'a Block) {
            let mut sorter = ItemSorter::default();
            sorter.visit(value);
            sorter.visit_all(self);
        }
        fn visit_module(&mut self, value: &'a cl_ast::Module) {
            self.scope(value.name, false, |scope| value.children(scope));
        }
        fn visit_alias(&mut self, value: &'a cl_ast::Alias) {
            let Alias { name, from } = value;
            match from.as_ref().map(Box::as_ref) {
                Some(Ty { kind: TyKind::Path(path), .. }) => {
                    let mut node = if path.absolute {
                        self.modules_mut().get_mut(0)
                    } else {
                        self.as_node_mut()
                    };
                    if let Some(item) = node.find_item(&path.parts) {
                        node.add_item(*name, item);
                    }
                }
                Some(other) => todo!("Type expressions in the collector: {other}"),
                None => self.scope(*name, true, |_| {}),
            }
        }
        fn visit_enum(&mut self, value: &'a cl_ast::Enum) {
            let Enum { name, gens: _, variants } = value;

            self.scope(*name, true, |frame| {
                for (idx, Variant { name, kind, body }) in variants.iter().enumerate() {
                    frame.visit(body);
                    frame.scope(*name, false, |frame| {
                        frame.bind("__discriminant", idx as isize);
                        match kind {
                            StructKind::Empty => {
                                frame.insert_tup_constructor("call".into(), 0);
                                frame.bind("__nmemb", ConValue::Int(0));
                            }
                            StructKind::Tuple(args) => {
                                // Constructs the AST from scratch. TODO: This, better.
                                frame.insert_tup_constructor("call".into(), args.len());
                                frame.bind("__nmemb", ConValue::Int(args.len() as _));
                            }
                            StructKind::Struct(members) => {
                                // TODO: more precise type checking of structs
                                for (idx, memb) in members.iter().enumerate() {
                                    let StructMember { vis: _, name, ty: _ } = memb;
                                    frame.bind(*name, idx as isize);
                                }
                                frame.bind("__nmemb", ConValue::Int(members.len() as _));
                            }
                        }
                    });
                }
            });
        }
        fn visit_struct(&mut self, value: &'a cl_ast::Struct) {
            let Struct { name, gens: _, kind } = value;

            self.scope(*name, true, |frame| {
                match kind {
                    StructKind::Empty => {
                        frame.insert_tup_constructor("call".into(), 0);
                        frame.bind("__nmemb", ConValue::Int(0));
                    }
                    StructKind::Tuple(args) => {
                        // Constructs the AST from scratch. TODO: This, better.
                        frame.insert_tup_constructor("call".into(), args.len());
                        frame.bind("__nmemb", ConValue::Int(args.len() as _));
                    }
                    StructKind::Struct(members) => {
                        // TODO: more precise type checking of structs
                        for (idx, memb) in members.iter().enumerate() {
                            let StructMember { vis: _, name, ty: _ } = memb;
                            frame.bind(*name, idx as isize);
                        }
                        frame.bind("__nmemb", ConValue::Int(members.len() as _));
                    }
                }
            });
        }
        fn visit_const(&mut self, value: &'a cl_ast::Const) {
            let Const { name, ty: _, init } = value;
            self.visit(init);
            self.bind(*name, ());
        }
        fn visit_static(&mut self, value: &'a cl_ast::Static) {
            let Static { mutable: _, name, ty: _, init } = value;
            self.visit(init);
            self.bind(*name, ());
        }
        fn visit_function(&mut self, value: &'a cl_ast::Function) {
            let Function { name, gens: _, sign: _, bind: _, body } = value;
            self.scope(*name, false, |scope| {
                scope.visit(body);
                let f = crate::function::Function::new(value);
                scope.bind("call", f);
            });
        }
        fn visit_impl(&mut self, value: &'a cl_ast::Impl) {
            let Impl { gens: _, target: ImplKind::Type(Ty { kind: TyKind::Path(name), .. }), body } =
                value
            else {
                eprintln!("TODO: impl X for Ty");
                return;
            };
            self.in_foreign_scope(&name.parts, |scope| {
                body.visit_in(scope);
            });
        }
        fn visit_use(&mut self, value: &'a cl_ast::Use) {
            fn traverse(dest: &mut Collector<'_>, node: usize, tree: &UseTree) {
                match tree {
                    UseTree::Tree(ts) => ts.iter().for_each(|tree| traverse(dest, node, tree)),
                    UseTree::Path(PathPart::Ident(name), tree) => {
                        if let (node, []) = dest.modules().find(node, &[PathPart::Ident(*name)]) {
                            traverse(dest, node, tree)
                        }
                    }
                    UseTree::Path(PathPart::SuperKw, tree) => {
                        if let Some(node) = dest.modules().parent(node) {
                            traverse(dest, node, tree)
                        }
                    }
                    UseTree::Path(PathPart::SelfTy, tree) => {
                        if let Some(node) = dest.modules().selfty(node) {
                            traverse(dest, node, tree)
                        }
                    }
                    UseTree::Alias(name, as_name) => {
                        if let Some(child) = dest.modules().child(node, name) {
                            dest.as_node_mut().add_import(*as_name, child);
                        }
                        if let Some(item) = dest.modules().item(node, name) {
                            dest.as_node_mut().add_item(*as_name, item);
                        }
                    }
                    UseTree::Name(name) => {
                        if let Some(child) = dest.modules().child(node, name) {
                            dest.as_node_mut().add_import(*name, child);
                        }
                        if let Some(item) = dest.modules().item(node, name) {
                            dest.as_node_mut().add_item(*name, item);
                        }
                    }
                    UseTree::Glob => {
                        let &mut Collector { module, ref mut env } = dest;
                        if let Some(children) = env.modules().children(node) {
                            for (name, index) in children.clone() {
                                env.modules_mut().add_import(module, name, index);
                            }
                        }
                        if let Some(items) = env.modules().items(node).cloned() {
                            env.modules_mut().add_items(node, items);
                        }
                    }
                }
            }

            let Use { absolute, tree } = value;
            let node = if *absolute { 0 } else { self.module };
            traverse(self, node, tree);
        }
    }

    // fn make_tuple_constructor(name: Sym, args: &[Ty]) -> ConValue {
    //     let span = match (
    //         args.first().map(|a| a.span.head),
    //         args.last().map(|a| a.span.tail),
    //     ) {
    //         (Some(head), Some(tail)) => Span(head, tail),
    //         _ => Span::dummy(),
    //     };

    //     let constructor = Function {
    //         name,
    //         gens: Default::default(),
    //         sign: TyFn {
    //             args: Ty { kind: TyKind::Tuple(TyTuple { types: args.to_vec() }), span }.into(),
    //             rety: Some(Ty { span: Span::dummy(), kind: TyKind::Path(Path::from(name))
    // }.into()),         },
    //         bind: Pattern::Tuple(
    //             args.iter()
    //                 .enumerate()
    //                 .map(|(idx, _)| Pattern::Name(idx.to_string().into()))
    //                 .collect(),
    //         ),
    //         body: None,
    //     };
    //     // ConValue::TupleConstructor(crate::constructor::Constructor {ind})
    //     todo!("Tuple constructor {constructor}")
    // }

    /// Sorts items
    #[derive(Debug, Default)]
    struct ItemSorter<'ast> {
        modules: Vec<&'ast Module>,
        structs: Vec<&'ast Struct>,
        enums: Vec<&'ast Enum>,
        aliases: Vec<&'ast Alias>,
        consts: Vec<&'ast Const>,
        statics: Vec<&'ast Static>,
        functions: Vec<&'ast Function>,
        impls: Vec<&'ast Impl>,
        imports: Vec<&'ast Use>,
    }

    impl<'a> ItemSorter<'a> {
        fn visit_all<V: Visit<'a>>(&self, v: &mut V) {
            let Self {
                modules,
                aliases,
                enums,
                structs,
                consts,
                statics,
                functions,
                impls,
                imports,
            } = self;

            // 0
            for item in modules {
                item.visit_in(v);
            }
            // 1
            for item in structs {
                item.visit_in(v);
            }
            for item in enums {
                item.visit_in(v);
            }
            for item in aliases {
                item.visit_in(v);
            }
            // 2
            // 5
            for item in consts {
                item.visit_in(v);
            }
            for item in statics {
                item.visit_in(v);
            }
            for item in functions {
                item.visit_in(v);
            }
            // 4
            for item in impls {
                item.visit_in(v);
            }
            // 3
            for item in imports {
                item.visit_in(v);
            }
        }
    }

    impl<'a> Visit<'a> for ItemSorter<'a> {
        fn visit_module(&mut self, value: &'a cl_ast::Module) {
            self.modules.push(value);
        }
        fn visit_alias(&mut self, value: &'a cl_ast::Alias) {
            self.aliases.push(value);
        }
        fn visit_enum(&mut self, value: &'a cl_ast::Enum) {
            self.enums.push(value);
        }
        fn visit_struct(&mut self, value: &'a cl_ast::Struct) {
            self.structs.push(value);
        }
        fn visit_const(&mut self, value: &'a cl_ast::Const) {
            self.consts.push(value);
        }
        fn visit_static(&mut self, value: &'a cl_ast::Static) {
            self.statics.push(value);
        }
        fn visit_function(&mut self, value: &'a cl_ast::Function) {
            self.functions.push(value);
        }
        fn visit_impl(&mut self, value: &'a cl_ast::Impl) {
            self.impls.push(value);
        }
        fn visit_use(&mut self, value: &'a cl_ast::Use) {
            self.imports.push(value);
        }
    }
}

pub mod error;

#[cfg(test)]
mod tests;
