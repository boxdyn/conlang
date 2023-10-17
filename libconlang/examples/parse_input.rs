//! This example grabs input from stdin, lexes it, parses it, and prints the AST
#![allow(unused_imports)]
use conlang::{lexer::Lexer, parser::Parser, pretty_printer::PrettyPrintable, token::Token};
use std::{
    error::Error,
    io::{stdin, IsTerminal, Read},
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
    if stdin().is_terminal() {
        for line in stdin().lines() {
            parse(&line?, None)
        }
    } else {
        parse(&std::io::read_to_string(stdin())?, None)
    }
    Ok(())
}

fn parse(file: &str, path: Option<&Path>) {
    match Parser::from(Lexer::new(file)).parse() {
        Ok(ast) => ast.print(),
        Err(e) => {
            println!("{e:?}");
            if let Some(t) = e.start() {
                print_token(path, file, t)
            }
        }
    }
    println!();
}

fn print_token(path: Option<&Path>, file: &str, t: conlang::token::Token) {
    let path = path.unwrap_or(Path::new(""));
    println!(
        "{path:?}:{:02}:{:02}: {} ({})",
        t.line(),
        t.col(),
        &file[t.range()],
        t.ty(),
    )
}
