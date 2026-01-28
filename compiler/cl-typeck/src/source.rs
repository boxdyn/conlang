//! Holds the [Source] of a definition in the AST

use cl_ast::{
    ast::*,
    types::Symbol,
    visit::{Visit, Walk},
};
use std::{convert::Infallible, fmt};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Root,
    Binding(Bind),
    Use(Use),
    Ty(Pat),
}

impl Source {
    pub fn name(&self) -> Option<Symbol> {
        match self {
            Source::Root => None,
            Source::Binding(bind) => NameFinder::get(bind),
            Source::Use(_) | Source::Ty(_) => None,
        }
    }

    /// Returns `true` if this [Source] defines a named value
    pub fn is_named_value(&self) -> bool {
        matches!(self, Self::Binding(Bind(BindOp::Let | BindOp::Fn, ..)))
    }

    /// Returns `true` if this [Source] defines a named type
    pub fn is_named_type(&self) -> bool {
        matches!(
            self,
            Self::Binding(Bind(
                BindOp::Type | BindOp::Struct | BindOp::Enum | BindOp::Mod,
                ..
            ))
        )
    }

    /// Returns `true` if this [Source] refers to a [Ty] with no name
    pub fn is_anon_type(&self) -> bool {
        matches!(self, Self::Ty(_))
    }

    /// Returns `true` if this [Source] refers to an [Impl] block
    pub fn is_impl(&self) -> bool {
        matches!(self, Self::Binding(Bind(BindOp::Impl, ..)))
    }

    /// Returns `true` if this [Source] refers to a [Use] import
    pub fn is_use_import(&self) -> bool {
        matches!(self, Self::Use(_))
    }
}

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

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Root => "🌳 root 🌳".fmt(f),
            Self::Binding(arg0) => arg0.fmt(f),
            Self::Use(arg0) => arg0.fmt(f),
            Self::Ty(arg0) => arg0.fmt(f),
        }
    }
}
