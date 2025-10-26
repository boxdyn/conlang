use std::error::Error;

use crate::{ansi, args::Mode, ctx};
use cl_ast::Stmt;
use cl_interpret::convalue::ConValue;
use cl_lexer::Lexer;
use cl_parser::Parser;
use repline::{Error as RlError, error::ReplResult, prebaked::*};

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
    (ansi::CYAN,            " .>", "  >", mode_run),
    (ansi::BRIGHT_BLUE,     " .>", "  >", mode_lex),
    (ansi::BRIGHT_MAGENTA,  " .>", "  >", mode_fmt),
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
    let mut mode = get_mode(mode);
    banner();

    let mut rl = repline::Repline::new(mode.0, mode.1, mode.2);
    loop {
        rl.set_prompt(mode.0, mode.1, mode.2);

        let line = match rl.read() {
            Err(RlError::CtrlC(_)) => return Ok(()),
            Err(RlError::CtrlD(line)) => {
                rl.deny();
                line
            }
            Ok(line) => line,
            Err(e) => Err(e)?,
        };
        print!("\x1b[G\x1b[J");
        match line.trim() {
            "" => continue,
            "clear" => clear(),
            "mode run" => mode = get_mode(Mode::Run),
            "mode lex" => mode = get_mode(Mode::Lex),
            "mode fmt" => mode = get_mode(Mode::Fmt),
            "quit" => return Ok(()),
            "help" => println!(
                "Valid commands
    help     : Print this list
    clear    : Clear the screen
    quit     : Exit the program
    mode lex : Lex the input
    mode fmt : Format the input
    mode run : Evaluate some expressions"
            ),
            _ => match mode.3(ctx, &line) {
                Ok(Response::Continue) => continue,
                Ok(Response::Break) => return Ok(()),
                Ok(Response::Deny) => {}
                Ok(Response::Accept) => {
                    rl.accept();
                    continue;
                }
                Err(e) => {
                    rl.print_inline(format_args!("\t\x1b[31m{e}\x1b[0m"))?;
                    continue;
                }
            },
        }
        rl.deny();
    }
}

pub fn mode_run(ctx: &mut ctx::Context, line: &str) -> Result<Response, Box<dyn Error>> {
    use cl_ast::ast_visitor::Fold;
    use cl_parser::inliner::ModuleInliner;

    if line.trim().is_empty() {
        return Ok(Response::Deny);
    }
    let code = Parser::new("", Lexer::new(line)).parse::<Stmt>()?;
    let code = ModuleInliner::new(".").fold_stmt(code);

    print!("{}", ansi::OUTPUT);
    match ctx.run(&code) {
        Ok(ConValue::Empty) => print!("{}", ansi::RESET),
        Ok(v) => println!("{}{v}", ansi::RESET),
        Err(e) => println!("{}! > {e}{}", ansi::RED, ansi::RESET),
    }
    Ok(Response::Accept)
}

pub fn mode_lex(_ctx: &mut ctx::Context, line: &str) -> Result<Response, Box<dyn Error>> {
    for token in Lexer::new(line) {
        match token {
            Ok(token) => crate::tools::print_token(&token),
            Err(e) => eprintln!("! > {}{e}{}", ansi::RED, ansi::RESET),
        }
    }

    Ok(Response::Accept)
}

pub fn mode_fmt(_ctx: &mut ctx::Context, line: &str) -> Result<Response, Box<dyn Error>> {
    let mut p = Parser::new("", Lexer::new(line));

    match p.parse::<Stmt>() {
        Ok(code) => println!("{}{code}{}", ansi::OUTPUT, ansi::RESET),
        Err(e) => Err(e)?,
    }

    Ok(Response::Accept)
}
