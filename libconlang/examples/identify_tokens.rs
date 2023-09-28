//! This example grabs input from stdin, lexes it, and prints which lexer rules matched
#![allow(unused_imports)]
use conlang::lexer::Lexer;
use std::{error::Error, io::stdin};

fn main() -> Result<(), Box<dyn Error>> {
    // get input from stdin
    for line in stdin().lines() {
        let line = line?;
        let mut lexer = Lexer::new(&line);
        while let Some(token) = lexer.any() {
            println!("{:#19} │{}│", token.ty(), &line[token.range()])
        }
    }
    Ok(())
}
