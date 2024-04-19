use crate::iter::chars::BadUnicode;

/// Result type for Repline
pub type ReplResult<T> = std::result::Result<T, Error>;
/// Borrowed error (does not implement [Error](std::error::Error)!)
#[derive(Debug)]
pub enum Error {
    /// User broke with Ctrl+C
    CtrlC(String),
    /// User broke with Ctrl+D
    CtrlD(String),
    /// Invalid unicode codepoint
    BadUnicode(u32),
    /// Error came from [std::io]
    IoFailure(std::io::Error),
    /// End of input
    EndOfInput,
}

impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::CtrlC(_) => write!(f, "Ctrl+C"),
            Error::CtrlD(_) => write!(f, "Ctrl+D"),
            Error::BadUnicode(u) => write!(f, "\\u{{{u:x}}} is not a valid unicode codepoint"),
            Error::IoFailure(s) => write!(f, "{s}"),
            Error::EndOfInput => write!(f, "End of input"),
        }
    }
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IoFailure(value)
    }
}
impl From<BadUnicode> for Error {
    fn from(value: BadUnicode) -> Self {
        let BadUnicode(code) = value;
        Self::BadUnicode(code)
    }
}
