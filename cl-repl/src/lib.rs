//! Utilities for cl-frontend
//!
//! # TODO
//! - [ ] Readline-like line editing
//! - [ ] Raw mode?
#![warn(clippy::all)]

pub mod ansi {
    // ANSI color escape sequences
    pub const ANSI_RED: &str = "\x1b[31m";
    pub const ANSI_GREEN: &str = "\x1b[32m"; // the color of type checker mode
    pub const ANSI_CYAN: &str = "\x1b[36m";
    // pub const ANSI_BRIGHT_GREEN: &str = "\x1b[92m";
    pub const ANSI_BRIGHT_BLUE: &str = "\x1b[94m";
    pub const ANSI_BRIGHT_MAGENTA: &str = "\x1b[95m";
    // const ANSI_BRIGHT_CYAN: &str = "\x1b[96m";
    pub const ANSI_RESET: &str = "\x1b[0m";
    pub const ANSI_OUTPUT: &str = "\x1b[38;5;117m";

    pub const ANSI_CLEAR_LINES: &str = "\x1b[G\x1b[J";
}

pub mod args {
    use crate::tools::is_terminal;
    use argh::FromArgs;
    use std::{path::PathBuf, str::FromStr};

    /// The Conlang prototype debug interface
    #[derive(Clone, Debug, FromArgs, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Args {
        /// the main source file
        #[argh(positional)]
        pub file: Option<PathBuf>,

        /// files to include
        #[argh(option, short = 'I')]
        pub include: Vec<PathBuf>,

        /// the Repl mode to start in
        #[argh(option, short = 'm', default = "Default::default()")]
        pub mode: Mode,

        /// whether to start the repl (`true` or `false`)
        #[argh(option, short = 'r', default = "is_terminal()")]
        pub repl: bool,
    }

    /// The CLI's operating mode
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
    pub enum Mode {
        Tokenize,
        Beautify,
        #[default]
        Interpret,
    }

    impl Mode {
        pub fn ansi_color(self) -> &'static str {
            use super::ansi::*;
            match self {
                Mode::Tokenize => ANSI_BRIGHT_BLUE,
                Mode::Beautify => ANSI_BRIGHT_MAGENTA,
                Mode::Interpret => ANSI_CYAN,
            }
        }
    }

    impl FromStr for Mode {
        type Err = &'static str;
        fn from_str(s: &str) -> Result<Self, &'static str> {
            Ok(match s {
                "i" | "interpret" | "r" | "run" => Mode::Interpret,
                "b" | "beautify" | "p" | "pretty" => Mode::Beautify,
                "t" | "tokenize" | "token" => Mode::Tokenize,
                _ => Err("Recognized modes are: 'r' \"run\", 'p' \"pretty\", 't' \"token\"")?,
            })
        }
    }
}

pub mod program {
    use cl_ast::ast;
    use cl_interpret::{
        env::Environment, error::IResult, interpret::Interpret, temp_type_impl::ConValue,
    };
    use cl_lexer::Lexer;
    use cl_parser::{error::PResult, Parser};
    use std::fmt::Display;

    pub struct Parsable;

    pub enum Parsed {
        File(ast::File),
        Stmt(ast::Stmt),
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
            }
        }
        pub fn print(&self) {
            match &self.data {
                Parsed::File(v) => println!("{v}"),
                Parsed::Stmt(v) => println!("{v}"),
            };
        }

        pub fn run(&self, env: &mut Environment) -> IResult<ConValue> {
            match &self.data {
                Parsed::File(v) => v.interpret(env),
                Parsed::Stmt(v) => v.interpret(env),
            }
        }
    }

    impl<'t> Display for Program<'t, Parsed> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match &self.data {
                Parsed::File(v) => write!(f, "{v}"),
                Parsed::Stmt(v) => write!(f, "{v}"),
            }
        }
    }
}

pub mod cli {
    //! Implement's the command line interface
    use crate::{
        args::{Args, Mode},
        program::{Parsable, Program},
        repl::Repl,
        tools::print_token,
    };
    use cl_interpret::{env::Environment, temp_type_impl::ConValue};
    use cl_lexer::Lexer;
    use cl_parser::Parser;
    use std::{error::Error, path::Path};

    /// Run the command line interface
    pub fn run(args: Args) -> Result<(), Box<dyn Error>> {
        let Args { file, include, mode, repl } = args;

        let mut env = Environment::new();
        for path in include {
            load_file(&mut env, path)?;
        }

        if repl {
            if let Some(file) = file {
                load_file(&mut env, file)?;
            }
            Repl::with_env(mode, env).repl()
        } else {
            let code = match &file {
                Some(file) => std::fs::read_to_string(file)?,
                None => std::io::read_to_string(std::io::stdin())?,
            };
            let code = Program::new(&code);

            match mode {
                Mode::Tokenize => tokenize(code, file),
                Mode::Beautify => beautify(code),
                Mode::Interpret => interpret(code, &mut env),
            }?;
        }
        Ok(())
    }

