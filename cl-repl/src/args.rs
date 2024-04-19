//! Handles argument parsing (currently using the [argh] crate)

use argh::FromArgs;
use std::{io::IsTerminal, path::PathBuf, str::FromStr};

/// The Conlang prototype debug interface
#[derive(Clone, Debug, FromArgs, PartialEq, Eq, PartialOrd, Ord)]
pub struct Args {
    /// the main source file
    #[argh(positional)]
    pub file: Option<PathBuf>,

    /// files to include
    #[argh(option, short = 'I')]
    pub include: Vec<PathBuf>,

    /// the CLI operating mode (`f`mt | `l`ex | `r`un)
    #[argh(option, short = 'm', default = "Default::default()")]
    pub mode: Mode,

    /// whether to start the repl (`true` or `false`)
    #[argh(option, short = 'r', default = "is_terminal()")]
    pub repl: bool,
}

/// gets whether stdin AND stdout are a terminal, for pipelining
pub fn is_terminal() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

/// The CLI's operating mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mode {
    #[default]
    Menu,
    Lex,
    Fmt,
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
