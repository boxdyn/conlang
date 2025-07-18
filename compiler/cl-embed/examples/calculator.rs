//! Demonstrates the cl_embed library

use cl_embed::*;
use repline::{Response, prebaked};

fn main() -> Result<(), repline::Error> {
    let mut env = Environment::new();

    if let Err(e) = conlang_include!("calculator/expression.cl")(&mut env) {
        panic!("{e}")
    }

    prebaked::read_and("", "calc >", "   ? >", |line| {
        env.bind("line", line);

        let res = conlang! {

            let (expr, rest) = parse(line.chars(), Power::None);
            execute(expr)

        }(&mut env)?;

        println!("{res}");

        Ok(Response::Accept)
    })
}
