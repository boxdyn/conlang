//! Implement's the command line interface
use crate::{
    args::{Args, Mode},
    ctx::Context,
    menu,
    tools::print_token,
};
use cl_ast::File;
use cl_interpret::{convalue::ConValue, env::Environment, interpret::Interpret};
use cl_lexer::Lexer;
use cl_parser::Parser;
use std::{error::Error, path::Path};

/// Run the command line interface
pub fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let Args { file, include, mode, repl } = args;

    let mut env = Environment::new();
    for path in include {
        load_file(&mut env, path)?;
    }

    if repl {
        if let Some(file) = file {
            load_file(&mut env, file)?;
        }
        let mut ctx = Context::with_env(env);
        match mode {
            Mode::Menu => menu::main_menu(&mut ctx)?,
            Mode::Lex => menu::lex(&mut ctx)?,
            Mode::Fmt => menu::fmt(&mut ctx)?,
            Mode::Run => menu::run(&mut ctx)?,
        }
    } else {
        let code = match &file {
            Some(file) => std::fs::read_to_string(file)?,
            None => std::io::read_to_string(std::io::stdin())?,
        };

        match mode {
            Mode::Lex => lex_code(&code, file),
            Mode::Fmt => fmt_code(&code),
            Mode::Run | Mode::Menu => run_code(&code, &mut env),
        }?;
    }
    Ok(())
}

fn load_file(env: &mut Environment, path: impl AsRef<Path>) -> Result<ConValue, Box<dyn Error>> {
    let inliner =
        cl_parser::inliner::ModuleInliner::new(path.as_ref().parent().unwrap_or(Path::new("")));
    let file = std::fs::read_to_string(path)?;
    let code = Parser::new(Lexer::new(&file)).parse()?;
    let code = match inliner.inline(code) {
        Ok(a) => a,
        Err((code, io_errs, parse_errs)) => {
            for (file, err) in io_errs {
                eprintln!("{}:{err}", file.display());
            }
            for (file, err) in parse_errs {
                eprintln!("{}:{err}", file.display());
            }
            code
        }
    };
    Ok(env.eval(&code)?)
}

fn lex_code(code: &str, path: Option<impl AsRef<Path>>) -> Result<(), Box<dyn Error>> {
    for token in Lexer::new(code) {
        if let Some(path) = &path {
            print!("{}:", path.as_ref().display());
        }
        match token {
            Ok(token) => print_token(&token),
            Err(e) => println!("{e}"),
        }
    }
    Ok(())
}

fn fmt_code(code: &str) -> Result<(), Box<dyn Error>> {
    let code = Parser::new(Lexer::new(code)).parse::<File>()?;
    println!("{code}");
    Ok(())
}

fn run_code(code: &str, env: &mut Environment) -> Result<(), Box<dyn Error>> {
    let code = Parser::new(Lexer::new(code)).parse::<File>()?;
    match code.interpret(env)? {
        ConValue::Empty => {}
        ret => println!("{ret}"),
    }
    if env.get("main".into()).is_ok() {
        match env.call("main".into(), &[])? {
            ConValue::Empty => {}
            ret => println!("{ret}"),
        }
    }
    Ok(())
}
