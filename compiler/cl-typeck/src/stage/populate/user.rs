//! The [User] imports [Use] items into a [Table](crate::table::Table)'s
//! lazy_imports and glob_imports tables.

use crate::{list::List, table::SymMap};
use cl_ast::{
    Use,
    types::{Path, Symbol},
};

/// Imports [Use] items into a module by their path
pub struct User<'parent> {
    path: List<'parent, Symbol>,
    imports: &'parent mut SymMap<Path>,
    globs: &'parent mut Vec<Path>,
}

impl<'parent> User<'parent> {
    pub fn new(
        path: List<'parent, Symbol>,
        imports: &'parent mut SymMap<Path>,
        globs: &'parent mut Vec<Path>,
    ) -> Self {
        Self { path, imports, globs }
    }

    pub fn visit_use(&mut self, item: &'parent Use) {
        let Self { path, imports, globs } = self;
        match item {
            Use::Glob => {
                globs.push((*path).into());
            }
            &Use::Name(name) => {
                let path: Path = path.enter(name).into();
                imports.insert(name, path);
            }
            &Use::Alias(name, alias) => {
                let path: Path = path.enter(name).into();
                imports.insert(alias, path);
            }
            Use::Path(name, rest) => {
                User { path: path.enter(*name), imports, globs }.visit_use(rest);
            }
            Use::Tree(items) => {
                items.iter().for_each(|item| self.visit_use(item));
            }
        }
    }
}
