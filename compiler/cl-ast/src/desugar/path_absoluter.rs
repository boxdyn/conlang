use crate::{ast::*, ast_visitor::Fold};

/// Converts relative paths into absolute paths
pub struct NormalizePaths {
    path: Path,
}

impl NormalizePaths {
    pub fn new() -> Self {
        Self { path: Path { absolute: true, parts: vec![] } }
    }
    /// Normalizes paths as if they came from within the provided paths
    pub fn in_path(path: Path) -> Self {
        Self { path }
    }
}

impl Default for NormalizePaths {
    fn default() -> Self {
        Self::new()
    }
}

impl Fold for NormalizePaths {
    fn fold_module(&mut self, m: Module) -> Module {
        let Module { name, kind } = m;
        self.path.push(PathPart::Ident(name));
        
        let name = self.fold_sym(name);
        let kind = kind.map(|f| self.fold_file(f));

        self.path.pop();
        Module { name, kind }
    }

    fn fold_path(&mut self, p: Path) -> Path {
        if p.absolute {
            p
        } else {
            self.path.clone().concat(&p)
        }
    }

    fn fold_use(&mut self, u: Use) -> Use {
        let Use { absolute, mut tree } = u;

        if !absolute {
            for segment in self.path.parts.iter().rev() {
                tree = UseTree::Path(segment.clone(), Box::new(tree))
            }
        }

        Use { absolute: true, tree: self.fold_use_tree(tree) }
    }
}