    fn load_file(
        env: &mut Environment,
        path: impl AsRef<Path>,
    ) -> Result<ConValue, Box<dyn Error>> {
        let file = std::fs::read_to_string(path)?;
        let code = Parser::new(Lexer::new(&file)).file()?;
        env.eval(&code).map_err(Into::into)
    }

    fn tokenize(
        code: Program<Parsable>,
        path: Option<impl AsRef<Path>>,
    ) -> Result<(), Box<dyn Error>> {
        for token in code.lex() {
            if let Some(ref path) = path {
                print!("{}:", path.as_ref().display());
            }
            match token {
                Ok(token) => print_token(&token),
                Err(e) => println!("{e}"),
            }
        }
        Ok(())
    }

    fn beautify(code: Program<Parsable>) -> Result<(), Box<dyn Error>> {
        code.parse()?.print();
        Ok(())
    }

    fn interpret(code: Program<Parsable>, env: &mut Environment) -> Result<(), Box<dyn Error>> {
        match code.parse()?.run(env)? {
            ConValue::Empty => {}
            ret => println!("{ret}"),
        }
        if env.get("main").is_ok() {
            match env.call("main", &[])? {
                ConValue::Empty => {}
                ret => println!("{ret}"),
            }
        }
        Ok(())
    }
}

pub mod repl {
    use crate::{
        ansi::*,
        args::Mode,
        program::{Parsable, Parsed, Program},
        tools::print_token,
    };
    use cl_interpret::{env::Environment, temp_type_impl::ConValue};
    use std::fmt::Display;

    /// Implements the interactive interpreter
    #[derive(Clone, Debug)]
    pub struct Repl {
        prompt_again: &'static str, // " ?>"
        prompt_begin: &'static str, // "cl>"
        prompt_error: &'static str, // "! >"
        prompt_succs: &'static str, // " ->"
        env: Environment,
        mode: Mode,
    }

    impl Default for Repl {
        fn default() -> Self {
            Self {
                prompt_begin: "cl>",
                prompt_again: " ?>",
                prompt_error: "! >",
                prompt_succs: " =>",
                env: Default::default(),
                mode: Default::default(),
            }
        }
    }

    /// Prompt functions
    impl Repl {
        pub fn prompt_result<T: Display, E: Display>(&self, res: Result<T, E>) {
            match &res {
                Ok(v) => self.prompt_succs(v),
                Err(e) => self.prompt_error(e),
            }
        }
        pub fn prompt_error(&self, err: &impl Display) {
            let Self { prompt_error: prompt, .. } = self;
            println!("{ANSI_CLEAR_LINES}{ANSI_RED}{prompt} {err}{ANSI_RESET}")
        }
        pub fn prompt_succs(&self, value: &impl Display) {
            let Self { prompt_succs: _prompt, .. } = self;
            println!("{ANSI_GREEN}{value}{ANSI_RESET}")
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
        /// Constructs a new [Repl] with the provided [Mode] and [Environment]
        pub fn with_env(mode: Mode, env: Environment) -> Self {
            Self { mode, env, ..Default::default() }
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
                "Commands:\n- $tokens\n    Tokenize Mode:\n    Outputs information derived by the Lexer\n- $pretty\n    Beautify Mode:\n    Pretty-prints the input\n- $run\n    Interpret Mode:\n    Interprets the input using Conlang\'s work-in-progress interpreter\n- $mode\n    Prints the current mode\n- $help\n    Prints this help message"
            );
        }
        fn command(&mut self, line: &str) -> bool {
            let Some(line) = line.trim().strip_prefix('$') else {
                return false;
            };
            if let Ok(mode) = line.parse() {
                self.mode = mode;
            } else {
                match line {
                    "$run" => self.mode = Mode::Interpret,
                    "mode" => println!("{:?} Mode", self.mode),
                    "help" => self.help(),
                    _ => return false,
                }
            }
            true
        }
        /// Dispatches calls to repl functions based on the program
        fn dispatch(&mut self, code: &mut Program<Parsed>) {
            match self.mode {
                Mode::Tokenize => {}
                Mode::Beautify => self.beautify(code),
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
            match code.run(&mut self.env) {
                Ok(ConValue::Empty) => {}
                res => self.prompt_result(res),
            }
        }
        fn beautify(&mut self, code: &Program<Parsed>) {
            code.print()
        }
    }
}

pub mod tools {
    use cl_token::Token;
    use std::io::IsTerminal;
    /// Prints a token in the particular way cl-repl does
    pub fn print_token(t: &Token) {
        println!(
            "{:02}:{:02}: {:#19} │{}│",
            t.line(),
            t.col(),
            t.ty(),
            t.data(),
        )
    }
    /// gets whether stdin AND stdout are a terminal, for pipelining
    pub fn is_terminal() -> bool {
        std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
    }
}

pub mod repline;
