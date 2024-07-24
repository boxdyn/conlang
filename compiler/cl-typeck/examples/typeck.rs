use cl_typeck::{
    entry::Entry, import::import, populate::Populator, table::Table,
    type_expression::TypeExpression,
};

use cl_ast::{
    ast_visitor::{Fold, Visit},
    desugar::*,
};
use cl_lexer::Lexer;
use cl_parser::{inliner::ModuleInliner, Parser};
use repline::{error::Error as RlError, prebaked::*};
use std::{error::Error, path};

// Path to display in standard library errors
const STDLIB_DISPLAY_PATH: &str = "stdlib/lib.cl";
// Statically included standard library
const STDLIB: &str = include_str!("../../../stdlib/lib.cl");

// Colors
const C_MAIN: &str = "";
const C_RESV: &str = "\x1b[35m";
const C_CODE: &str = "\x1b[36m";
const C_BYID: &str = "\x1b[95m";
const C_LISTING: &str = "\x1b[38;5;117m";

/// A home for immutable intermediate ASTs
///
/// TODO: remove this.
static mut TREES: TreeManager = TreeManager::new();

fn main() -> Result<(), Box<dyn Error>> {
    let mut prj = Table::default();

    let mut parser = Parser::new(Lexer::new(STDLIB));
    let code = match parser.file() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{STDLIB_DISPLAY_PATH}:{e}");
            Err(e)?
        }
    };
    let code = inline_modules(code, concat!(env!("PWD"), "/stdlib"));
    Populator::new(&mut prj).visit_file(unsafe { TREES.push(code) });
    // NameCollector::new(&mut prj).visit_file(unsafe { TREES.push(code) });

    main_menu(&mut prj)?;
    Ok(())
}

