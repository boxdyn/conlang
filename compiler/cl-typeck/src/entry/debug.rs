//! [std::fmt::Debug] implementation for [Entry]

use super::Entry;

impl std::fmt::Debug for Entry<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // virtual fields
        let mut ds = f.debug_struct("Entry");
        if let Some(name) = self.name() {
            ds.field("name", &name.to_ref());
        }
        ds.field("kind", &self.kind());
        if let Some(ty) = self.ty() {
            ds.field("type", ty);
        }
        if let Some(meta) = self.meta() {
            ds.field("meta", &meta);
        }
        if let Some(children) = self.children() {
            ds.field("children", children);
        }
        if let Some(imports) = self.lazy_imports() {
            ds.field("lazy_imports", imports);
        }
        if let Some(imports) = self.glob_imports() {
            ds.field("glob_imports", &imports);
        }
        ds.field("implements", &self.impl_target()).finish()
    }
}
