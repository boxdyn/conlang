use cl_structures::intern::string_interner::StringInterner;
use cl_typeck::{entry::Entry, stage::*, table::Table, type_expression::TypeExpression};

use cl_ast::{
    ast_visitor::{Fold, Visit},
    desugar::*,
    Stmt, Ty,
};
use cl_lexer::Lexer;
use cl_parser::{inliner::ModuleInliner, Parser};
use repline::{error::Error as RlError, prebaked::*};
use std::{
    error::Error,
    path::{self, PathBuf},
};

// Path to display in standard library errors
const STDLIB_DISPLAY_PATH: &str = "stdlib/lib.cl";
// Statically included standard library
const PREAMBLE: &str = r"
pub mod std;
pub use std::preamble::*;
";

// Colors
const C_MAIN: &str = C_LISTING;
const C_RESV: &str = "\x1b[35m";
const C_CODE: &str = "\x1b[36m";
const C_BYID: &str = "\x1b[95m";
const C_ERROR: &str = "\x1b[31m";
const C_LISTING: &str = "\x1b[38;5;117m";

/// A home for immutable intermediate ASTs
///
/// TODO: remove this.
static mut TREES: TreeManager = TreeManager::new();

fn main() -> Result<(), Box<dyn Error>> {
    let mut prj = Table::default();

    let mut parser = Parser::new(Lexer::new(PREAMBLE));
    let code = match parser.parse() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{STDLIB_DISPLAY_PATH}:{e}");
            Err(e)?
        }
    };
    // This code is special - it gets loaded from a hard-coded project directory (for now)
    let code = inline_modules(code, concat!(env!("CARGO_MANIFEST_DIR"), "/../../stdlib"));
    Populator::new(&mut prj).visit_file(unsafe { TREES.push(code) });

    main_menu(&mut prj)?;
    Ok(())
}

