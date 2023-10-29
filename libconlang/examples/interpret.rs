//! This example grabs input from stdin or a file, lexes it, parses it, and interprets it
use conlang::{
    ast::{expression::Expr, Start},
    interpreter::Interpreter,
    lexer::Lexer,
    parser::Parser,
};
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

macro_rules! prompt {
    ($($t:tt)*) => {{
        let mut out = stdout().lock();
        out.write_all(b"\x1b[36m")?;
        write!(out, $($t)*)?;
        out.write_all(b"\x1b[0m")?;
        out.flush()
    }};
}

fn take_stdin() -> Result<(), Box<dyn Error>> {
    const CONLANG: &str = "cl>";
    const MOREPLS: &str = " ?>";
    if stdin().is_terminal() {
        let mut interpreter = Interpreter::new();
        let mut buf = String::new();
        prompt!("{CONLANG} ")?;
        while let Ok(len) = stdin().read_line(&mut buf) {
            let code = Program::parse(&buf);
            match (len, code) {
                // Exit the loop
                (0, _) => {
                    println!();
                    break;
                }
                // If the code is OK, run it and print any errors
                (_, Ok(code)) => {
                    let _ = code.run(&mut interpreter).map_err(|e| eprintln!("{e}"));
                    buf.clear();
                }
                // If the user types two newlines, print syntax errors
                (1, Err(e)) => {
                    eprintln!("{e}");
                    buf.clear();
                }
                // Otherwise, ask for more input
                _ => {
                    prompt!("{MOREPLS} ")?;
                    continue;
                }
            }
            prompt!("{CONLANG} ")?;
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

enum Program {
    Program(Start),
    Expr(Expr),
}

impl Program {
    fn parse(input: &str) -> conlang::parser::error::PResult<Self> {
        let ast = Parser::from(Lexer::new(input)).parse();
        if let Ok(ast) = ast {
            return Ok(Self::Program(ast));
        }
        Ok(Self::Expr(Parser::from(Lexer::new(input)).parse_expr()?))
    }
    fn run(&self, interpreter: &mut Interpreter) -> conlang::interpreter::error::IResult<()> {
        match self {
            Program::Program(start) => interpreter.interpret(start),
            Program::Expr(expr) => {
                for value in interpreter.eval(expr)? {
                    println!("{value}")
                }
                Ok(())
            }
        }
    }
}
