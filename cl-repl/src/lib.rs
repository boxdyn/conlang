//! Utilities for cl-frontend
//!
//! # TODO
//! - [ ] Readline-like line editing
//! - [ ] Raw mode?

pub mod args {
    use crate::cli::Mode;
    use std::{
        io::{stdin, IsTerminal},
        ops::Deref,
        path::{Path, PathBuf},
    };

    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Args {
        pub path: Option<PathBuf>, // defaults None
        pub repl: bool,            // defaults true if stdin is terminal
        pub mode: Mode,            // defaults Interpret
    }
    const HELP: &str = "[( --repl | --no-repl )] [--mode (tokens | pretty | type | run)] [( -f | --file ) <filename>]";

    impl Args {
        pub fn new() -> Self {
            Args { path: None, repl: stdin().is_terminal(), mode: Mode::Interpret }
        }
        pub fn parse(mut self) -> Option<Self> {
            let mut args = std::env::args();
            let name = args.next().unwrap_or_default();
            let mut unknown = false;
            while let Some(arg) = args.next() {
                match arg.deref() {
                    "--repl" => self.repl = true,
                    "--no-repl" => self.repl = false,
                    "-f" | "--file" => self.path = args.next().map(PathBuf::from),
                    "-m" | "--mode" => {
                        self.mode = args.next().unwrap_or_default().parse().unwrap_or_default()
                    }
                    arg => {
                        eprintln!("Unknown argument: {arg}");
                        unknown = true;
                    }
                }
            }
            if unknown {
                println!("Usage: {name} {HELP}");
                None?
            }
            Some(self)
        }
        /// Returns the path to a file, if one was specified
        pub fn path(&self) -> Option<&Path> {
            self.path.as_deref()
        }
        /// Returns whether to start a REPL session or not
        pub fn repl(&self) -> bool {
            self.repl
        }
        /// Returns the repl Mode
        pub fn mode(&self) -> Mode {
            self.mode
        }
    }
    impl Default for Args {
        fn default() -> Self {
            Self::new()
        }
    }
}

pub mod program {
    use std::{fmt::Display, io::Write};

    use conlang::{
        ast::{self, ast_impl::format::Pretty},
        interpreter::{
            env::Environment, error::IResult, interpret::Interpret, temp_type_impl::ConValue,
        },
        // pretty_printer::{PrettyPrintable, Printer},
        lexer::Lexer,
        parser::{error::PResult, Parser},
        resolver::{error::TyResult, Resolver},
    };

    pub struct Parsable;

    pub enum Parsed {
        File(ast::File),
        Stmt(ast::Stmt),
        Expr(ast::Expr),
    }

