//! The [ModuleInliner] reads files described in the module structure of the

use crate::Parser;
use cl_ast::{ast_visitor::Fold, *};
use cl_lexer::Lexer;
use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

type IoErrs = Vec<(PathBuf, std::io::Error)>;
type ParseErrs = Vec<(PathBuf, crate::error::Error)>;

pub struct ModuleInliner {
    path: PathBuf,
    io_errs: IoErrs,
    parse_errs: ParseErrs,
}

impl ModuleInliner {
    /// Creates a new [ModuleInliner]
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            path: root.as_ref().to_path_buf(),
            io_errs: Default::default(),
            parse_errs: Default::default(),
        }
    }
    pub fn inline(mut self, file: File) -> Result<File, (File, IoErrs, ParseErrs)> {
        let file = self.fold_file(file);

        if self.io_errs.is_empty() && self.parse_errs.is_empty() {
            Ok(file)
        } else {
            Err((file, self.io_errs, self.parse_errs))
        }
    }
    /// Records an [I/O error](std::io::Error), and rebuilds the outline [Module]
    fn io_oops(&mut self, name: Identifier, error: std::io::Error) -> Module {
        self.io_errs.push((self.path.clone(), error));
        Module { name, kind: ModuleKind::Outline }
    }
    /// Records a [parse error](crate::error::Error), and rebuilds the outline [Module]
    fn parse_oops(&mut self, name: Identifier, error: crate::error::Error) -> Module {
        self.parse_errs.push((self.path.clone(), error));
        Module { name, kind: ModuleKind::Outline }
    }
}

impl Fold for ModuleInliner {
    fn fold_module(&mut self, m: Module) -> cl_ast::Module {
        let Module { name, kind } = m;
        let mut inliner = PathMiser::new(self, &name.0);

        let kind = inliner.fold_module_kind(kind);

        let ModuleKind::Outline = kind else {
            return Module { name, kind };
        };

        inliner.path.set_extension("cl");
        let file = match std::fs::read_to_string(&inliner.path) {
            Err(e) => return inliner.io_oops(name, e),
            Ok(s) => s,
        };

        let kind = match Parser::new(Lexer::new(&file)).file() {
            Err(e) => return inliner.parse_oops(name, e),
            Ok(file) => ModuleKind::Inline(file),
        };

        inliner.path.set_extension("");
        Module { name, kind: inliner.fold_module_kind(kind) }
    }
}

/// I am the [Path] miser. Whatever I touch turns to [Path] in my clutch.
pub struct PathMiser<'mi> {
    mi: &'mi mut ModuleInliner,
}
impl<'mi> PathMiser<'mi> {
    pub fn new(mi: &'mi mut ModuleInliner, name: &str) -> Self {
        mi.path.push(name);
        Self { mi }
    }
}
impl<'mi> Deref for PathMiser<'mi> {
    type Target = ModuleInliner;
    fn deref(&self) -> &Self::Target {
        self.mi
    }
}
impl<'mi> DerefMut for PathMiser<'mi> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mi
    }
}
impl<'mi> Drop for PathMiser<'mi> {
    fn drop(&mut self) {
        self.mi.path.pop();
    }
}
