//! An algorithm for importing external nodes

use crate::{
    handle::Handle,
    source::Source,
    table::{NodeKind, Table},
};
use cl_ast::{PathPart, Sym, Use, UseTree};
use core::slice;
use std::mem;

pub fn import<'a>(table: &mut Table<'a>) -> Vec<(Handle, Error<'a>)> {
    let pending = mem::take(&mut table.uses);
    let mut failed = vec![];
    for import in pending {
        if let Err(e) = import_one(table, import) {
            failed.push((import, e));
        }
    }
    failed
}

fn import_one<'a>(table: &mut Table<'a>, item: Handle) -> UseResult<'a, ()> {
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
    let Use { absolute, tree } = tree;

    eprintln!("Resolving import {item}: {code}");
    import_tree(table, if !absolute { dst } else { table.root() }, dst, tree)
}

fn import_tree<'a>(
    table: &mut Table<'a>,
    src: Handle,
    dst: Handle,
    tree: &UseTree,
) -> UseResult<'a, ()> {
    match tree {
        UseTree::Tree(trees) => {
            for tree in trees {
                import_tree(table, src, dst, tree)?
            }
            Ok(())
        }
        UseTree::Path(part, rest) => {
            let source = table
                .nav(src, slice::from_ref(part))
                .ok_or_else(|| Error::NotFound(src, part.clone()))?;
            import_tree(table, source, dst, rest)
        }
        UseTree::Alias(src_name, dst_name) => import_name(table, src, src_name, dst, dst_name),
        UseTree::Name(src_name) => import_name(table, src, src_name, dst, src_name),
        UseTree::Glob => import_glob(table, src, dst),
    }
}

fn import_glob<'a>(table: &mut Table<'a>, src: Handle, dst: Handle) -> UseResult<'a, ()> {
    let Table { children, imports, .. } = table;
    if let Some(children) = children.get(&src) {
        // TODO: check for new imports clobbering existing imports
        imports.entry(dst).or_default().extend(children);
    }
    Ok(())
}

fn import_name<'a>(
    table: &mut Table<'a>,
    src: Handle,
    src_name: &Sym,
    dst: Handle,
    dst_name: &Sym,
) -> UseResult<'a, ()> {
    match table.get_by_sym(src, src_name) {
        // TODO: check for new imports clobbering existing imports
        Some(src_id) => table.add_import(dst, *dst_name, src_id),
        None => Err(Error::NotFound(src, PathPart::Ident(*src_name)))?,
    };
    Ok(())
}

pub type UseResult<'a, T> = Result<T, Error<'a>>;

#[derive(Debug)]
pub enum Error<'a> {
    ItsNoUse,
    NoParents,
    NoSource,
    BadSource(Source<'a>),
    NotFound(Handle, PathPart),
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
