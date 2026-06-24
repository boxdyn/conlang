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

use cl_ast::{
    AstNode, At, Bind, DefaultTypes, Expr, Pat, Use,
    desugar::type_bubbler::Bubbler,
    fold::Foldable,
    macro_matcher::{Match, Subst},
    visit::Walk,
};
use cl_interpret::{convalue::ConValue, env::Environment, interpret::Interpret};
use cl_lexer::{EOF, LexError, Lexer};
use cl_parser::{PResultExt, Parse, ParseError, Parser, inliner::ModuleInliner};
use cl_structures::span::Span;
use cl_token::{TKind, Token};
// use cl_typeck::Collector;
use repline::prebaked::*;
use std::{
    error::Error,
    io::{IsTerminal, stdin, stdout},
    marker::PhantomData,
};

mod builtin;

/// Prints the `--- conlang version ---` banner
fn banner() {
    println!("--- conlang v{} 💪🦈 ---", env!("CARGO_PKG_VERSION"))
}

/// Clears the terminal
fn clear() {
    print!("\x1b[H\x1b[2J\x1b[3J");
}

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
        read_and_mut(color, begin, "  > ", |rl, line| match line.trim_end() {
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

/// Performs macro substitution
fn subst() -> Result<(), Box<dyn Error>> {
    let mut rl = repline::Repline::new("\x1b[35mexp", " >", "?>");
    let exp = rl.read()?;
    let exp: At<Expr> = Parser::new(Lexer::new("<interactive>".into(), &exp)).parse(0)?;
    let mut exp = inline_modules(exp).0;
    println!("\x1b[G\x1b[J{exp}");

    rl.accept();

    loop {
        rl.set_color("\x1b[36mpat");
        let pat = rl.read()?;
        rl.accept();
        print!("\x1b[G\x1b[J");
        let mut p = Parser::new(Lexer::new("<interactive>".into(), &pat));

        let Ok(pat) = p.parse::<Expr>(0) else {
            println!("{exp}");
            continue;
        };

        if p.next_if(TKind::Arrow).is_err() {
            let Some(Subst { exp, pat }) = exp.match_with(&pat) else {
                println!("Match failed: {exp} <- {pat}");
                continue;
            };
            let mut pats: Vec<_> = pat.into_iter().collect();
            pats.sort_by_key(|(a, _)| a.to_ref());
            for (name, pat) in pats {
                println!("{name}: {pat}")
            }
            let mut exprs: Vec<_> = exp.into_iter().collect();
            exprs.sort_by_key(|(a, _)| a.to_ref());
            for (name, expr) in exprs.iter() {
                println!("{name}: {expr}")
            }
            continue;
        }

        let sub: Expr = p.parse(0)?;
        if exp.apply_rule(&pat, &sub) {
            println!("{exp}");
        } else {
            println!("No match: {pat} in {exp}\n")
        }
    }
}

/// Gets the English-language pluralizer for `count`
fn plural(count: usize) -> &'static str {
    match count {
        1 => "",
        _ => "s",
    }
}

/// Prints the tokenization of the input
fn tokens<'e: 't, 't, T: Parse<'t> + ?Sized>(
    _: &'e mut Environment,
    document: &'t str,
    verbose: Verbosity,
) -> Result<(), Box<dyn Error>> {
    let _: PhantomData<T>; // for lifetime variance
    let mut lexer = Lexer::new("<tokens>".into(), document);
    loop {
        match (lexer.scan(), verbose) {
            (Err(LexError { res: EOF, .. }), _) => {
                break;
            }
            (Err(e), _) => Err(e)?,
            (Ok(Token { lexeme, kind, span: Span { path: _, head, tail } }), Verbosity::Pretty) => {
                println!("{kind:?}\x1b[11G {head:<4} {tail:<4} {lexeme:?}")
            }
            (Ok(token), Verbosity::DebugPretty) => {
                println!("{token:#?}");
            }
            (Ok(token), Verbosity::Debug) => {
                println!("{token:?}")
            }
            _ => {}
        }
    }
    Ok(())
}

/// Parses and displays `T`s from the input
fn parse<'env: 't, 't, T>(
    _: &'env mut Environment,
    document: &'t str,
    verbose: Verbosity,
) -> Result<(), Box<dyn Error>>
where
    T: Parse<'t> + AstNode + for<'a> Walk<'a, DefaultTypes> + Foldable<DefaultTypes, DefaultTypes>,
    <T as Foldable<DefaultTypes, DefaultTypes>>::Out: AstNode,
{
    let mut parser = Parser::new(Lexer::new("<parse>".into(), document));
    for idx in 0..6 {
        let color_tag = if idx == 0 { 96 } else { (idx + 4) % 6 + 31 };
        match (
            parser
                .parse::<At<T, _>>(T::Prec::default())
                .map(inline_modules),
            verbose,
        ) {
            (Err(ParseError::EOF(_)), Verbosity::Quiet) => break,
            (Err(e @ ParseError::EOF(_)), _) => {
                println!(
                    "\x1b[92m{e} (total {} byte{}, {idx} expression{})\x1b[0m",
                    document.len(),
                    plural(document.len()),
                    plural(idx),
                );
                break;
            }
            (Err(e), _) => Err(e)?,
            (Ok(At(expr, span)), Verbosity::Pretty) => {
                println!("\x1b[{color_tag}m{span:?}:\n{expr}");
            }
            // (Ok(At(expr, span)), Verbosity::Frob) => {
            //     println!("\x1b[{color_tag}m{span}:\n");
            //     let _ = expr.visit_in(&mut Collector::new());
            // }
            (Ok(expr), Verbosity::Debug) => {
                println!("\x1b[{color_tag}m{expr:?}");
            }
            (Ok(expr), Verbosity::DebugPretty) => {
                println!("\x1b[{color_tag}m{expr:#?}");
            }
            (Ok(expr), Verbosity::Quiet) => {
                println!("{expr}");
            }
            _ => {}
        }
    }
    Ok(())
}

