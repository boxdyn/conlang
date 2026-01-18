//! The [ModuleInliner] reads files described in the module structure of the

use crate::{ParseError, Parser};
use cl_ast::{
    fold::{Fold, Foldable},
    types::{Literal, Path as AstPath, Symbol},
    *,
};
use cl_lexer::Lexer;
use cl_structures::span::Span;
use std::{
    convert::Infallible,
    path::{Path, PathBuf},
};

pub type IoErrs = Vec<(PathBuf, std::io::Error)>;
pub type ParseErrs = Vec<(PathBuf, ParseError)>;

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

    pub fn fork(&self, path: impl AsRef<Path>) -> Self {
        Self::new(path.as_ref())
    }

    pub fn join(&mut self, other: ModuleInliner) -> &mut Self {
        let ModuleInliner { path: _, io_errs, parse_errs } = other;
        self.io_errs.extend(io_errs);
        self.parse_errs.extend(parse_errs);
        self
    }

    pub fn main_path(&self) -> PathBuf {
        self.path.with_extension("cl")
    }

    pub fn fallback_path(&self) -> Option<PathBuf> {
        let basename = self.path.file_name()?;
        Some(
            self.path
                .parent()?
                .parent()?
                .join(basename)
                .with_added_extension("cl"),
        )
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
    pub fn inline(mut self, expr: Expr) -> Result<Expr, (Expr, IoErrs, ParseErrs)> {
        let Ok(file) = self.fold_expr(expr);

        match self.into_errs() {
            Some((io, parse)) => Err((file, io, parse)),
            None => Ok(file),
        }
    }

    /// Records an [I/O error](std::io::Error) for later
    fn handle_io_error<T>(&mut self, path: PathBuf, error: std::io::Error) -> Option<T> {
        self.io_errs.push((path, error));
        None
    }

    /// Records a [parse error](crate::error::Error) for later
    fn handle_parse_error<T>(&mut self, path: PathBuf, error: ParseError) -> Option<T> {
        self.parse_errs.push((path, error));
        None
    }
}

impl Fold<DefaultTypes> for ModuleInliner {
    type Error = Infallible;

    /// Traverses down the module tree, entering ever nested directories
    fn fold_bind(&mut self, bind: Bind<DefaultTypes>) -> Result<Bind<DefaultTypes>, Self::Error> {
        let Bind(BindOp::Mod, ts, pat, exprs) = bind else {
            return bind.children(self);
        };

        let name = if let Pat::Name(name) = pat {
            name
        } else if let Pat::Value(expr) = &pat
            && let Expr::Lit(Literal::Str(path)) = &expr.as_ref().0
            && let Some(Ok(At(out, span))) = self.inline_file_at(path)
            && let [At(Expr::Omitted, _)] = exprs.as_slice()
        {
            let sym = Path::new(path).with_extension("");
            let sym = sym.file_name().expect("should have filename after load");
            let sym = sym
                .to_str()
                .unwrap_or(path)
                .replace([',', '.', '-', ' '], "_")
                .to_lowercase()
                .as_str()
                .into();
            let expr = Expr::Op(Op::Block, vec![out.at(span)]).at(span);
            return Ok(Bind(BindOp::Mod, ts, Pat::Name(sym), vec![expr]));
        } else {
            return Ok(Bind(BindOp::Mod, ts, pat, exprs));
        };

        self.path.push(name.0); // cd ./name
        let out = if let [At(Expr::Omitted, _)] = exprs.as_slice()
            && let Some(Ok(At(out, span))) = self.inline_file()
        {
            let expr = Expr::Op(Op::Block, vec![out.at(span)]).at(span);
            Ok(Bind(BindOp::Mod, ts, pat, vec![expr]))
        } else {
            Bind(BindOp::Mod, ts, pat, exprs).children(self)
        };

        self.path.pop(); // cd ..
        out
    }

    fn fold_annotation(&mut self, span: Span) -> Result<Span, Self::Error> {
        Ok(span)
    }

    fn fold_macro_id(&mut self, name: Symbol) -> Result<Symbol, Self::Error> {
        Ok(name)
    }

    fn fold_symbol(&mut self, name: Symbol) -> Result<Symbol, Self::Error> {
        Ok(name)
    }

    fn fold_path(&mut self, path: AstPath) -> Result<AstPath, Self::Error> {
        Ok(path)
    }

    fn fold_literal(&mut self, lit: Literal) -> Result<Literal, Self::Error> {
        Ok(lit)
    }
}

impl ModuleInliner {
    fn inline_file(&mut self) -> Option<Result<At<Expr>, Infallible>> {
        // cd path/mod.cl
        let path = self.main_path();
        let used_path = if path.exists() {
            path
        } else {
            self.fallback_path().unwrap_or(path)
        };

        let file = match std::fs::read_to_string(&used_path) {
            Err(e) => return self.handle_io_error(used_path, e),
            Ok(file) => file,
        };

        let path = used_path.display().to_string().as_str().into();

        match Parser::new(Lexer::new(path, &file)).parse(0) {
            Err(e) => self.handle_parse_error(used_path, e),
            Ok(file) => {
                // The newly loaded module may need further inlining
                Some(self.fold_at_expr(file))
            }
        }
    }

    /// Inlines a file at the given `path`,
    fn inline_file_at(&mut self, path: &str) -> Option<Result<At<Expr>, Infallible>> {
        let mut full_path = self.path.clone();
        full_path.push(path);
        let file = match std::fs::read_to_string(&full_path) {
            Err(error) => return self.handle_io_error(full_path, error),
            Ok(file) => file,
        };

        match Parser::new(Lexer::new(path.into(), &file)).parse_entire(0) {
            Err(e) => self.handle_parse_error(full_path, e),
            Ok(file) => {
                let full_path = full_path.with_extension("");
                // The newly loaded module may need further inlining
                let mut mi = self.fork(full_path);
                let out = mi.fold_at_expr(file);
                self.join(mi);
                Some(out)
            }
        }
    }
}
