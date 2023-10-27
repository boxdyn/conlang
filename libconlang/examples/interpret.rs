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
            run_file(&std::fs::read_to_string(path)?, Some(path))?;
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
        let mut buf = String::new();
        print!("{PROMPT}");
        stdout().flush()?;
        while let Ok(len) = stdin().read_line(&mut buf) {
            match len {
                0 => {
                    println!();
                    break;
                }
                1 => {
                    let _ = run(&buf, &mut interpreter).map_err(|e| eprintln!("{e}"));
                    print!("\n{PROMPT}");
                    buf.clear();
                }
                _ => print!(". "),
            }
            stdout().flush()?;
        }
    } else {
        run_file(&std::io::read_to_string(stdin())?, None)?
    }
    Ok(())
}

fn run_file(file: &str, path: Option<&Path>) -> Result<(), Box<dyn Error>> {
    match Parser::from(Lexer::new(file)).parse() {
        Ok(ast) => Interpreter::new().interpret(&ast)?,
        Err(e) if e.start().is_some() => print!("{:?}:{}", path.unwrap_or(Path::new("-")), e),
        Err(e) => print!("{e}"),
    }
    println!();
    Ok(())
}

fn run(input: &str, interpreter: &mut Interpreter) -> Result<(), Box<dyn Error>> {
    // If it parses successfully as a program, run the program
    let ast = Parser::from(Lexer::new(input)).parse();
    if let Ok(ast) = ast {
        interpreter.interpret(&ast)?;
        return Ok(());
    }
    let ast = Parser::from(Lexer::new(input)).parse_expr()?;
    for value in interpreter.eval(&ast)? {
        println!("{value}");
    }
    Ok(())
}
