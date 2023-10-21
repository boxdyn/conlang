//! This example grabs input from stdin, lexes it, parses it, and prints the AST
#![allow(unused_imports)]
use conlang::{lexer::Lexer, parser::Parser, pretty_printer::PrettyPrintable, token::Token};
use std::{
    error::Error,
    io::{stdin, stdout, IsTerminal, Read, Write},
    path::{Path, PathBuf},
};

fn main() -> Result<(), Box<dyn Error>> {
    let conf = Config::new();
    if conf.paths.is_empty() {
        take_stdin()?;
    } else {
        for path in conf.paths.iter().map(PathBuf::as_path) {
            parse(&std::fs::read_to_string(path)?, Some(path));
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
        print!("{PROMPT}");
        stdout().flush()?;
        for line in stdin().lines() {
            let line = line?;
            if !line.is_empty() {
                parse(&line, None);
                println!();
            }
            print!("{PROMPT}");
            stdout().flush()?;
        }
    } else {
        parse(&std::io::read_to_string(stdin())?, None)
    }
    Ok(())
}

fn parse(file: &str, path: Option<&Path>) {
    use conlang::parser::error::Error;
    match Parser::from(Lexer::new(file)).parse() {
        Ok(ast) => ast.print(),
        Err(e) if e.start().is_some() => print!("{:?}:{}", path.unwrap_or(Path::new("-")), e),
        Err(e) => print!("{e}"),
    }
    println!();
}
