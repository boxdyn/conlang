use cl_lexer::Lexer;
use cl_parser::Parser;
use cl_repl::repline::{error::Error as RlError, Repline};
use cl_typeck::{
    definition::Def, name_collector::NameCollectable, project::Project, type_resolver::resolve,
};
use std::error::Error;

// Path to display in standard library errors
const STDLIB_DISPLAY_PATH: &str = "stdlib/lib.cl";
// Statically included standard library
const STDLIB: &str = include_str!("../../stdlib/lib.cl");

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
    unsafe { TREES.push(code) }.collect_in_root(&mut prj)?;

    main_menu(&mut prj)
}

pub enum Response {
    Accept,
    Deny,
    Break,
}

fn read_and(
    color: &str,
    begin: &str,
    mut f: impl FnMut(&str) -> Result<Response, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let mut rl = Repline::new(color, begin, "? >");
    loop {
        let line = match rl.read() {
            Err(RlError::CtrlC(_)) => break,
            Err(RlError::CtrlD(line)) => {
                rl.deny();
                line
            }
            Ok(line) => line,
            Err(e) => Err(e)?,
        };
        print!("\x1b[G\x1b[J");
        match f(&line) {
            Ok(Response::Accept) => rl.accept(),
            Ok(Response::Deny) => rl.deny(),
            Ok(Response::Break) => break,
            Err(e) => print!("\x1b[40G\x1bJ\x1b[91m{e}\x1b[0m"),
        }
    }
    Ok(())
}

fn main_menu(prj: &mut Project) -> Result<(), Box<dyn Error>> {
    banner();
    read_and(C_MAIN, "mu>", |line| {
        match line.trim() {
            "c" | "code" => enter_code(prj),
            "clear" => clear(),
            "e" | "exit" => return Ok(Response::Break),
            "l" | "list" => list_types(prj),
            "q" | "query" => query_type_expression(prj),
            "r" | "resolve" => resolve_all(prj),
            "h" | "help" => {
                println!(
                    "Valid commands are:
    code    (c): Enter code to type-check
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
        .map(|_| Response::Accept)
    })
}

fn enter_code(prj: &mut Project) -> Result<(), Box<dyn Error>> {
    read_and(C_CODE, "cl>", |line| {
        if line.trim().is_empty() {
            return Ok(Response::Break);
        }
        let code = Parser::new(Lexer::new(line)).file()?;

        // Safety: this is totally unsafe
        unsafe { TREES.push(code) }.collect_in_root(prj)?;

        Ok(Response::Accept)
    })
}

fn query_type_expression(prj: &mut Project) -> Result<(), Box<dyn Error>> {
    read_and(C_RESV, "ty>", |line| {
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
    for id in prj.pool.key_iter() {
        resolve(prj, id)?;
    }
    println!("Types resolved successfully!");
    Ok(())
}

fn list_types(prj: &mut Project) -> Result<(), Box<dyn Error>> {
    println!("     name\x1b[30G  type");
    for (idx, Def { name, vis, kind, .. }) in prj.pool.iter().enumerate() {
        print!("{idx:3}: {vis}");
        if name.is_empty() {
            print!("\x1b[30m_\x1b[0m")
        }
        println!("{name}\x1b[30G| {kind}");
    }
    Ok(())
}

fn pretty_def(def: &Def, id: impl Into<usize>) {
    let id = id.into();
    let Def { vis, name, kind, module, meta, source } = def;
    for meta in *meta {
        println!("#[{meta}]")
    }
    println!("{vis}{name} [id: {id}] = {kind}");
    if let Some(source) = source {
        println!("Source:\n{C_LISTING}{source}\x1b[0m");
    }
    println!("\x1b[90m{module}\x1b[0m");
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
