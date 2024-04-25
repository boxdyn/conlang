//! WIP use-item importer. This performs eager import resolution on the AST
//!
//! # TODOs:
//! - [ ] Resolve imports using a graph traversal rather than linear iteration
//! - [ ] Separate imported items from natively declared items
//! - [ ] Separate the use-import pass from the project
//! - [ ] Report errors in a meaningful way
//! - [ ] Lazy import resolution using graph-edge traversal during name lookup?
//!     - It doesn't seem to me like the imports in a given scope *can change*.

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

    pub fn visit_use_leaf(&mut self, part: &'a Identifier, parent: DefID, c: DefID) -> UseResult {
        let Identifier(name) = part;
        self.visit_use_alias(name, name, parent, c)
    }

    pub fn visit_use_alias(
        &mut self,
        from: &Sym,
        name: &Sym,
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
            parent.types.insert(*name, tid);
            imported = true;
        }

        if let Some(vid) = vid {
            parent.values.insert(*name, vid);
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
