//! [std::fmt::Debug] implementation for [Entry]

use super::Entry;

impl std::fmt::Debug for Entry<'_, '_> {
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
        if let Some(body) = self.bodies() {
            ds.field("body", body);
        }
        if let Some(children) = self.children() {
            ds.field("children", children);
        }
        if let Some(imports) = self.imports() {
            ds.field("imports", imports);
        }
        // if let Some(source) = self.source() {
        //     ds.field("source", source);
        // }
        ds.field("implements", &self.impl_target()).finish()
    }
}
