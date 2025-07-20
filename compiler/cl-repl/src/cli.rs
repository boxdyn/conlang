//! Implement's the command line interface
use crate::{
    args::{Args, Mode},
    ctx::Context,
    menu,
    tools::print_token,
};
use cl_ast::File;
use cl_interpret::{builtin::builtins, convalue::ConValue, env::Environment, interpret::Interpret};
use cl_lexer::Lexer;
use cl_parser::Parser;
use std::{borrow::Cow, error::Error, path::Path};

/// Run the command line interface
pub fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let Args { file, include, mode, repl } = args;

    let mut env = Environment::new();

    env.add_builtins(&builtins! {
        /// Clears the screen
        fn clear() {
            menu::clear();
            Ok(ConValue::Empty)
        }

        fn eval(string) @env {
            use cl_interpret::error::Error;
            let string = match string {
                ConValue::Str(string) => string.to_ref(),
                ConValue::String(string) => string.as_str(),
                ConValue::Ref(v) => {
                    let string = env.get_id(*v).cloned().unwrap_or_default();
                    return eval(env, &[string])
                }
                _ => Err(Error::TypeError())?
            };
            match Parser::new("eval", Lexer::new(string)).parse::<cl_ast::Stmt>() {
                Err(e) => Ok(ConValue::Str(format!("{e}").into())),
                Ok(v) => v.interpret(env),
            }
        }

        /// Executes a file
        fn import(path) @env {
            use cl_interpret::error::Error;
            match path {
                ConValue::Str(path) => load_file(env, &**path).or(Ok(ConValue::Empty)),
                ConValue::String(path) => load_file(env, &**path).or(Ok(ConValue::Empty)),
                _ => Err(Error::TypeError())
            }
        }

        fn putchar(ConValue::Char(c)) {
            print!("{c}");
            Ok(ConValue::Empty)
        }

        /// Gets a line of input from stdin
        fn get_line(prompt) {
            use cl_interpret::error::Error;
            let prompt = match prompt {
                ConValue::Str(prompt) => prompt.to_ref(),
                ConValue::String(prompt) => prompt.as_str(),
                _ => Err(Error::TypeError())?,
            };
            match repline::Repline::new("", prompt, "").read() {
                Ok(line) => Ok(ConValue::String(line)),
                Err(repline::Error::CtrlD(line)) => Ok(ConValue::String(line)),
                Err(repline::Error::CtrlC(_)) => Err(cl_interpret::error::Error::Break(ConValue::Empty)),
                Err(e) => Ok(ConValue::Str(e.to_string().into())),
            }
        }
    });

    for path in include {
        load_file(&mut env, path)?;
    }

    if repl {
        if let Some(file) = file {
            if let Err(e) = load_file(&mut env, file) {
                eprintln!("{e}")
            }
        }
        let mut ctx = Context::with_env(env);
        match mode {
            Mode::Menu => menu::main_menu(&mut ctx)?,
            Mode::Lex => menu::lex(&mut ctx)?,
            Mode::Fmt => menu::fmt(&mut ctx)?,
            Mode::Run => menu::run(&mut ctx)?,
        }
    } else {
        let path = format_path_for_display(file.as_deref());
        let code = match &file {
            Some(file) => std::fs::read_to_string(file)?,
            None => std::io::read_to_string(std::io::stdin())?,
        };

        match mode {
            Mode::Lex => lex_code(&path, &code),
            Mode::Fmt => fmt_code(&path, &code),
            Mode::Run | Mode::Menu => run_code(&path, &code, &mut env),
        }?;
    }
    Ok(())
}

fn format_path_for_display(path: Option<&Path>) -> Cow<str> {
    match path {
        Some(file) => file
            .to_str()
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Owned(file.display().to_string())),
        None => Cow::Borrowed(""),
    }
}

fn load_file(env: &mut Environment, path: impl AsRef<Path>) -> Result<ConValue, Box<dyn Error>> {
    let path = path.as_ref();
    let inliner = cl_parser::inliner::ModuleInliner::new(path.with_extension(""));
    let file = std::fs::read_to_string(path)?;
    let code = Parser::new(path.display().to_string(), Lexer::new(&file)).parse()?;
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
    use cl_ast::WeightOf;
    eprintln!("File {} weighs {} units", code.name, code.weight_of());

    match env.eval(&code) {
        Ok(v) => Ok(v),
        Err(e) => {
            eprintln!("{e}");
            Ok(ConValue::Empty)
        }
    }
}

fn lex_code(path: &str, code: &str) -> Result<(), Box<dyn Error>> {
    for token in Lexer::new(code) {
        if !path.is_empty() {
            print!("{}:", path);
        }
        match token {
            Ok(token) => print_token(&token),
            Err(e) => println!("{e}"),
        }
    }
    Ok(())
}

fn fmt_code(path: &str, code: &str) -> Result<(), Box<dyn Error>> {
    let code = Parser::new(path, Lexer::new(code)).parse::<File>()?;
    println!("{code}");
    Ok(())
}

fn run_code(path: &str, code: &str, env: &mut Environment) -> Result<(), Box<dyn Error>> {
    let code = Parser::new(path, Lexer::new(code)).parse::<File>()?;
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
