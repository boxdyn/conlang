use crate::{ansi, ctx};
use cl_lexer::Lexer;
use cl_parser::Parser;
use repline::{error::ReplResult, prebaked::*};

fn clear() {
    println!("{}", ansi::CLEAR_ALL);
    banner()
}

pub fn banner() {
    println!("--- conlang v{} 💪🦈 ---\n", env!("CARGO_PKG_VERSION"))
}

/// Presents a selection interface to the user
pub fn main_menu(ctx: &mut ctx::Context) -> ReplResult<()> {
    banner();
    run(ctx)?;
    read_and(ansi::GREEN, "mu>", " ?>", |line| {
        match line.trim() {
            "clear" => clear(),
            "l" | "lex" => lex(ctx)?,
            "f" | "fmt" => fmt(ctx)?,
            "r" | "run" => run(ctx)?,
            "q" | "quit" => return Ok(Response::Break),
            "h" | "help" => println!(
                "Valid commands
    lex     (l): Spin up a lexer, and lex some lines
    fmt     (f): Format the input
    run     (r): Enter the REPL, and evaluate some statements
    help    (h): Print this list
    quit    (q): Exit the program"
            ),
            _ => Err("Unknown command. Type \"help\" for help")?,
        }
        Ok(Response::Accept)
    })
}

pub fn run(ctx: &mut ctx::Context) -> ReplResult<()> {
    read_and(ansi::CYAN, "cl>", " ?>", |line| {
        let code = Parser::new(Lexer::new(line)).stmt()?;

        print!("{}", ansi::OUTPUT);
        match ctx.run(&code) {
            Ok(v) => println!("{}{v}", ansi::RESET),
            Err(e) => println!("{}! > {e}{}", ansi::RED, ansi::RESET),
        }
        Ok(Response::Accept)
    })
}

pub fn lex(_ctx: &mut ctx::Context) -> ReplResult<()> {
    read_and(ansi::BRIGHT_BLUE, "lx>", " ?>", |line| {
        for token in Lexer::new(line) {
            match token {
                Ok(token) => crate::tools::print_token(&token),
                Err(e) => eprintln!("! > {}{e}{}", ansi::RED, ansi::RESET),
            }
        }

        Ok(Response::Accept)
    })
}

pub fn fmt(_ctx: &mut ctx::Context) -> ReplResult<()> {
    read_and(ansi::BRIGHT_MAGENTA, "cl>", " ?>", |line| {
        let mut p = Parser::new(Lexer::new(line));

        match p.stmt() {
            Ok(code) => println!("{}{code}{}", ansi::OUTPUT, ansi::RESET),
            Err(e) => Err(e)?,
        }

        Ok(Response::Accept)
    })
}