fn main_menu(prj: &mut Table) -> Result<(), RlError> {
    banner();
    read_and(C_MAIN, "mu>", "? >", |line| {
        match line.trim() {
            "c" | "code" => enter_code(prj)?,
            "clear" => clear()?,
            "e" | "exit" => return Ok(Response::Break),
            "l" | "list" => list_types(prj),
            "q" | "query" => query_type_expression(prj)?,
            "i" | "id" => get_by_id(prj)?,
            "r" | "resolve" => resolve_all(prj)?,
            "d" | "desugar" => live_desugar()?,
            "h" | "help" => {
                println!(
                    "Valid commands are:
    code    (c): Enter code to type-check
    list    (l): List all known types
    query   (q): Query the type system
    id      (i): Get a type by its type ID
    resolve (r): Perform type resolution
    desugar (d): WIP: Test the experimental desugaring passes
    help    (h): Print this list
    exit    (e): Exit the program"
                );
                return Ok(Response::Deny);
            }
            _ => Err(r#"Invalid command. Type "help" to see the list of valid commands."#)?,
        }
        Ok(Response::Accept)
    })
}

fn enter_code(prj: &mut Table) -> Result<(), RlError> {
    read_and(C_CODE, "cl>", "? >", |line| {
        if line.trim().is_empty() {
            return Ok(Response::Break);
        }
        let code = Parser::new(Lexer::new(line)).file()?;
        let code = inline_modules(code, "");
        let code = WhileElseDesugar.fold_file(code);
        // Safety: this is totally unsafe
        Populator::new(prj).visit_file(unsafe { TREES.push(code) });
        Ok(Response::Accept)
    })
}

fn live_desugar() -> Result<(), RlError> {
    read_and(C_RESV, "se>", "? >", |line| {
        let code = Parser::new(Lexer::new(line)).stmt()?;
        println!("Raw, as parsed:\n{C_LISTING}{code}\x1b[0m");

        let code = SquashGroups.fold_stmt(code);
        println!("SquashGroups\n{C_LISTING}{code}\x1b[0m");

        let code = WhileElseDesugar.fold_stmt(code);
        println!("WhileElseDesugar\n{C_LISTING}{code}\x1b[0m");

        let code = NormalizePaths::new().fold_stmt(code);
        println!("NormalizePaths\n{C_LISTING}{code}\x1b[0m");

        Ok(Response::Accept)
    })
}

fn query_type_expression(prj: &mut Table) -> Result<(), RlError> {
    read_and(C_RESV, "ty>", "? >", |line| {
        if line.trim().is_empty() {
            return Ok(Response::Break);
        }
        // parse it as a path, and convert the path into a borrowed path
        let ty = Parser::new(Lexer::new(line)).ty()?.kind;
        let id = ty.evaluate(prj, prj.root())?;
        pretty_handle(id.to_entry(prj))?;
        Ok(Response::Accept)
    })
}

fn get_by_id(prj: &mut Table) -> Result<(), RlError> {
    use cl_structures::index_map::MapIndex;
    use cl_typeck::handle::Handle;
    read_and(C_BYID, "id>", "? >", |line| {
        if line.trim().is_empty() {
            return Ok(Response::Break);
        }
        let mut parser = Parser::new(Lexer::new(line));
        let def_id = match parser.literal()? {
            cl_ast::Literal::Int(int) => int as _,
            other => Err(format!("Expected integer, got {other}"))?,
        };
        let mut path = parser.path().unwrap_or_default();
        path.absolute = false;

        let handle = Handle::from_usize(def_id).to_entry(prj);

        print!("  > {{{C_LISTING}{handle}\x1b[0m}}");
        if !path.parts.is_empty() {
            print!("::{path}")
        }
        println!();

        let Some(handle) = handle.nav(&path.parts) else {
            Err("No results.")?
        };

        pretty_handle(handle)?;

        Ok(Response::Accept)
    })
}

fn resolve_all(prj: &mut Table) -> Result<(), Box<dyn Error>> {
    println!("Resolving imports:");
    for (id, error) in import(prj) {
        eprintln!("{error} in {} ({id})", id.to_entry(prj))
    }
    // todo!("Resolve imports");
    // prj.resolve_imports()?;
    // for id in prj.pool.keys() {
    //     resolve(prj, id)?;
    // }
    // println!("Types resolved successfully!");
    Ok(())
}

fn list_types(prj: &mut Table) {
    for handle in prj.debug_handle_iter() {
        let id = handle.id();
        let kind = handle.kind().unwrap();
        let name = handle.name().unwrap_or("".into());
        println!("{id:3}: {name:16}| {kind} {handle}");
    }
}

fn pretty_handle(h: Entry) -> Result<(), std::io::Error> {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    let Some(kind) = h.kind() else {
        return writeln!(out, "Invalid handle: {h}");
    };
    write!(out, "{C_LISTING}{kind}")?;
    if let Some(name) = h.name() {
        write!(out, " {name}")?;
    }
    writeln!(out, "\x1b[0m: {h}")?;

    if let Some(parent) = h.parent() {
        writeln!(out, "- {C_LISTING}Parent\x1b[0m: {parent}")?;
    }

    if let Some(span) = h.span() {
        writeln!(
            out,
            "- {C_LISTING}Span:\x1b[0m ({}, {})",
            span.head, span.tail
        )?;
    }

    match h.meta() {
        Some(meta) if !meta.is_empty() => {
            writeln!(out, "- {C_LISTING}Meta:\x1b[0m")?;
            for meta in meta {
                writeln!(out, "  - {meta}")?;
            }
        }
        _ => {}
    }

    if let Some(children) = h.children() {
        writeln!(out, "- {C_LISTING}Children:\x1b[0m")?;
        for (name, child) in children {
            writeln!(
                out,
                "  - {C_LISTING}{name}\x1b[0m ({child}): {}",
                h.with_id(*child)
            )?
        }
    }

    if let Some(imports) = h.imports() {
        writeln!(out, "- {C_LISTING}Imports:\x1b[0m")?;
        for (name, child) in imports {
            writeln!(
                out,
                "  - {C_LISTING}{name}\x1b[0m ({child}): {}",
                h.with_id(*child)
            )?
        }
    }

    // let Some(vis) = handle.vis() else {
    //     return writeln!(stdout, "Invalid handle: {handle}");
    // };
    // writeln!(stdout, "{C_LISTING}{}\x1b[0m: {vis}{handle}", handle.id())?;
    // if let Some(parent) = handle.parent() {
    // }
    // if let Some(types) = handle.types() {
    //     writeln!(stdout, "{C_LISTING}Types:\x1b[0m")?;
    //     for (name, def) in types {
    //     }
    // }
    // if let Some(values) = handle.values() {
    //     writeln!(stdout, "{C_LISTING}Values:\x1b[0m")?;
    //     for (name, def) in values {
    //         writeln!(stdout, "- {C_LISTING}{name}\x1b[0m: {}", handle.with(*def))?
    //     }
    // }
    // write!(stdout, "\x1b[0m")
    Ok(())
}

fn inline_modules(code: cl_ast::File, path: impl AsRef<path::Path>) -> cl_ast::File {
    match ModuleInliner::new(path).inline(code) {
        Err((code, io, parse)) => {
            for (file, error) in io {
                eprintln!("{}:{error}", file.display());
            }
            for (file, error) in parse {
                eprintln!("{}:{error}", file.display());
            }
            code
        }
        Ok(code) => code,
    }
}

fn clear() -> Result<(), Box<dyn Error>> {
    println!("\x1b[H\x1b[2J");
    banner();
    Ok(())
}

fn banner() {
    println!(
        "--- {} v{} 💪🦈 ---",
        env!("CARGO_BIN_NAME"),
        env!("CARGO_PKG_VERSION"),
    );
}

/// Keeps leaked references to past ASTs, for posterity:tm:
struct TreeManager {
    trees: Vec<&'static cl_ast::File>,
}

impl TreeManager {
    const fn new() -> Self {
        Self { trees: vec![] }
    }
    fn push(&mut self, tree: cl_ast::File) -> &'static cl_ast::File {
        let ptr = Box::leak(Box::new(tree));
        self.trees.push(ptr);
        ptr
    }
}
