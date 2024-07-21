//! A [Path] is a borrowed view of an [AST Path](AstPath)
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
    pub fn front(self) -> Option<Self> {
        let Self { absolute, parts } = self;
        Some(Self { absolute, parts: parts.get(..1)? })
    }
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }
    pub fn len(&self) -> usize {
        self.parts.len()
    }
    pub fn first(&self) -> Option<&PathPart> {
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
