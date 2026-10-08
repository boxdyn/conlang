//! The new Conlang REPL
//!
//! # Introduction
//! Conlang is a ~~statically-typed~~ expressional language in the ML language family.
//!
//! It aims for maximal flexibility at (almost) any cost, allowing you to use
//! (almost) any syntax in (almost) any context, with as minimal bracketing
//! as possible.
//!
//! # Syntax
//!
//! ```ignore
#![doc = include_str!("tutorial.cl")]
//! ```

use repline::prebaked::*;
use std::{
    error::Error,
    io::{IsTerminal, stdin, stdout},
};

use conlang::*;

/// Prints the usage string
fn usage(command: &str) {
    println!("Usage: {command} [help | clear] [PARSEMODE] [VERBOSITY] [*.cl ...] [CODE ...]");
    println!();
    println!("Commands:");
    println!("    *.cl        Import a source file (by name)");
    println!("    code        Run some Conlang code");
    println!("    help        Print this help-text");
    println!("    clear       Clear the terminal on startup");
    println!();
    println!("Flags:");
    println!("    PARSEMODE   run, expr, pat, bind, use, tokens, or bubble");
    println!("    VERBOSITY   pretty, debugpretty or dp, debug, or quiet");
    println!();
}

/// The return type of [pargs]
type Args = (Verbosity, ParseMode, String, String, bool);

/// Parses [Args]
fn pargs() -> Result<Args, Box<dyn Error>> {
    let mut verbose = Verbosity::try_from(std::env::var("DO_VERBOSE").as_deref().unwrap_or(""))
        .unwrap_or_default();
    let mut parsing = ParseMode::try_from(std::env::var("DO_PARSING").as_deref().unwrap_or(""))
        .unwrap_or_default();
    let mut includes = String::new();
    let mut entrypoint = String::new();
    let mut interactive = stdin().is_terminal() && stdout().is_terminal();

    let mut args = std::env::args();
    let command = args.next();
    for arg in args {
        match arg.as_str() {
            "clear" => clear(),
            "-i" | "--interactive" => interactive = true,
            "--" => interactive = false,
            "help" | "-h" | "--help" => {
                usage(command.as_deref().unwrap_or("conlang"));
                std::process::exit(0);
            }
            line if let Ok(mode) = ParseMode::try_from(line) => parsing = mode,
            line if let Ok(mode) = Verbosity::try_from(line) => verbose = mode,
            line if line.ends_with(".cl") => includes += &format!("mod \"{line}\";\n"),
            _ => entrypoint += &(arg + " "),
        }
    }

    Ok((verbose, parsing, includes, entrypoint, interactive))
}

fn main() -> Result<(), Box<dyn Error>> {
    let (mut verbose, mut parsing, includes, entrypoint, interactive) = pargs()?;
    let mut env = builtin::get_env();
    let color = parsing.color();
    let begin = verbose.begin();

    if !includes.is_empty() {
        parsing.with()(&mut env, &includes, verbose)?;
    }
    if !entrypoint.is_empty() {
        parsing.with()(&mut env, &entrypoint, verbose)?;
        return Ok(());
    }

    if interactive {
        banner();
        read_and_mut(color, begin, "    ", |rl, line| match line.trim_end() {
            "" => Ok(Response::Continue),
            "exit" => Ok(Response::Break),
            "help" => {
                println!("Parsing: {parsing:?} (run, expr, pat, bind, use, tokens, bubble)");
                println!("Verbose: {verbose:?} (pretty, debugpretty (dp), debug, quiet)");
                Ok(Response::Deny)
            }
            "clear" => {
                clear();
                banner();
                Ok(Response::Deny)
            }
            "macro" => {
                if let Err(e) = subst() {
                    println!("\x1b[31m{e}\x1b[0m");
                }
                Ok(Response::Accept)
            }
            line if let Ok(mode) = ParseMode::try_from(line) => {
                parsing = mode;
                println!("Parse mode set to '{parsing:?}'");
                rl.set_color(parsing.color());
                Ok(Response::Accept)
            }
            line if let Ok(mode) = Verbosity::try_from(line) => {
                verbose = mode;
                println!("Verbosity set to '{verbose:?}'");
                rl.set_begin(verbose.begin());
                Ok(Response::Accept)
            }
            _ if let ParseMode::Run = parsing => {
                parsing.with()(&mut env, line, verbose)?;
                Ok(Response::Accept)
            }
            _ if line.ends_with("\n\n") => {
                parsing.with()(&mut env, line, verbose)?;
                Ok(Response::Accept)
            }
            _ => Ok(Response::Continue),
        })?;
    } else {
        let doc = std::io::read_to_string(stdin())?;
        parsing.with()(&mut env, &doc, verbose)?;
    }
    Ok(())
}