fn main_menu(prj: &mut Table) -> Result<(), RlError> {
    banner();
    read_and(C_MAIN, "mu>", "? >", |line| {
        match line.trim() {
            "c" | "code" => enter_code(prj)?,
            "clear" => clear()?,
            "d" | "desugar" => live_desugar()?,
            "e" | "exit" => return Ok(Response::Break),
            "f" | "file" => import_files(prj)?,
            "i" | "id" => get_by_id(prj)?,
            "l" | "list" => list_types(prj),
            "q" | "query" => query_type_expression(prj)?,
            "r" | "resolve" => resolve_all(prj)?,
            "s" | "strings" => print_strings(),
            "h" | "help" | "" => {
                println!(
                    "Valid commands are:
    clear      : Clear the screen
    code    (c): Enter code to type-check
    desugar (d): WIP: Test the experimental desugaring passes
    file    (f): Load files from disk
    id      (i): Get a type by its type ID
    list    (l): List all known types
    query   (q): Query the type system
    resolve (r): Perform type resolution
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
        let code = Parser::new(Lexer::new(line)).parse()?;
        let code = inline_modules(code, "");
        let code = WhileElseDesugar.fold_file(code);
        // Safety: this is totally unsafe
        Populator::new(prj).visit_file(unsafe { TREES.push(code) });
        Ok(Response::Accept)
    })
}

fn live_desugar() -> Result<(), RlError> {
    read_and(C_RESV, "se>", "? >", |line| {
        let code = Parser::new(Lexer::new(line)).parse::<Stmt>()?;
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

fn print_strings() {
    println!("{}", StringInterner::global());
}

fn query_type_expression(prj: &mut Table) -> Result<(), RlError> {
    read_and(C_RESV, "ty>", "? >", |line| {
        if line.trim().is_empty() {
            return Ok(Response::Break);
        }
        // parse it as a path, and convert the path into a borrowed path
        let ty: Ty = Parser::new(Lexer::new(line)).parse()?;
        let id = ty.evaluate(prj, prj.root())?;
        pretty_handle(id.to_entry(prj))?;
        Ok(Response::Accept)
    })
}

fn get_by_id(prj: &mut Table) -> Result<(), RlError> {
    use cl_parser::parser::Parse;
    use cl_structures::index_map::MapIndex;
    use cl_typeck::handle::Handle;
    read_and(C_BYID, "id>", "? >", |line| {
        if line.trim().is_empty() {
            return Ok(Response::Break);
        }
        let mut parser = Parser::new(Lexer::new(line));
        let def_id = match Parse::parse(&mut parser)? {
            cl_ast::Literal::Int(int) => int as _,
            other => Err(format!("Expected integer, got {other}"))?,
        };
        let mut path = parser.parse::<cl_ast::Path>().unwrap_or_default();
        path.absolute = false;

        let handle = Handle::from_usize(def_id).to_entry(prj);

        print!("  > {{{C_LISTING}{handle}\x1b[0m}}");
        if !path.parts.is_empty() {
            print!("::{path}")
        }
        println!();

        let Some(entry) = handle.nav(&path.parts) else {
            Err("No results.")?
        };

        pretty_handle(entry)?;

        Ok(Response::Accept)
    })
}

fn resolve_all(table: &mut Table) -> Result<(), Box<dyn Error>> {
    for (id, error) in import(table) {
        eprintln!("{error} in {} ({id})", id.to_entry(table))
    }
    for handle in table.handle_iter() {
        if let Err(error) = handle.to_entry_mut(table).categorize() {
            eprintln!("{error}");
        }
    }

    for handle in implement(table) {
        eprintln!("Unable to reparent {} ({handle})", handle.to_entry(table))
    }

    println!("...Resolved!");
    Ok(())
}

fn list_types(table: &mut Table) {
    for handle in table.debug_entry_iter() {
        let id = handle.id();
        let kind = handle.kind().unwrap();
        let name = handle.name().unwrap_or("".into());
        println!("{id:3}: {name:16}| {kind}: {handle}");
    }
}

fn import_files(table: &mut Table) -> Result<(), RlError> {
    read_and(C_RESV, "fi>", "? >", |line| {
        let line = line.trim();
        if line.is_empty() {
            return Ok(Response::Break);
        }
        let Ok(file) = std::fs::read_to_string(line) else {
            for file in std::fs::read_dir(line)? {
                println!("{}", file?.path().display())
            }
            return Ok(Response::Accept);
        };

        let mut parser = Parser::new(Lexer::new(&file));
        let code = match parser.parse() {
            Ok(code) => inline_modules(code, PathBuf::from(line).parent().unwrap_or("".as_ref())),
            Err(e) => {
                eprintln!("{C_ERROR}{line}:{e}\x1b[0m");
                return Ok(Response::Deny);
            }
        };

        Populator::new(table).visit_file(unsafe { TREES.push(code) });

        println!("...Imported!");
        Ok(Response::Accept)
    })
}

fn pretty_handle(entry: Entry) -> Result<(), std::io::Error> {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    let Some(kind) = entry.kind() else {
        return writeln!(out, "{entry}");
    };
    write!(out, "{C_LISTING}{kind}")?;

    if let Some(name) = entry.name() {
        write!(out, " {name}")?;
    }
    writeln!(out, "\x1b[0m ({}): {entry}", entry.id())?;

    if let Some(parent) = entry.parent() {
        writeln!(
            out,
            "- {C_LISTING}Parent\x1b[0m: {parent} ({})",
            parent.id()
        )?;
    }

    if let Some(span) = entry.span() {
        writeln!(
            out,
            "- {C_LISTING}Span:\x1b[0m ({}, {})",
            span.head, span.tail
        )?;
    }

    match entry.meta() {
        Some(meta) if !meta.is_empty() => {
            writeln!(out, "- {C_LISTING}Meta:\x1b[0m")?;
            for meta in meta {
                writeln!(out, "  - {meta}")?;
            }
        }
        _ => {}
    }

    if let Some(children) = entry.children() {
        writeln!(out, "- {C_LISTING}Children:\x1b[0m")?;
        for (name, child) in children {
            writeln!(
                out,
                "  - {C_LISTING}{name}\x1b[0m ({child}): {}",
                entry.with_id(*child)
            )?
        }
    }

    if let Some(imports) = entry.imports() {
        writeln!(out, "- {C_LISTING}Imports:\x1b[0m")?;
        for (name, child) in imports {
            writeln!(
                out,
                "  - {C_LISTING}{name}\x1b[0m ({child}): {}",
                entry.with_id(*child)
            )?
        }
    }

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
