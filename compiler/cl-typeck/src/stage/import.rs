//! An algorithm for importing external nodes

use crate::{
    handle::Handle,
    source::Source,
    table::{NodeKind, Table},
};
use cl_ast::{Use, types::Symbol};
use core::slice;
use std::{collections::HashSet, mem};

type Seen = HashSet<Handle>;

pub fn import<'a>(table: &mut Table<'a>) -> Vec<(Handle, Error<'a>)> {
    let pending = mem::take(&mut table.uses);

    let mut seen = Seen::new();
    let mut failed = vec![];
    for import in pending {
        let Err(e) = import_one(table, import, &mut seen) else {
            continue;
        };
        if let Error::NotFound(_, _) = e {
            table.mark_use_item(import)
        }
        failed.push((import, e));
    }
    failed
}

fn import_one<'a>(table: &mut Table<'a>, item: Handle, seen: &mut Seen) -> UseResult<'a, ()> {
    if !seen.insert(item) {
        return Ok(());
    }

    let Some(NodeKind::Use) = table.kind(item) else {
        Err(Error::ItsNoUse)?
    };
    let Some(&dst) = table.parent(item) else {
        Err(Error::NoParents)?
    };
    let Some(code) = table.source(item) else {
        Err(Error::NoSource)?
    };
    let &Source::Use(tree) = code else {
        Err(Error::BadSource(*code))?
    };

    import_tree(table, dst, dst, tree, seen)
}

fn import_tree<'a>(
    table: &mut Table<'a>,
    src: Handle,
    dst: Handle,
    tree: &Use,
    seen: &mut Seen,
) -> UseResult<'a, ()> {
    match tree {
        Use::Tree(trees) => trees
            .iter()
            .try_for_each(|tree| import_tree(table, src, dst, tree, seen)),
        Use::Path(part, rest) => {
            let source = table
                .nav(src, slice::from_ref(part))
                .ok_or(Error::NotFound(src, *part))?;
            import_tree(table, source, dst, rest, seen)
        }
        Use::Alias(src_name, dst_name) => import_name(table, src, src_name, dst, dst_name, seen),
        Use::Name(src_name) => import_name(table, src, src_name, dst, src_name, seen),
        Use::Glob => import_glob(table, src, dst, seen),
    }
}

fn import_glob<'a>(
    table: &mut Table<'a>,
    src: Handle,
    dst: Handle,
    seen: &mut Seen,
) -> UseResult<'a, ()> {
    let Table { children, imports, .. } = table;

    if let Some(c) = children.get(&src) {
        imports.entry(dst).or_default().extend(c)
    }

    import_deps(table, src, seen)?;

    let Table { imports, .. } = table;

    // Importing imports requires some extra work, since we can't `get_many_mut`
    if let Some(i) = imports.get(&src) {
        let uses: Vec<_> = i.iter().map(|(&k, &v)| (k, v)).collect();
        imports.entry(dst).or_default().extend(uses);
    }

    Ok(())
}

fn import_name<'a>(
    table: &mut Table<'a>,
    src: Handle,
    src_name: &Symbol,
    dst: Handle,
    dst_name: &Symbol,
    seen: &mut Seen,
) -> UseResult<'a, ()> {
    import_deps(table, src, seen)?;
    match table.get_by_sym(src, src_name) {
        // TODO: check for new imports clobbering existing imports
        Some(src_id) => table.add_import(dst, *dst_name, src_id),
        None => Err(Error::NotFound(src, *src_name))?,
    };
    Ok(())
}

/// Imports the dependencies of this node
fn import_deps<'a>(table: &mut Table<'a>, node: Handle, seen: &mut Seen) -> UseResult<'a, ()> {
    if let Some(items) = table.use_items.get(&node) {
        let out = items.clone();
        for item in out {
            import_one(table, item, seen)?;
        }
    }
    Ok(())
}

pub type UseResult<'a, T> = Result<T, Error<'a>>;

#[derive(Debug)]
pub enum Error<'a> {
    ItsNoUse,
    NoParents,
    NoSource,
    BadSource(Source<'a>),
    NotFound(Handle, Symbol),
}

impl std::fmt::Display for Error<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ItsNoUse => write!(f, "Entry is not use"),
            Error::NoParents => write!(f, "Entry has no parents"),
            Error::NoSource => write!(f, "Entry has no source"),
            Error::BadSource(s) => write!(f, "Entry incorrectly marked as use item: {s}"),
            Error::NotFound(id, part) => write!(f, "Could not traverse {id}::{part}"),
        }
    }
}
