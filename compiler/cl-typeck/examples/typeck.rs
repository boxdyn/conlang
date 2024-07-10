use cl_ast::{
    ast_visitor::{Fold, Visit},
    desugar::*,
};
use cl_lexer::Lexer;
use cl_parser::{inliner::ModuleInliner, Parser};
use cl_typeck::{
    definition::Def,
    name_collector::NameCollector,
    node::{Node, NodeSource},
    project::Project,
    type_resolver::resolve,
};
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
const C_LISTING: &str = "\x1b[38;5;117m";

/// A home for immutable intermediate ASTs
///
/// TODO: remove this.
static mut TREES: TreeManager = TreeManager::new();

fn main() -> Result<(), Box<dyn Error>> {
    let mut prj = Project::default();

    let mut parser = Parser::new(Lexer::new(STDLIB));
    let code = match parser.file() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{STDLIB_DISPLAY_PATH}:{e}");
            Err(e)?
        }
    };
    let code = inline_modules(code, concat!(env!("PWD"), "/stdlib"));
    NameCollector::new(&mut prj).visit_file(unsafe { TREES.push(code) });

    main_menu(&mut prj)?;
    Ok(())
}

fn main_menu(prj: &mut Project) -> Result<(), RlError> {
    banner();
    read_and(C_MAIN, "mu>", "? >", |line| {
        match line.trim() {
            "c" | "code" => enter_code(prj)?,
            "clear" => clear()?,
            "e" | "exit" => return Ok(Response::Break),
            "l" | "list" => list_types(prj),
            "q" | "query" => query_type_expression(prj)?,
            "r" | "resolve" => resolve_all(prj)?,
            "d" | "desugar" => live_desugar()?,
            "h" | "help" => {
                println!(
                    "Valid commands are:
    code    (c): Enter code to type-check
    list    (l): List all known types
    query   (q): Query the type system
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

fn enter_code(prj: &mut Project) -> Result<(), RlError> {
    read_and(C_CODE, "cl>", "? >", |line| {
        if line.trim().is_empty() {
            return Ok(Response::Break);
        }
        let code = Parser::new(Lexer::new(line)).file()?;
        let code = inline_modules(code, "");
        let code = WhileElseDesugar.fold_file(code);
        // Safety: this is totally unsafe
        NameCollector::new(prj).visit_file(unsafe { TREES.push(code) });

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

fn query_type_expression(prj: &mut Project) -> Result<(), RlError> {
    read_and(C_RESV, "ty>", "? >", |line| {
        if line.trim().is_empty() {
            return Ok(Response::Break);
        }
        // parse it as a path, and convert the path into a borrowed path
        let ty = Parser::new(Lexer::new(line)).ty()?.kind;
        let id = prj.evaluate(&ty, prj.root)?;
        pretty_def(&prj[id], id);
        Ok(Response::Accept)
    })
}

fn resolve_all(prj: &mut Project) -> Result<(), Box<dyn Error>> {
    prj.resolve_imports()?;
    for id in prj.pool.keys() {
        resolve(prj, id)?;
    }
    println!("Types resolved successfully!");
    Ok(())
}

fn list_types(prj: &mut Project) {
    println!("     name\x1b[30G  type");
    for (idx, Def { kind, node: Node { vis, kind: source, .. }, .. }) in
        prj.pool.values().enumerate()
    {
        print!("{idx:3}: {vis}");
        if let Some(Some(name)) = source.as_ref().map(NodeSource::name) {
            print!("{name}");
        }
        println!("\x1b[30G| {kind}")
    }
}

fn pretty_def(def: &Def, id: impl Into<usize>) {
    let id = id.into();
    let Def { module, kind, node: Node { meta, vis, kind: source, .. } } = def;
    for meta in *meta {
        println!("#[{meta}]")
    }
    if let Some(Some(name)) = source.as_ref().map(NodeSource::name) {
        print!("{vis}{name} ");
    }
    println!("[id: {id}] = {kind}");
    if let Some(source) = source {
        println!("Source:\n{C_LISTING}{source}\x1b[0m");
    }
    println!("\x1b[90m{module}\x1b[0m");
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
