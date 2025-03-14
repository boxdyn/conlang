//! A bare-minimum harness to evaluate a Conlang program

use std::{error::Error, path::PathBuf};

use cl_ast::Expr;
use cl_interpret::{convalue::ConValue, env::Environment};
use cl_lexer::Lexer;
use cl_parser::{inliner::ModuleInliner, Parser};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args();

    let prog = args.next().unwrap();
    let Some(path) = args.next().map(PathBuf::from) else {
        println!("Usage: {prog} `file.cl` [ args... ]");
        return Ok(());
    };

    let parent = path.parent().unwrap_or("".as_ref());

    let code = std::fs::read_to_string(&path)?;
    let code = Parser::new(path.display().to_string(), Lexer::new(&code)).parse()?;
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
                Parser::new(&arg, Lexer::new(&arg))
                    .parse::<Expr>()
                    .map(|arg| env.eval(&arg))
            })
            .collect::<Result<Vec<_>, _>>()?;

        match env.call(main, &args)? {
            ConValue::Empty => {}
            retval => println!("{retval}"),
        }
    }

    Ok(())
}
