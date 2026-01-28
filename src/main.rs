//! Tests the lexer\

use cl_ast::{
    Annotation, At, Bind, DefaultTypes, Expr, Pat, Use,
    fold::Foldable,
    macro_matcher::{Match, Subst},
    visit::Walk,
};
use cl_lexer::{EOF, LexError, Lexer};
use cl_parser::{Parse, ParseError, Parser, inliner::ModuleInliner};
use cl_structures::span::Span;
use cl_token::{TKind, Token};
// use cl_typeck::Collector;
use repline::prebaked::*;
use std::{
    error::Error,
    io::{IsTerminal, stdin},
    marker::PhantomData,
};

fn banner() {
    println!("--- conlang v{} 💪🦈 ---", env!("CARGO_PKG_VERSION"))
}

fn clear() {
    print!("\x1b[H\x1b[2J\x1b[3J");
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut verbose = Verbosity::from(std::env::var("DO_VERBOSE").as_deref().unwrap_or_default());
    let mut parsing = ParseMode::from(std::env::var("DO_PARSING").as_deref().unwrap_or_default());
    let color = parsing.color();
    let begin = verbose.begin();
    banner();

    if stdin().is_terminal() {
        read_and_mut(color, begin, "  > ", |rl, line| match line.trim_end() {
            "" => Ok(Response::Continue),
            "exit" => Ok(Response::Break),
            "help" => {
                println!("Parsing: {parsing:?} (expr, pat, bind, use, tokens)");
                println!("Verbose: {verbose:?} (pretty, debug, quiet)");
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
            line @ ("tokens" | "expr" | "pat" | "bind" | "use") => {
                parsing = ParseMode::from(line);
                println!("Parse mode set to '{parsing:?}'");
                rl.set_color(parsing.color());
                Ok(Response::Accept)
            }
            line @ ("quiet" | "debug" | "debugpretty" | "dp" | "frob" | "pretty") => {
                verbose = Verbosity::from(line);
                println!("Verbosity set to '{verbose:?}'");
                rl.set_begin(verbose.begin());
                Ok(Response::Accept)
            }
            _ if line.ends_with("\n\n") => {
                parsing.with()(line, verbose);
                Ok(Response::Accept)
            }
            _ => Ok(Response::Continue),
        })?;
    } else {
        let doc = std::io::read_to_string(stdin())?;
        parsing.with()(&doc, verbose);
    }
    Ok(())
}

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

fn plural(count: usize) -> &'static str {
    match count {
        1 => "",
        _ => "s",
    }
}

fn tokens<'t, T: Parse<'t> + ?Sized>(document: &'t str, verbose: Verbosity) {
    let _: PhantomData<T>; // for lifetime variance
    let mut lexer = Lexer::new("<interactive>".into(), document);
    loop {
        match (lexer.scan(), verbose) {
            (Err(LexError { res: EOF, .. }), _) => {
                break;
            }
            (Err(e), _) => {
                println!("\x1b[31m{e}\x1b[0m");
                break;
            }
            (Ok(Token { lexeme, kind, span: Span { path: _, head, tail } }), Verbosity::Pretty) => {
                println!("{kind:?}\x1b[11G {head:<4} {tail:<4} {lexeme:?}")
            }
            (Ok(token), Verbosity::Debug) => {
                println!("{token:?}")
            }
            _ => {}
        }
    }
}

fn parse<'t, T>(document: &'t str, verbose: Verbosity)
where
    T: Parse<'t>
        + Annotation
        + for<'a> Walk<'a, DefaultTypes>
        + Foldable<DefaultTypes, DefaultTypes>,
    <T as Foldable<DefaultTypes, DefaultTypes>>::Out: Annotation,
{
    let mut parser = Parser::new(Lexer::new("<interactive>".into(), document));
    for idx in 0..6 {
        match (
            parser
                .parse::<At<T, _>>(T::Prec::default())
                .map(inline_modules),
            verbose,
        ) {
            (Err(e @ ParseError::EOF(_)), _) => {
                println!(
                    "\x1b[92m{e} (total {} byte{}, {idx} expression{})\x1b[0m",
                    document.len(),
                    plural(document.len()),
                    plural(idx),
                );
                break;
            }
            (Err(e), _) => {
                println!("\x1b[91m{e}\x1b[0m");
                break;
            }
            (Ok(At(expr, span)), Verbosity::Pretty) => {
                println!("\x1b[{}m{span:?}:\n{expr}", (idx + 5) % 6 + 31);
            }
            // (Ok(At(expr, span)), Verbosity::Frob) => {
            //     println!("\x1b[{}m{span}:\n", (idx + 5) % 6 + 31);
            //     let _ = expr.visit_in(&mut Collector::new());
            // }
            (Ok(expr), Verbosity::Debug) => {
                println!("\x1b[{}m{expr:?}", (idx + 5) % 6 + 31);
            }
            (Ok(expr), Verbosity::DebugPretty) => {
                println!("\x1b[{}m{expr:#?}", (idx + 5) % 6 + 31);
            }
            _ => {}
        }
    }
}

fn inline_modules<T>(expr: At<T>) -> At<T::Out>
where
    T: Annotation + Foldable<DefaultTypes, DefaultTypes>,
    T::Out: Annotation,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Verbosity {
    #[default]
    Pretty,
    Debug,
    DebugPretty,
    Frob,
    Quiet,
}

impl From<&str> for Verbosity {
    fn from(value: &str) -> Self {
        match value {
            "quiet" | "false" | "0" | "no" => Verbosity::Quiet,
            "debug" | "d" => Verbosity::Debug,
            "debugpretty" | "debug_pretty" | "dp" => Verbosity::DebugPretty,
            "frob" => Verbosity::Frob,
            "pretty" => Verbosity::Pretty,
            _ => Default::default(),
        }
    }
}

impl Verbosity {
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ParseMode {
    #[default]
    Expr,
    Pat,
    Bind,
    Use,
    Tokens,
}

impl From<&str> for ParseMode {
    fn from(value: &str) -> Self {
        match value {
            "expr" => Self::Expr,
            "pat" => Self::Pat,
            "bind" => Self::Bind,
            "use" => Self::Use,
            "tokens" => Self::Tokens,
            _ => Default::default(),
        }
    }
}
impl ParseMode {
    fn with<'a>(&self) -> fn(&'a str, Verbosity) {
        match self {
            Self::Expr => parse::<'a, Expr>,
            Self::Pat => parse::<'a, Pat>,
            Self::Bind => parse::<'a, Bind>,
            Self::Use => parse::<'a, Use>,
            Self::Tokens => tokens::<'a, dyn Parse<'a, Prec = ()>>,
        }
    }

    fn color(&self) -> &'static str {
        match self {
            Self::Expr => "\x1b[36m",
            Self::Pat => "\x1b[35m",
            Self::Bind => "\x1b[34m",
            Self::Use => "\x1b[33m",
            Self::Tokens => "\x1b[32m",
        }
    }
}
