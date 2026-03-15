use std::error::Error;

use crate::{ansi, args::Mode, ctx};
use cl_ast::{At, Expr};
use cl_interpret::convalue::ConValue;
use cl_lexer::Lexer;
use cl_parser::Parser;
use repline::{error::ReplResult, prebaked::*};

pub fn clear() {
    print!("{}", ansi::CLEAR_ALL);
    banner()
}

pub fn banner() {
    println!("--- conlang v{} 💪🦈 ---", env!("CARGO_PKG_VERSION"))
}

type ReplCallback = fn(&mut ctx::Context, &str) -> Result<Response, Box<dyn Error>>;
type ReplMode = (&'static str, &'static str, &'static str, ReplCallback);

#[rustfmt::skip]
const MODES: &[ReplMode] = &[
    (ansi::CYAN,            " .> ", "  > ", mode_run),
    (ansi::BRIGHT_BLUE,     " .> ", "  > ", mode_lex),
    (ansi::BRIGHT_MAGENTA,  " .> ", "  > ", mode_fmt),
];

const fn get_mode(mode: Mode) -> ReplMode {
    match mode {
        Mode::Lex => MODES[1],
        Mode::Fmt => MODES[2],
        Mode::Run => MODES[0],
    }
}

/// Presents a selection interface to the user
pub fn main_menu(mode: Mode, ctx: &mut ctx::Context) -> ReplResult<()> {
    banner();

    const HELP: &str = "Valid commands
    help  : Print this list
    clear : Clear the screen
    exit  : Exit the program
    lex   : Lex the input
    fmt   : Format the input
    run   : Evaluate some expressions";
    ctx.env.bind("help", HELP);

    let mut mode = get_mode(mode);
    read_and_mut(mode.0, mode.1, mode.2, |rl, line| {
        match line.trim() {
            "" => return Ok(Response::Continue),
            "help" => println!("{HELP}"),
            "clear" => clear(),
            "exit" => return Ok(Response::Break),
            "lex" => mode = get_mode(Mode::Lex),
            "fmt" => mode = get_mode(Mode::Fmt),
            "run" => mode = get_mode(Mode::Run),
            _ => return mode.3(ctx, line),
        }
        rl.set_prompt(mode.0, mode.1, mode.2);
        Ok(Response::Accept)
    })
}

pub fn mode_run(ctx: &mut ctx::Context, line: &str) -> Result<Response, Box<dyn Error>> {
    use cl_ast::fold::Fold;
    use cl_parser::inliner::ModuleInliner;

    if line.trim().is_empty() {
        return Ok(Response::Deny);
    }
    let code = Parser::new(Lexer::new("".into(), line)).parse::<At<Expr>>(0)?;
    let Ok(code) = ModuleInliner::new(".").fold_at_expr(code);

    print!("{}", ansi::OUTPUT);
    match ctx.run(&code) {
        Ok(ConValue::Empty) => print!("{}", ansi::RESET),
        Ok(v) => println!("{}{v}", ansi::RESET),
        Err(e) => println!("{}! > {e}{}", ansi::RED, ansi::RESET),
    }
    Ok(Response::Accept)
}

pub fn mode_lex(_ctx: &mut ctx::Context, line: &str) -> Result<Response, Box<dyn Error>> {
    let mut lexer = Lexer::new("".into(), line);
    while let Ok(token) = lexer.scan() {
        crate::tools::print_token(&token);
    }

    Ok(Response::Accept)
}

pub fn mode_fmt(_ctx: &mut ctx::Context, line: &str) -> Result<Response, Box<dyn Error>> {
    let mut p = Parser::new(Lexer::new("".into(), line));

    match p.parse::<At<Expr>>(0) {
        Ok(code) => println!("{}{code}{}", ansi::OUTPUT, ansi::RESET),
        Err(e) => Err(e)?,
    }

    Ok(Response::Accept)
}
