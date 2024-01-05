//! Utilities for cl-frontend

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
    const HELP: &str = "[( --repl | --no-repl )] [( -f | --file ) <filename>]";

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
    use std::io::{Result as IOResult, Write};

    use conlang::{
        ast::preamble::{expression::Expr, *},
        interpreter::{env::Environment, error::IResult},
        lexer::Lexer,
        parser::{error::PResult, Parser},
        pretty_printer::{PrettyPrintable, Printer},
        resolver::{error::TyResult, Resolve, Resolver},
        token::Token,
    };

    pub struct Tokenized {
        tokens: Vec<Token>,
    }

    pub enum Parsed {
        Program(Start),
        Expr(Expr),
    }

    impl TryFrom<Tokenized> for Parsed {
        type Error = conlang::parser::error::Error;
        fn try_from(value: Tokenized) -> Result<Self, Self::Error> {
            let mut parser = Parser::new(value.tokens);
            let ast = parser.parse()?;
            //if let Ok(ast) = ast {
            //return
            Ok(Self::Program(ast))
            //}

            //Ok(Self::Expr(parser.reset().parse_expr()?))
        }
    }

    pub struct Program<Variant> {
        data: Variant,
    }

    impl Program<Tokenized> {
        pub fn new(input: &str) -> Result<Self, Vec<conlang::lexer::error::Error>> {
            let mut tokens = vec![];
            let mut errors = vec![];
            for token in Lexer::new(input) {
                match token {
                    Ok(token) => tokens.push(token),
                    Err(error) => errors.push(error),
                }
            }
            if errors.is_empty() {
                Ok(Self { data: Tokenized { tokens } })
            } else {
                Err(errors)
            }
        }
        pub fn tokens(&self) -> &[Token] {
            &self.data.tokens
        }
        pub fn parse(self) -> PResult<Program<Parsed>> {
            Ok(Program { data: Parsed::try_from(self.data)? })
        }
    }

    impl Program<Parsed> {
        pub fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<()> {
            match &mut self.data {
                Parsed::Program(start) => start.resolve(resolver),
                Parsed::Expr(expr) => expr.resolve(resolver),
            }
            .map(|ty| println!("{ty}"))
        }
        /// Runs the [Program] in the specified [Interpreter]
        pub fn run(&self, env: &mut Environment) -> IResult<()> {
            println!(
                "{}",
                match &self.data {
                    Parsed::Program(start) => env.eval(start)?,
                    Parsed::Expr(expr) => env.eval(expr)?,
                }
            );
            Ok(())
        }
    }

    impl PrettyPrintable for Program<Parsed> {
        fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
            match &self.data {
                Parsed::Program(value) => value.visit(p),
                Parsed::Expr(value) => value.visit(p),
            }
        }
    }
}

pub mod cli {
    use conlang::{
        interpreter::env::Environment, pretty_printer::PrettyPrintable, resolver::Resolver,
        token::Token,
    };

    use crate::{
        args::Args,
        program::{Parsed, Program, Tokenized},
    };
    use std::{
        convert::Infallible,
        error::Error,
        io::{stdin, stdout, Write},
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
            match (mode, program) {
                (Mode::Tokenize, Ok(program)) => {
                    for token in program.tokens() {
                        if let Some(path) = path {
                            print!("{}:", path.display());
                        }
                        print_token(token)
                    }
                }
                (Mode::Beautify, Ok(program)) => Self::beautify(program),
                (Mode::Resolve, Ok(program)) => Self::resolve(program, Default::default()),
                (Mode::Interpret, Ok(program)) => Self::interpret(program, Environment::new()),
                (_, Err(errors)) => {
                    for error in errors {
                        if let Some(path) = path {
                            eprint!("{}:", path.display());
                        }
                        eprintln!("{error}")
                    }
                }
            }
        }
        fn beautify(program: Program<Tokenized>) {
            match program.parse() {
                Ok(program) => program.print(),
                Err(e) => eprintln!("{e}"),
            };
        }
        fn resolve(program: Program<Tokenized>, mut resolver: Resolver) {
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
        fn interpret(program: Program<Tokenized>, mut interpreter: Environment) {
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
                "i" | "interpret" => Mode::Interpret,
                "b" | "beautify" | "p" | "pretty" => Mode::Beautify,
                "r" | "resolve" | "typecheck" => Mode::Resolve,
                "t" | "tokenize" => Mode::Tokenize,
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
        pub fn prompt_begin(&self) {
            print!(
                "{}{} {ANSI_RESET}",
                self.mode.ansi_color(),
                self.prompt_begin
            );
            let _ = stdout().flush();
        }
        pub fn prompt_again(&self) {
            print!(
                "{}{} {ANSI_RESET}",
                self.mode.ansi_color(),
                self.prompt_again
            );
            let _ = stdout().flush();
        }
        pub fn prompt_error(&self, err: &impl Error) {
            println!("{ANSI_RED}{} {err}{ANSI_RESET}", self.prompt_error)
        }
        pub fn begin_output(&self) {
            print!("{ANSI_OUTPUT}")
        }
        fn reprompt(&self, buf: &mut String) {
            self.prompt_begin();
            buf.clear();
        }
    }
    /// The actual REPL
    impl Repl {
        /// Constructs a new [Repl] with the provided [Mode]
        pub fn new(mode: Mode) -> Self {
            Self { mode, ..Default::default() }
        }
        /// Runs the main REPL loop
        pub fn repl(&mut self) {
            let mut buf = String::new();
            self.prompt_begin();
            while let Ok(len) = stdin().read_line(&mut buf) {
                // Exit the loop
                if len == 0 {
                    println!();
                    break;
                }
                self.begin_output();
                // Process mode-change commands
                if self.command(&buf) {
                    self.reprompt(&mut buf);
                    continue;
                }
                // Lex the buffer, or reset and output the error
                let code = match Program::new(&buf) {
                    Ok(code) => code,
                    Err(e) => {
                        for error in e {
                            eprintln!("{error}");
                        }
                        self.reprompt(&mut buf);
                        continue;
                    }
                };
                // Tokenize mode doesn't require valid parse, so it gets processed first
                if self.mode == Mode::Tokenize {
                    self.tokenize(&code);
                    self.reprompt(&mut buf);
                    continue;
                }
                // Parse code and dispatch to the proper function
                match (len, code.parse()) {
                    // If the code is OK, run it and print any errors
                    (_, Ok(mut code)) => {
                        self.dispatch(&mut code);
                        buf.clear();
                    }
                    // If the user types two newlines, print syntax errors
                    (1, Err(e)) => {
                        self.prompt_error(&e);
                        buf.clear();
                    }
                    // Otherwise, ask for more input
                    _ => {
                        self.prompt_again();
                        continue;
                    }
                }
                self.prompt_begin()
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
                "$mode" => print!("{:?} Mode", self.mode),
                "$help" => self.help(),
                _ => return false,
            }
            println!();
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
        fn tokenize(&mut self, code: &Program<Tokenized>) {
            for token in code.tokens() {
                print_token(token);
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