    pub struct Program<'t, Variant> {
        text: &'t str,
        data: Variant,
    }
    impl<'t, V> Program<'t, V> {
        pub fn lex(&self) -> Lexer {
            Lexer::new(self.text)
        }
    }

    impl<'t> Program<'t, Parsable> {
        pub fn new(text: &'t str) -> Self {
            Self { text, data: Parsable }
        }
        pub fn parse(self) -> PResult<Program<'t, Parsed>> {
            self.parse_file().or_else(|_| self.parse_stmt())
        }
        pub fn parse_expr(&self) -> PResult<Program<'t, Parsed>> {
            Ok(Program { data: Parsed::Expr(Parser::new(self.lex()).expr()?), text: self.text })
        }
        pub fn parse_stmt(&self) -> PResult<Program<'t, Parsed>> {
            Ok(Program { data: Parsed::Stmt(Parser::new(self.lex()).stmt()?), text: self.text })
        }
        pub fn parse_file(&self) -> PResult<Program<'t, Parsed>> {
            Ok(Program { data: Parsed::File(Parser::new(self.lex()).file()?), text: self.text })
        }
    }

    impl<'t> Program<'t, Parsed> {
        pub fn debug(&self) {
            match &self.data {
                Parsed::File(v) => eprintln!("{v:?}"),
                Parsed::Stmt(v) => eprintln!("{v:?}"),
                Parsed::Expr(v) => eprintln!("{v:?}"),
            }
        }
        pub fn print(&self) {
            let mut f = std::io::stdout().pretty();
            let _ = match &self.data {
                Parsed::File(v) => writeln!(f, "{v}"),
                Parsed::Stmt(v) => writeln!(f, "{v}"),
                Parsed::Expr(v) => writeln!(f, "{v}"),
            };
            // println!("{self}")
        }

        pub fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<()> {
            todo!("Program::resolve(\n{self},\n{resolver:?}\n)")
        }

        pub fn run(&self, env: &mut Environment) -> IResult<ConValue> {
            match &self.data {
                Parsed::File(v) => v.interpret(env),
                Parsed::Stmt(v) => v.interpret(env),
                Parsed::Expr(v) => v.interpret(env),
            }
        }

        // pub fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<()> {
        //     match &mut self.data {
        //         Parsed::Program(start) => start.resolve(resolver),
        //         Parsed::Expr(expr) => expr.resolve(resolver),
        //     }
        //     .map(|ty| println!("{ty}"))
        // }

        // /// Runs the [Program] in the specified [Environment]
        // pub fn run(&self, env: &mut Environment) -> IResult<()> {
        //     println!(
        //         "{}",
        //         match &self.data {
        //             Parsed::Program(start) => env.eval(start)?,
        //             Parsed::Expr(expr) => env.eval(expr)?,
        //         }
        //     );
        //     Ok(())
        // }
    }

    impl<'t> Display for Program<'t, Parsed> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match &self.data {
                Parsed::File(v) => write!(f, "{v}"),
                Parsed::Stmt(v) => write!(f, "{v}"),
                Parsed::Expr(v) => write!(f, "{v}"),
            }
        }
    }

    // impl PrettyPrintable for Program<Parsed> {
    //     fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
    //         match &self.data {
    //             Parsed::Program(value) => value.visit(p),
    //             Parsed::Expr(value) => value.visit(p),
    //         }
    //     }
    // }
}

pub mod cli {
    use conlang::{interpreter::env::Environment, resolver::Resolver, token::Token};

    use crate::{
        args::Args,
        program::{Parsable, Parsed, Program},
    };
    use std::{
        convert::Infallible,
        error::Error,
        path::{Path, PathBuf},
        str::FromStr,
    };

    // ANSI color escape sequences
    const ANSI_RED: &str = "\x1b[31m";
    const ANSI_GREEN: &str = "\x1b[32m";
    const ANSI_CYAN: &str = "\x1b[36m";
    const ANSI_BRIGHT_BLUE: &str = "\x1b[94m";
    const ANSI_BRIGHT_MAGENTA: &str = "\x1b[95m";
    // const ANSI_BRIGHT_CYAN: &str = "\x1b[96m";
    const ANSI_RESET: &str = "\x1b[0m";
    const ANSI_OUTPUT: &str = "\x1b[38;5;117m";

    const ANSI_CLEAR_LINES: &str = "\x1b[G\x1b[J";

