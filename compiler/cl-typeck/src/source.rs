//! Holds the [Source] of a definition in the AST

use cl_ast::ast::*;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Source<'a> {
    Root,
    Module(&'a Module),
    Alias(&'a Alias),
    Enum(&'a Enum),
    Variant(&'a Variant),
    Struct(&'a Struct),
    Const(&'a Const),
    Static(&'a Static),
    Function(&'a Function),
    Local(&'a Let),
    Impl(&'a Impl),
    Use(&'a Use),
    Ty(&'a TyKind),
}

impl Source<'_> {
    pub fn name(&self) -> Option<Sym> {
        match self {
            Source::Root => None,
            Source::Module(v) => Some(v.name),
            Source::Alias(v) => Some(v.to),
            Source::Enum(v) => Some(v.name),
            Source::Variant(v) => Some(v.name),
            Source::Struct(v) => Some(v.name),
            Source::Const(v) => Some(v.name),
            Source::Static(v) => Some(v.name),
            Source::Function(v) => Some(v.name),
            Source::Local(l) => Some(l.name),
            Source::Impl(_) | Source::Use(_) | Source::Ty(_) => None,
        }
    }

    /// Returns `true` if this [Source] defines a named value
    pub fn is_named_value(&self) -> bool {
        matches!(self, Self::Const(_) | Self::Static(_) | Self::Function(_))
    }

    /// Returns `true` if this [Source] defines a named type
    pub fn is_named_type(&self) -> bool {
        matches!(
            self,
            Self::Module(_) | Self::Alias(_) | Self::Enum(_) | Self::Struct(_)
        )
    }

    /// Returns `true` if this [Source] refers to a [Ty] with no name
    pub fn is_anon_type(&self) -> bool {
        matches!(self, Self::Ty(_))
    }

    /// Returns `true` if this [Source] refers to an [Impl] block
    pub fn is_impl(&self) -> bool {
        matches!(self, Self::Impl(_))
    }

    /// Returns `true` if this [Source] refers to a [Use] import
    pub fn is_use_import(&self) -> bool {
        matches!(self, Self::Use(_))
    }
}

impl fmt::Display for Source<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Root => "🌳 root 🌳".fmt(f),
            Self::Module(arg0) => arg0.fmt(f),
            Self::Alias(arg0) => arg0.fmt(f),
            Self::Enum(arg0) => arg0.fmt(f),
            Self::Variant(arg0) => arg0.fmt(f),
            Self::Struct(arg0) => arg0.fmt(f),
            Self::Const(arg0) => arg0.fmt(f),
            Self::Static(arg0) => arg0.fmt(f),
            Self::Function(arg0) => arg0.fmt(f),
            Self::Impl(arg0) => arg0.fmt(f),
            Self::Use(arg0) => arg0.fmt(f),
            Self::Ty(arg0) => arg0.fmt(f),
            Self::Local(arg0) => arg0.fmt(f),
        }
    }
}
