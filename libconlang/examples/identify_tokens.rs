//! This example grabs input from stdin, lexes it, and prints which lexer rules matched
#![allow(unused_imports)]
use conlang::lexer::Lexer;
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
            lex_tokens(&std::fs::read_to_string(path)?, Some(path));
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
            lex_tokens(&line?, None)
        }
    } else {
        lex_tokens(&std::io::read_to_string(stdin())?, None)
    }
    Ok(())
}

fn lex_tokens(file: &str, path: Option<&Path>) {
    for token in Lexer::new(file) {
        if let Some(path) = path {
            print!("{path:?}:")
        }
        print_token(file, token);
    }
}

fn print_token(line: &str, t: conlang::token::Token) {
    println!(
        "{:02}:{:02}: {:#19} │{}│",
        t.line(),
        t.col(),
        t.ty(),
        &line[t.range()]
    )
}
