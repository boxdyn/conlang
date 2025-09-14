//! Utils for [Path]
use crate::{PathPart, Sym, ast::Path};

impl Path {
    /// Appends a [PathPart] to this [Path]
    pub fn push(&mut self, part: PathPart) {
        self.parts.push(part);
    }
    /// Removes a [PathPart] from this [Path]
    pub fn pop(&mut self) -> Option<PathPart> {
        self.parts.pop()
    }
    /// Concatenates `self::other`. If `other` is an absolute [Path],
    /// this replaces `self` with `other`
    pub fn concat(mut self, other: &Self) -> Self {
        if other.absolute {
            other.clone()
        } else {
            self.parts.extend(other.parts.iter().cloned());
            self
        }
    }

    /// Gets the defining [Sym] of this path
    pub fn as_sym(&self) -> Option<Sym> {
        match self.parts.as_slice() {
            [.., PathPart::Ident(name)] => Some(*name),
            _ => None,
        }
    }

    /// Checks whether this path ends in the given [Sym]
    pub fn ends_with(&self, name: &str) -> bool {
        match self.parts.as_slice() {
            [.., PathPart::Ident(last)] => name == &**last,
            _ => false,
        }
    }

    /// Checks whether this path refers to the sinkhole identifier, `_`
    pub fn is_sinkhole(&self) -> bool {
        if let [PathPart::Ident(id)] = self.parts.as_slice()
            && let "_" = id.to_ref()
        {
            return true;
        }
        false
    }
}
impl PathPart {
    pub fn from_sym(ident: Sym) -> Self {
        Self::Ident(ident)
    }
}

impl From<Sym> for Path {
    fn from(value: Sym) -> Self {
        Self { parts: vec![PathPart::Ident(value)], absolute: false }
    }
}
