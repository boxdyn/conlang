//! Handles argument parsing (currently using the [argwerk] crate)

use std::{io::IsTerminal, path::PathBuf, str::FromStr};

argwerk::define! {
    ///
    ///The Conlang prototype debug interface
    #[usage = "conlang [<file>] [-I <include...>] [-m <mode>] [-r <repl>]"]
    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Args {
        pub file: Option<PathBuf>,
        pub include: Vec<PathBuf>,
        pub mode: Mode,
        pub repl: bool = is_terminal(),
    }

    ///files to include
    ["-I" | "--include", path] => {
        include.push(path.into());
    }
    ///the CLI operating mode (`f`mt | `l`ex | `r`un)
    ["-m" | "--mode", flr] => {
        mode = flr.parse()?;
    }
    ///whether to start the repl (`true` or `false`)
    ["-r" | "--repl", bool] => {
        repl = bool.parse()?;
    }
    ///display usage information
    ["-h" | "--help"] => {
        println!("{}", Args::help());
        if true { std::process::exit(0); }
    }
    ///the main source file
    [#[option] path] if file.is_none() => {
        file = path.map(Into::into);
    }

    [path] if file.is_some() => {
        include.push(path.into());
    }
}

/// gets whether stdin AND stdout are a terminal, for pipelining
pub fn is_terminal() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

/// The CLI's operating mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mode {
    Lex,
    Fmt,
    #[default]
    Run,
}

impl FromStr for Mode {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, &'static str> {
        Ok(match s {
            "f" | "fmt" | "p" | "pretty" => Mode::Fmt,
            "l" | "lex" | "tokenize" | "token" => Mode::Lex,
            "r" | "run" => Mode::Run,
            _ => Err("Recognized modes are: 'r' \"run\", 'f' \"fmt\", 'l' \"lex\"")?,
        })
    }
}