    #[derive(Clone, Debug)]
    pub enum CLI {
        Repl(Repl),
        File { mode: Mode, path: PathBuf },
        Stdin { mode: Mode },
    }
    impl From<Args> for CLI {
        fn from(value: Args) -> Self {
            let Args { path, repl, mode } = value;
            match (repl, path) {
                (true, Some(path)) => {
                    let prog = std::fs::read_to_string(path).unwrap();
                    let code = conlang::parser::Parser::new(conlang::lexer::Lexer::new(&prog))
                        .file()
                        .unwrap();
                    let mut env = conlang::interpreter::env::Environment::new();
                    env.eval(&code).unwrap();
                    env.call("dump", &[])
                        .expect("calling dump in the environment shouldn't fail");

                    Self::Repl(Repl { mode, env, ..Default::default() })
                }
                (_, Some(path)) => Self::File { mode, path },
                (true, None) => Self::Repl(Repl { mode, ..Default::default() }),
                (false, None) => Self::Stdin { mode },
            }
        }
    }
    impl CLI {
        pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
            use std::{fs, io};
            match self {
                CLI::Repl(repl) => repl.repl(),
                CLI::File { mode, ref path } => {
                    // read file
                    Self::no_repl(*mode, Some(path), &fs::read_to_string(path)?)
                }
                CLI::Stdin { mode } => {
                    Self::no_repl(*mode, None, &io::read_to_string(io::stdin())?)
                }
            }
            Ok(())
        }
        fn no_repl(mode: Mode, path: Option<&Path>, code: &str) {
            let program = Program::new(code);
            match mode {
                Mode::Tokenize => {
                    for token in program.lex() {
                        if let Some(path) = path {
                            print!("{}:", path.display());
                        }
                        match token {
                            Ok(token) => print_token(&token),
                            Err(e) => println!("{e}"),
                        }
                    }
                }
                Mode::Beautify => Self::beautify(program),
                Mode::Resolve => Self::resolve(program, Default::default()),
                Mode::Interpret => Self::interpret(program, Environment::new()),
            }
        }
        fn beautify(program: Program<Parsable>) {
            match program.parse() {
                Ok(program) => program.print(),
                Err(e) => eprintln!("{e}"),
            };
        }
        fn resolve(program: Program<Parsable>, mut resolver: Resolver) {
            let mut program = match program.parse() {
                Ok(program) => program,
                Err(e) => {
                    eprintln!("{e}");
                    return;
                }
            };
            if let Err(e) = program.resolve(&mut resolver) {
                eprintln!("{e}");
            }
        }
        fn interpret(program: Program<Parsable>, mut interpreter: Environment) {
            let program = match program.parse() {
                Ok(program) => program,
                Err(e) => {
                    eprintln!("{e}");
                    return;
                }
            };
            if let Err(e) = program.run(&mut interpreter) {
                eprintln!("{e}");
                return;
            }
            if let Err(e) = interpreter.call("main", &[]) {
                eprintln!("{e}");
            }
        }
    }

    /// The CLI's operating mode
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
    pub enum Mode {
        Tokenize,
        Beautify,
        Resolve,
        #[default]
        Interpret,
    }
    impl Mode {
        pub fn ansi_color(self) -> &'static str {
            match self {
                Mode::Tokenize => ANSI_BRIGHT_BLUE,
                Mode::Beautify => ANSI_BRIGHT_MAGENTA,
                Mode::Resolve => ANSI_GREEN,
                Mode::Interpret => ANSI_CYAN,
            }
        }
    }
    impl FromStr for Mode {
        type Err = Infallible;
        fn from_str(s: &str) -> Result<Self, Infallible> {
            Ok(match s {
                "i" | "interpret" | "run" => Mode::Interpret,
                "b" | "beautify" | "p" | "pretty" => Mode::Beautify,
                "r" | "resolve" | "typecheck" | "type" => Mode::Resolve,
                "t" | "tokenize" | "tokens" => Mode::Tokenize,
                _ => Mode::Interpret,
            })
        }
    }

    /// Implements the interactive interpreter
    #[derive(Clone, Debug)]
    pub struct Repl {
        prompt_again: &'static str, // " ?>"
        prompt_begin: &'static str, // "cl>"
        prompt_error: &'static str, // "! >"
        env: Environment,
        resolver: Resolver,
        mode: Mode,
    }

    impl Default for Repl {
        fn default() -> Self {
            Self {
                prompt_begin: "cl>",
                prompt_again: " ?>",
                prompt_error: "! >",
                env: Default::default(),
                resolver: Default::default(),
                mode: Default::default(),
            }
        }
    }

    /// Prompt functions
    impl Repl {
        pub fn prompt_error(&self, err: &impl Error) {
            let Self { prompt_error: prompt, .. } = self;
            println!("{ANSI_CLEAR_LINES}{ANSI_RED}{prompt} {err}{ANSI_RESET}",)
        }
        /// Resets the cursor to the start of the line, clears the terminal,
        /// and sets the output color
        pub fn begin_output(&self) {
            print!("{ANSI_CLEAR_LINES}{ANSI_OUTPUT}")
        }
        pub fn clear_line(&self) {}
    }
    /// The actual REPL
    impl Repl {
        /// Constructs a new [Repl] with the provided [Mode]
        pub fn new(mode: Mode) -> Self {
            Self { mode, ..Default::default() }
        }
        /// Runs the main REPL loop
        pub fn repl(&mut self) {
            use crate::repline::{error::Error, Repline};
            let mut rl = Repline::new(self.mode.ansi_color(), self.prompt_begin, self.prompt_again);
            fn clear_line() {
                print!("\x1b[G\x1b[J");
            }
            loop {
                let buf = match rl.read() {
                    Ok(buf) => buf,
                    // Ctrl-C: break if current line is empty
                    Err(Error::CtrlC(buf)) => {
                        if buf.is_empty() || buf.ends_with('\n') {
                            return;
                        }
                        rl.accept();
                        println!("Cancelled. (Press Ctrl+C again to quit.)");
                        continue;
                    }
                    // Ctrl-D: reset input, and parse it for errors
                    Err(Error::CtrlD(buf)) => {
                        rl.deny();
                        if let Err(e) = Program::new(&buf).parse() {
                            clear_line();
                            self.prompt_error(&e);
                        }
                        continue;
                    }
                    Err(e) => {
                        self.prompt_error(&e);
                        return;
                    }
                };

                self.begin_output();
                if self.command(&buf) {
                    rl.deny();
                    rl.set_color(self.mode.ansi_color());
                    continue;
                }
                let code = Program::new(&buf);
                if self.mode == Mode::Tokenize {
                    self.tokenize(&code);
                    rl.deny();
                    continue;
                }
                match code.lex().into_iter().find(|l| l.is_err()) {
                    None => {}
                    Some(Ok(_)) => unreachable!(),
                    Some(Err(error)) => {
                        rl.deny();
                        self.prompt_error(&error);
                        continue;
                    }
                }
                if let Ok(mut code) = code.parse() {
                    rl.accept();
                    self.dispatch(&mut code);
                }
            }
        }

        fn help(&self) {
            println!(
                "Commands:\n- $tokens\n    Tokenize Mode:\n    Outputs information derived by the Lexer\n- $pretty\n    Beautify Mode:\n    Pretty-prints the input\n- $type\n    Resolve Mode:\n    Attempts variable resolution and type-checking on the input\n- $run\n    Interpret Mode:\n    Interprets the input using Conlang\'s work-in-progress interpreter\n- $mode\n    Prints the current mode\n- $help\n    Prints this help message"
            );
        }
        fn command(&mut self, line: &str) -> bool {
            match line.trim() {
                "$pretty" => self.mode = Mode::Beautify,
                "$tokens" => self.mode = Mode::Tokenize,
                "$type" => self.mode = Mode::Resolve,
                "$run" => self.mode = Mode::Interpret,
                "$mode" => println!("{:?} Mode", self.mode),
                "$help" => self.help(),
                _ => return false,
            }
            true
        }
        /// Dispatches calls to repl functions based on the program
        fn dispatch(&mut self, code: &mut Program<Parsed>) {
            match self.mode {
                Mode::Tokenize => (),
                Mode::Beautify => self.beautify(code),
                Mode::Resolve => self.typecheck(code),
                Mode::Interpret => self.interpret(code),
            }
        }
        fn tokenize(&mut self, code: &Program<Parsable>) {
            for token in code.lex() {
                match token {
                    Ok(token) => print_token(&token),
                    Err(e) => println!("{e}"),
                }
            }
        }
        fn interpret(&mut self, code: &Program<Parsed>) {
            if let Err(e) = code.run(&mut self.env) {
                self.prompt_error(&e)
            }
        }
        fn typecheck(&mut self, code: &mut Program<Parsed>) {
            if let Err(e) = code.resolve(&mut self.resolver) {
                self.prompt_error(&e)
            }
        }
        fn beautify(&mut self, code: &Program<Parsed>) {
            code.print()
        }
    }

    fn print_token(t: &Token) {
        println!(
            "{:02}:{:02}: {:#19} │{}│",
            t.line(),
            t.col(),
            t.ty(),
            t.data(),
        )
    }
}

pub mod repline;
