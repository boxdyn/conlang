//! This example grabs input from stdin, lexes it, and prints which lexer rules matched
#![allow(unused_imports)]
use conlang::lexer::Lexer;
use std::{io::stdin, error::Error};

fn main() -> Result<(), Box<dyn Error>>{
    // get input from stdin
    for line in stdin().lines() {
        let line = line?;
        // lex the line
        for func in [Lexer::line_comment, Lexer::block_comment, Lexer::shebang_comment, Lexer::identifier, Lexer::integer, Lexer::float, Lexer::string] {
            if let Some(token) = func(&mut Lexer::new(&line)) {
                println!("{:?}: {}", token, &line[token.range()])
            }
        }
    }
    Ok(())
}