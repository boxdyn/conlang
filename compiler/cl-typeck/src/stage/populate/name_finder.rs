use cl_ast::{
    ast::*,
    types::Symbol,
    visit::{Visit, Walk},
};
use std::convert::Infallible;

/// Finds the first name mentioned anywhere in a syntax tree
#[derive(Clone, Debug, Default)]
pub struct NameFinder {
    name: Option<Symbol>,
}

impl NameFinder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get<'a, T: Walk<'a, DefaultTypes>>(walker: &'a T) -> Option<Symbol> {
        let mut finder = Self::new();
        finder.visit(walker);
        finder.name
    }
}

impl<'a> Visit<'a, DefaultTypes> for NameFinder {
    type Error = Infallible;

    fn visit_symbol(&mut self, name: &'a Symbol) -> Result<(), Self::Error> {
        if self.name.is_none() {
            self.name = Some(*name);
        }
        Ok(())
    }
}
