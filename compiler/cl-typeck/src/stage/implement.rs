use crate::{handle::Handle, table::Table};

pub fn implement(table: &mut Table) -> Vec<Handle> {
    let pending = std::mem::take(&mut table.pending_impls);
    let mut errors = vec![];
    for node in pending {
        if let Err(e) = impl_one(table, node) {
            errors.push(e);
        }
    }
    errors
}

pub fn impl_one(table: &mut Table, node: Handle) -> Result<(), Handle> {
    let Some(target) = table.impl_target(node) else {
        Err(node)?
    };
    if let Some(children) = table.children.get_mut(&node) {
        let children = children.clone();
        table.children.entry(target).or_default().extend(children);
    }
    Ok(())
}
