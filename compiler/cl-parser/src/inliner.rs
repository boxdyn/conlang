//! The [ModuleInliner] reads files described in the module structure of the

use crate::Parser;
use cl_ast::{ast_visitor::Fold, *};
use cl_lexer::Lexer;
use std::path::{Path, PathBuf};

pub type IoErrs = Vec<(PathBuf, std::io::Error)>;
pub type ParseErrs = Vec<(PathBuf, crate::error::Error)>;

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

    /// Returns true when the [ModuleInliner] has errors to report
    pub fn has_errors(&self) -> bool {
        !(self.io_errs.is_empty() && self.parse_errs.is_empty())
    }

    /// Returns the [IO Errors](IoErrs) and [parse Errors](ParseErrs)
    pub fn into_errs(self) -> Option<(IoErrs, ParseErrs)> {
        self.has_errors().then_some((self.io_errs, self.parse_errs))
    }

    /// Traverses a [File], attempting to inline all submodules.
    ///
    /// This is a simple wrapper around [ModuleInliner::fold_file()] and
    /// [ModuleInliner::into_errs()]
    pub fn inline(mut self, file: File) -> Result<File, (File, IoErrs, ParseErrs)> {
        let file = self.fold_file(file);

        match self.into_errs() {
            Some((io, parse)) => Err((file, io, parse)),
            None => Ok(file),
        }
    }

    /// Records an [I/O error](std::io::Error) for later
    fn handle_io_error(&mut self, error: std::io::Error) -> ModuleKind {
        self.io_errs.push((self.path.clone(), error));
        ModuleKind::Outline
    }

    /// Records a [parse error](crate::error::Error) for later
    fn handle_parse_error(&mut self, error: crate::error::Error) -> ModuleKind {
        self.parse_errs.push((self.path.clone(), error));
        ModuleKind::Outline
    }
}

impl Fold for ModuleInliner {
    /// Traverses down the module tree, entering ever nested directories
    fn fold_module(&mut self, m: Module) -> Module {
        let Module { name, kind } = m;
        self.path.push(&*name); // cd ./name

        let kind = self.fold_module_kind(kind);

        self.path.pop(); // cd ..
        Module { name, kind }
    }

    /// Attempts to read and parse a file for every module in the tree
    fn fold_module_kind(&mut self, m: ModuleKind) -> ModuleKind {
        if let ModuleKind::Inline(f) = m {
            return ModuleKind::Inline(self.fold_file(f));
        }
        // cd path/mod.cl
        self.path.set_extension("cl");

        let file = match std::fs::read_to_string(&self.path) {
            Err(error) => return self.handle_io_error(error),
            Ok(file) => file,
        };

        let kind = match Parser::new(Lexer::new(&file)).file() {
            Err(e) => return self.handle_parse_error(e),
            Ok(file) => ModuleKind::Inline(file),
        };
        // cd path/mod
        self.path.set_extension("");

        // The newly loaded module may need further inlining
        self.fold_module_kind(kind)
    }
}
