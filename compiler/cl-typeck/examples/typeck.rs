use cl_typeck::{
    entry::Entry,
    stage::{
        infer::{engine::InferenceEngine, error::InferenceError, inference::Inference},
        *,
    },
    table::Table,
    type_expression::TypeExpression,
};

use cl_ast::{Expr, types::Path, visit::Visit};
use cl_lexer::Lexer;
use cl_parser::{Parser, inliner::ModuleInliner};
use cl_structures::intern::string_interner::StringInterner;
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

fn main() -> Result<(), Box<dyn Error>> {
    let mut prj = Table::default();

    let mut parser = Parser::new(Lexer::new(STDLIB_DISPLAY_PATH.into(), PREAMBLE));
    let code = match parser.parse(0) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{STDLIB_DISPLAY_PATH}:{e}");
            Err(e)?
        }
    };
    // This code is special - it gets loaded from a hard-coded project directory (for now)
    let code = inline_modules(code, concat!(env!("CARGO_MANIFEST_DIR"), "/../../stdlib"));
    // let code = cl_ast::desugar::WhileElseDesugar.fold_file(code);
    let _ = Populator::new(&mut prj).visit_expr(interned(code));

    for arg in std::env::args().skip(1) {
        import_file(&mut prj, arg)?;
    }

    resolve_all(&mut prj)?;

    main_menu(&mut prj)?;
    Ok(())
}

