use cl_embed::*;
use repline::{Response, prebaked};

fn main() -> Result<(), repline::Error> {
    prebaked::read_and("", "calc >", "   ? >", |line| {
        calc(line).map_err(Into::into)
    })
}

fn calc(line: &str) -> Result<Response, EvalError> {
    let mut env = Environment::new();
    env.bind("line", line);

    let res = conlang!(
        mod expression;
        use expression::{eval, parse};

        let (expr, rest) = parse(line.chars(), 0);
        eval(expr)
    )(&mut env)?;

    println!("{res}");

    Ok(Response::Accept)
}