/// Parses and executes expressions from the input
fn run<'env: 't, 't>(
    env: &'env mut Environment,
    document: &'t str,
    verbose: Verbosity,
) -> Result<(), Box<dyn Error>> {
    let mut parser = Parser::new(Lexer::new("<run>".into(), document));
    for idx in 0..6 {
        let color_tag = (idx + 5) % 6 + 31;
        let Some(code) = parser.parse::<At<Expr>>(0).allow_eof()? else {
            break;
        };
        match (inline_modules(code).interpret(env), verbose) {
            (Err(error), _) => {
                println!("\x1b[31m{error}");
            }
            (Ok(ConValue::Empty), Verbosity::Pretty) => {}
            (Ok(value), Verbosity::Pretty) => {
                println!("\x1b[{color_tag}m{value}");
            }
            (Ok(value), Verbosity::Debug) => {
                println!("\x1b[{color_tag}m{value:?}");
            }
            (Ok(value), Verbosity::DebugPretty) => {
                println!("\x1b[{color_tag}m{value:#?}");
            }
            _ => {}
        }
    }
    Ok(())
}

/// Performs experimental desugaring on expressions from the input
fn bubble<'env: 't, 't>(
    _: &'env mut Environment,
    document: &'t str,
    verbose: Verbosity,
) -> Result<(), Box<dyn Error>> {
    let mut parser = Parser::new(Lexer::new("<bubble>".into(), document));
    for idx in 0..6 {
        match (
            parser
                .parse::<At<Expr>>(Default::default())
                .map(inline_modules)
                .map(|v| v.fold_in(&mut Bubbler(verbose == Verbosity::Frob)).unwrap()),
            verbose,
        ) {
            (Err(ParseError::EOF(_)), _) => break,
            (Err(e), _) => Err(e)?,
            (Ok(pat), Verbosity::Pretty | Verbosity::Frob) => {
                println!("\x1b[{}m{pat}", (idx + 5) % 6 + 31);
            }
            (Ok(pat), Verbosity::Debug) => {
                println!("\x1b[{}m{pat:?}", (idx + 5) % 6 + 31);
            }
            (Ok(pat), Verbosity::DebugPretty) => {
                println!("\x1b[{}m{pat:#?}", (idx + 5) % 6 + 31);
            }
            _ => {}
        }
    }
    Ok(())
}