fn main_menu(prj: &mut Table) -> Result<(), RlError> {
    banner();
    read_and(C_MAIN, "mu>", "? >", |line| {
        for line in line.trim().split_ascii_whitespace() {
            match line {
                "c" | "code" => enter_code(prj)?,
                "clear" => clear()?,
                "dump" => dump(prj)?,
                "d" | "desugar" => live_desugar()?,
                "e" | "exit" => return Ok(Response::Break),
                "f" | "file" => import_files(prj)?,
                "i" | "id" => get_by_id(prj)?,
                "l" | "list" => list_types(prj),
                "q" | "query" => query_type_expression(prj)?,
                "r" | "resolve" => resolve_all(prj)?,
                "s" | "strings" => print_strings(),
                "a" | "all" => infer_all(prj)?,
                "t" | "test" => infer_expression(prj)?,
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
        }
        Ok(Response::Accept)
    })
}

fn enter_code(prj: &mut Table) -> Result<(), RlError> {
    read_and(C_CODE, "cl>", "? >", |line| {
        if line.trim().is_empty() {
            return Ok(Response::Break);
        }
        let code = Parser::new(Lexer::new("".into(), line)).parse(0)?;
        let code = inline_modules(code, "");
        // let code = WhileElseDesugar.fold_file(code);

        let _ = Populator::new(prj).visit_expr(interned(code));
        Ok(Response::Accept)
    })
}

fn live_desugar() -> Result<(), RlError> {
    read_and(C_RESV, "se>", "? >", |line| {
        let code = Parser::new(Lexer::new("".into(), line)).parse::<Expr>(0)?;
        println!("Raw, as parsed:\n{C_LISTING}{code}\x1b[0m");

        // let code = ConstantFolder.fold_stmt(code);
        // println!("ConstantFolder\n{C_LISTING}{code}\x1b[0m");

        // let code = SquashGroups.fold_stmt(code);
        // println!("SquashGroups\n{C_LISTING}{code}\x1b[0m");

        // let code = WhileElseDesugar.fold_stmt(code);
        // println!("WhileElseDesugar\n{C_LISTING}{code}\x1b[0m");

        // let code = NormalizePaths::new().fold_stmt(code);
        // println!("NormalizePaths\n{C_LISTING}{code}\x1b[0m");

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
        // A query is comprised of a Ty and a relative Path
        let mut p = Parser::new(Lexer::new("".into(), line));
        let ty: cl_ast::Pat = p.parse(cl_parser::pat::Prec::Alt)?;
        let path: cl_ast::types::Path = p.parse(()).unwrap_or_else(|_| Path::from(""));
        let id = ty.evaluate(prj, prj.root())?;
        let id = path.evaluate(prj, id)?;
        pretty_handle(id.to_entry(prj))?;
        Ok(Response::Accept)
    })
}

#[allow(dead_code)]
fn infer_expression(prj: &mut Table) -> Result<(), RlError> {
    read_and(C_RESV, "ex>", "!?>", |line| {
        if line.trim().is_empty() {
            return Ok(Response::Break);
        }
        let mut p = Parser::new(Lexer::new("".into(), line));
        let e: Expr = p.parse(0)?;
        let mut inf = InferenceEngine::new(prj, prj.root());
        let ty = match interned(e).infer(&mut inf) {
            Ok(ty) => ty,
            Err(e) => match e {
                InferenceError::Mismatch(a, b) => {
                    eprintln!("Mismatched types: {}, {}", prj.entry(a), prj.entry(b));
                    return Ok(Response::Deny);
                }
                InferenceError::Recursive(a, b) => {
                    eprintln!("Recursive types: {}, {}", prj.entry(a), prj.entry(b));
                    return Ok(Response::Deny);
                }
                e => Err(e)?,
            },
        };
        eprintln!("--> {}", prj.entry(ty));
        Ok(Response::Accept)
    })
}

fn get_by_id(prj: &mut Table) -> Result<(), RlError> {
    use cl_parser::Parse;
    use cl_structures::index_map::MapIndex;
    use cl_typeck::handle::Handle;
    read_and(C_BYID, "id>", "? >", |line| {
        if line.trim().is_empty() {
            return Ok(Response::Break);
        }
        let mut parser = Parser::new(Lexer::new("".into(), line));
        let def_id = match Parse::parse(&mut parser, ())? {
            cl_ast::types::Literal::Int(int, _) => int as _,
            other => Err(format!("Expected integer, got {other}"))?,
        };
        let path = parser
            .parse::<cl_ast::types::Path>(())
            .unwrap_or_else(|_| Path::from(""));

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

fn infer_all(table: &mut Table) -> Result<(), Box<dyn Error>> {
    for (id, error) in InferenceEngine::new(table, table.root()).infer_all() {
        match error {
            InferenceError::Mismatch(a, b) => {
                eprint!("Mismatched types: {}, {}", table.entry(a), table.entry(b));
            }
            InferenceError::Recursive(a, b) => {
                eprint!("Recursive types: {}, {}", table.entry(a), table.entry(b));
            }
            e => eprint!("{e}"),
        }
        eprintln!(" in {id}\n({})\n", id.to_entry(table).source().unwrap())
    }

    println!("...Inferred!");
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

fn import_file(table: &mut Table, path: impl AsRef<std::path::Path>) -> Result<(), Box<dyn Error>> {
    let Ok(file) = std::fs::read_to_string(path.as_ref()) else {
        for file in std::fs::read_dir(path)? {
            println!("{}", file?.path().display())
        }
        return Ok(());
    };

    let mut parser = Parser::new(Lexer::new("".into(), &file));
    let code = match parser.parse(0) {
        Ok(code) => inline_modules(
            code,
            PathBuf::from(path.as_ref()).parent().unwrap_or("".as_ref()),
        ),
        Err(e) => {
            eprintln!("{C_ERROR}{}:{e}\x1b[0m", path.as_ref().display());
            return Ok(());
        }
    };

    // let code = cl_ast::desugar::WhileElseDesugar.fold_file(code);
    let _ = Populator::new(table).visit_expr(interned(code));

    Ok(())
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

        let mut parser = Parser::new(Lexer::new("".into(), &file));
        let code = match parser.parse(0) {
            Ok(code) => inline_modules(code, PathBuf::from(line).parent().unwrap_or("".as_ref())),
            Err(e) => {
                eprintln!("{C_ERROR}{line}:{e}\x1b[0m");
                return Ok(Response::Deny);
            }
        };

        let _ = Populator::new(table).visit_expr(interned(code));

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

fn inline_modules(code: Expr, path: impl AsRef<path::Path>) -> Expr {
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

fn dump(table: &Table) -> Result<(), Box<dyn Error>> {
    fn dump_recursive(
        name: cl_ast::types::Symbol,
        entry: Entry,
        depth: usize,
        to_file: &mut std::fs::File,
    ) -> std::io::Result<()> {
        use std::io::Write;
        write!(to_file, "{:w$}{name}: {entry}", "", w = depth)?;
        if let Some(children) = entry.children() {
            writeln!(to_file, " {{")?;
            for (name, child) in children {
                dump_recursive(*name, entry.with_id(*child), depth + 2, to_file)?;
            }
            write!(to_file, "{:w$}}}", "", w = depth)?;
        }
        writeln!(to_file)
    }

    let mut file = std::fs::File::create("typeck-table.ron")?;
    dump_recursive("root".into(), table.root_entry(), 0, &mut file)?;
    Ok(())
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

/// Interns an [Expr](cl_ast::Expr), returning a static reference to it.
fn interned(expr: Expr) -> &'static Expr {
    // lol. lmao even.
    Box::leak(Box::new(expr))
}
