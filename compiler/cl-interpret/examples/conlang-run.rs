//! A bare-minimum harness to evaluate a Conlang program

use std::{error::Error, path::PathBuf};

use cl_ast::{Expr, types::Symbol};
use cl_interpret::{convalue::ConValue, env::Environment};
use cl_lexer::Lexer;
use cl_parser::{Parser, inliner::ModuleInliner};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args();

    let prog = args.next().unwrap();
    let Some(path) = args.next().map(PathBuf::from) else {
        println!("Usage: {prog} `file.cl` [ args... ]");
        return Ok(());
    };

    let display_path: Symbol = Symbol::from(&*path.display().to_string());

    let parent = path.parent().unwrap_or("".as_ref());

    let code = std::fs::read_to_string(&path)?;
    let code: Expr = Parser::new(Lexer::new(display_path, &code)).parse(0)?;
    let code = match ModuleInliner::new(parent).inline(code) {
        Ok(code) => code,
        Err((code, ioerrs, perrs)) => {
            for (p, err) in ioerrs {
                eprintln!("{}:{err}", p.display());
            }
            for (p, err) in perrs {
                eprintln!("{}:{err}", p.display());
            }
            code
        }
    };

    let mut env = Environment::new();
    env.eval(&code)?;

    let main = "main".into();
    if env.get(main).is_ok() {
        let args = args
            .flat_map(|arg| {
                Parser::new(Lexer::new("conlang-run".into(), &arg))
                    .parse::<Expr>(0)
                    .map(|arg| env.eval(&arg))
            })
            .collect::<Result<Vec<_>, _>>()?;

        match env.call(main, &args) {
            Ok(ConValue::Empty) => {}
            Ok(retval) => println!("{retval}"),
            Err(e) => {
                panic!("{e}");
            }
        }
    }

    Ok(())
}
