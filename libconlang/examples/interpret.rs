//! This example grabs input from stdin or a file, lexes it, parses it, and interprets it
use conlang::{interpreter::Interpreter, lexer::Lexer, parser::Parser};
use std::{
    error::Error,
    io::{stdin, stdout, IsTerminal, Write},
    path::{Path, PathBuf},
};

fn main() -> Result<(), Box<dyn Error>> {
    let conf = Config::new();
    if conf.paths.is_empty() {
        take_stdin()?;
    } else {
        for path in conf.paths.iter().map(PathBuf::as_path) {
            parse(&std::fs::read_to_string(path)?, Some(path))?;
        }
    }
    Ok(())
}

struct Config {
    paths: Vec<PathBuf>,
}

impl Config {
    fn new() -> Self {
        Config { paths: std::env::args().skip(1).map(PathBuf::from).collect() }
    }
}

fn take_stdin() -> Result<(), Box<dyn Error>> {
    const PROMPT: &str = "> ";
    if stdin().is_terminal() {
        let mut interpreter = Interpreter::new();
        print!("{PROMPT}");
        stdout().flush()?;
        for line in stdin().lines() {
            let line = line?;
            if !line.is_empty() {
                let _ = run(&line, &mut interpreter).map_err(|e| eprintln!("{e}"));
                println!();
            }
            print!("{PROMPT}");
            stdout().flush()?;
        }
    } else {
        parse(&std::io::read_to_string(stdin())?, None)?
    }
    Ok(())
}

fn parse(file: &str, path: Option<&Path>) -> Result<(), Box<dyn Error>> {
    match Parser::from(Lexer::new(file)).parse() {
        Ok(ast) => Interpreter::new().interpret(&ast)?,
        Err(e) if e.start().is_some() => print!("{:?}:{}", path.unwrap_or(Path::new("-")), e),
        Err(e) => print!("{e}"),
    }
    println!();
    Ok(())
}

fn run(file: &str, interpreter: &mut Interpreter) -> Result<(), Box<dyn Error>> {
    // If it parses successfully as a program, run the program
    match Parser::from(Lexer::new(file)).parse() {
        Ok(ast) => interpreter.interpret(&ast)?,
        Err(e) => {
            // If not, re-parse as an expression, and print the stack
            let Ok(expr) = Parser::from(Lexer::new(file)).parse_expr() else {
                Err(e)?
            };
            for value in interpreter.eval(&expr)? {
                println!("{value}");
            }
        }
    }
    Ok(())
}