/// Inlines modules at a given `T` relative to the PWD
fn inline_modules<T>(expr: At<T>) -> At<T::Out>
where
    T: AstNode + Foldable<DefaultTypes, DefaultTypes>,
    T::Out: AstNode,
{
    let mut mi = ModuleInliner::new(".");
    let At(expr, span) = expr;
    let Ok(expr) = expr.fold_in(&mut mi);
    if let Some((io_errs, parse_errs)) = mi.into_errs() {
        for (path, err) in io_errs {
            println!("{}: {err}", path.display());
        }
        for (path, err) in parse_errs {
            println!("{}: {err}", path.display());
        }
    }

    At(expr, span)
}

/// How much information to show about results
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Verbosity {
    #[default]
    Pretty,
    Debug,
    DebugPretty,
    Frob,
    Quiet,
}

impl TryFrom<&str> for Verbosity {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "quiet" => Ok(Verbosity::Quiet),
            "debug" | "d" => Ok(Verbosity::Debug),
            "debugpretty" | "debug_pretty" | "dp" => Ok(Verbosity::DebugPretty),
            "frob" => Ok(Verbosity::Frob),
            "pretty" => Ok(Verbosity::Pretty),
            _ => Err(()),
        }
    }
}

impl Verbosity {
    /// Gets a prompt string representing this verbosity
    fn begin(self) -> &'static str {
        match self {
            Self::Pretty => " .> ",
            Self::Debug => " ?> ",
            Self::DebugPretty => " #> ",
            Self::Frob => "🐸> ",
            Self::Quiet => " _> ",
        }
    }
}

/// What the next operation should be
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ParseMode {
    #[default]
    Run,
    Expr,
    Pat,
    Bind,
    Use,
    Tokens,
    Bubble,
}

impl TryFrom<&str> for ParseMode {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "run" => Ok(Self::Run),
            "fmt" | "format" | "expr" => Ok(Self::Expr),
            "pat" => Ok(Self::Pat),
            "bind" => Ok(Self::Bind),
            "use" => Ok(Self::Use),
            "tokens" => Ok(Self::Tokens),
            "bubble" => Ok(Self::Bubble),
            _ => Err(()),
        }
    }
}

impl ParseMode {
    /// Gets a function implementing this operation
    #[expect(clippy::type_complexity)]
    fn with<'env: 'a, 'a>(
        &self,
    ) -> fn(&'env mut Environment, &'a str, Verbosity) -> Result<(), Box<dyn Error>> {
        match self {
            Self::Expr => parse::<'env, 'a, Expr>,
            Self::Pat => parse::<'env, 'a, Pat>,
            Self::Bind => parse::<'env, 'a, Bind>,
            Self::Use => parse::<'env, 'a, Use>,
            Self::Tokens => tokens::<'env, 'a, dyn Parse<'a, Prec = ()>>,
            Self::Run => run::<'env, 'a>,
            Self::Bubble => bubble::<'env, 'a>,
        }
    }

    /// Gets an ANSI color representing this operation
    fn color(&self) -> &'static str {
        match self {
            Self::Run => "\x1b[36m",
            Self::Expr => "\x1b[35m",
            Self::Pat => "\x1b[34m",
            Self::Bind => "\x1b[33m",
            Self::Use => "\x1b[32m",
            Self::Tokens => "\x1b[31m",
            Self::Bubble => "\x1b[90m",
        }
    }
}
