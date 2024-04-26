//! A [Module] is a node in the Module Tree (a component of a
//! [Project](crate::project::Project))
use cl_ast::Sym;
use cl_structures::deprecated_intern_pool::InternKey;

use crate::key::DefID;
use std::collections::HashMap;

/// A [Module] is a node in the Module Tree (a component of a
/// [Project](crate::project::Project)).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Module {
    pub parent: Option<DefID>,
    pub types: HashMap<Sym, DefID>,
    pub values: HashMap<Sym, DefID>,
    pub imports: Vec<DefID>,
}

impl Module {
    pub fn new(parent: DefID) -> Self {
        Self { parent: Some(parent), ..Default::default() }
    }
    pub fn with_optional_parent(parent: Option<DefID>) -> Self {
        Self { parent, ..Default::default() }
    }

    pub fn get(&self, name: Sym) -> (Option<DefID>, Option<DefID>) {
        (self.get_type(name), self.get_value(name))
    }
    pub fn get_type(&self, name: Sym) -> Option<DefID> {
        self.types.get(&name).copied()
    }
    pub fn get_value(&self, name: Sym) -> Option<DefID> {
        self.values.get(&name).copied()
    }

    /// Inserts a type with the provided [name](str) and [id](DefID)
    pub fn insert_type(&mut self, name: Sym, id: DefID) -> Option<DefID> {
        self.types.insert(name, id)
    }

    /// Inserts a value with the provided [name](str) and [id](DefID)
    pub fn insert_value(&mut self, name: Sym, id: DefID) -> Option<DefID> {
        self.values.insert(name, id)
    }
}

impl std::fmt::Display for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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
