use cl_lexer::Lexer;
use cl_parser::Parser;
use cl_repl::repline::{error::Error as RlError, Repline};
use cl_typeck::{name_collector::NameCollector, project::Project};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut prj = Project::default();
    let mut tcol = NameCollector::new(&mut prj);

    println!(
        "--- {} v{} 💪🦈 ---",
        env!("CARGO_BIN_NAME"),
        env!("CARGO_PKG_VERSION"),
    );

    read_and(
        "\x1b[33m",
        "cl>",
        "? >",
        |line| -> Result<_, Box<dyn Error>> {
            if line.trim_start().is_empty() {
                query(&tcol)?;
                return Ok(Response::Deny);
            }
            let mut parser = Parser::new(Lexer::new(line));
            let code = match parser.file() {
                Ok(code) => code,
                Err(e) => Err(e)?,
            };
            tcol.file(&code)?;
            Ok(Response::Accept)
        },
    )
}

pub enum Response {
    Accept,
    Deny,
    Break,
}

fn read_and(
    color: &str,
    begin: &str,
    again: &str,
    mut f: impl FnMut(&str) -> Result<Response, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let mut rl = Repline::new(color, begin, again);
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

fn query(prj: &Project) -> Result<(), Box<dyn Error>> {
    use cl_typeck::{
        definition::{Def, DefKind},
        type_kind::TypeKind,
    };
    read_and("\x1b[35m", "qy>", "? >", |line| {
        if line.trim_start().is_empty() {
            return Ok(Response::Break);
        }
        match line {
            "$all\n" => println!("{prj:#?}"),
            _ => {
                // parse it as a path, and convert the path into a borrowed path
                let path = Parser::new(Lexer::new(line)).path()?;

                let Some((type_id, path)) = prj.get_type((&path).into(), prj.module_root) else {
                    return Ok(Response::Deny);
                };
                let Def { name, vis, meta: _, kind, source: _, module } = &prj[type_id];
                match (kind, prj.get_value(path, type_id)) {
                    (_, Some((val, path))) => {
                        println!("value {}; {path}\n{:#?}", usize::from(val), prj[val])
                    }
                    (DefKind::Type(TypeKind::Module), None) => println!(
                        "{vis}mod \"{name}\" (#{}); {path}\n{:#?}",
                        usize::from(type_id),
                        module
                    ),
                    (_, None) => println!(
                        "type {name}(#{}); {path}\n{:#?}",
                        usize::from(type_id),
                        prj.pool[type_id]
                    ),
                };
            }
        }
        Ok(Response::Accept)
    })
}
