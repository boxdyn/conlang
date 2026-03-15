//! The [Error] type represents any error thrown by the [Environment](super::Environment)

use cl_ast::{Pat, types::Symbol};
use cl_structures::span::Span;

use super::convalue::ConValue;

pub type IResult<T> = Result<T, Error>;

#[derive(Clone, Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub(super) span: Option<Span>,
}

impl Error {
    #![allow(non_snake_case)]

    /// Adds a [struct Span] to this [Error], if there isn't already a more specific one.
    pub fn with_span(self, span: Span) -> Self {
        Self { span: self.span.or(Some(span)), ..self }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Propagate a Return value
    pub fn Return(value: ConValue) -> Self {
        Self { kind: ErrorKind::Return(value), span: None }
    }
    /// Propagate a Break value
    pub fn Break(value: ConValue) -> Self {
        Self { kind: ErrorKind::Break(value), span: None }
    }
    /// Break propagated across function bounds
    pub fn BadBreak(value: ConValue) -> Self {
        Self { kind: ErrorKind::BadBreak(value), span: None }
    }
    /// Continue to the next iteration of a loop
    pub fn Continue() -> Self {
        Self { kind: ErrorKind::Continue, span: None }
    }
    /// Underflowed the stack
    pub fn StackUnderflow() -> Self {
        Self { kind: ErrorKind::StackUnderflow, span: None }
    }
    /// Overflowed the stack
    pub fn StackOverflow(place: usize) -> Self {
        Self { kind: ErrorKind::StackOverflow(place), span: None }
    }
    /// Exited the last scope
    pub fn ScopeExit() -> Self {
        Self { kind: ErrorKind::ScopeExit, span: None }
    }
    /// Type incompatibility
    // TODO: store the type information in this error
    pub fn TypeError(want: &'static str, got: &'static str) -> Self {
        Self { kind: ErrorKind::TypeError(want, got), span: None }
    }
    /// In clause of For loop didn't yield a Range
    pub fn NotIterable() -> Self {
        Self { kind: ErrorKind::NotIterable, span: None }
    }
    /// A value could not be indexed
    pub fn NotIndexable() -> Self {
        Self { kind: ErrorKind::NotIndexable, span: None }
    }
    /// An array index went out of bounds
    pub fn OobIndex(index: usize, length: usize) -> Self {
        Self { kind: ErrorKind::OobIndex(index, length), span: None }
    }
    /// An expression in place position is not a place-expression
    pub fn NotPlace() -> Self {
        Self { kind: ErrorKind::NotPlace, span: None }
    }
    /// A name was not defined in scope before being used
    pub fn NotDefined(name: Symbol) -> Self {
        Self { kind: ErrorKind::NotDefined(name), span: None }
    }
    /// A name was defined but not initialized
    pub fn NotInitialized(name: Symbol) -> Self {
        Self { kind: ErrorKind::NotInitialized(name), span: None }
    }
    /// A value was called, but is not callable
    pub fn NotCallable(value: ConValue) -> Self {
        Self { kind: ErrorKind::NotCallable(value), span: None }
    }
    /// A function was called with the wrong number of arguments
    pub fn ArgNumber(want: usize, got: usize) -> Self {
        Self { kind: ErrorKind::ArgNumber { want, got }, span: None }
    }
    /// A pattern failed to match
    pub fn PatFailed(pat: Box<Pat>) -> Self {
        Self { kind: ErrorKind::PatFailed(pat), span: None }
    }
    /// Fell through a non-exhaustive match
    pub fn MatchNonexhaustive() -> Self {
        Self { kind: ErrorKind::MatchNonexhaustive, span: None }
    }
    /// Explicit panic
    pub fn Panic(msg: String) -> Self {
        Self { kind: ErrorKind::Panic(msg, 0), span: None }
    }
    /// Error produced by a Builtin
    pub fn BuiltinError(msg: String) -> Self {
        Self { kind: ErrorKind::BuiltinError(msg), span: None }
    }
}

impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { kind, span } = self;
        if let Some(Span { path, head, tail }) = span {
            write!(f, "{path}:{head}..{tail}: ")?;
        }
        write!(f, "{kind}")
    }
}

/// Represents any error thrown by the [Environment](super::Environment)
#[derive(Clone, Debug)]
pub enum ErrorKind {
    /// Propagate a Return value
    Return(ConValue),
    /// Propagate a Break value
    Break(ConValue),
    /// Break propagated across function bounds
    BadBreak(ConValue),
    /// Continue to the next iteration of a loop
    Continue,
    /// Underflowed the stack
    StackUnderflow,
    /// Overflowed the stack
    StackOverflow(usize),
    /// Exited the last scope
    ScopeExit,
    /// Type incompatibility
    // TODO: store the type information in this error
    TypeError(&'static str, &'static str),
    /// In clause of For loop didn't yield a Range
    NotIterable,
    /// A value could not be indexed
    NotIndexable,
    /// An array index went out of bounds
    OobIndex(usize, usize),
    /// An expression is not assignable
    NotPlace,
    /// A name was not defined in scope before being used
    NotDefined(Symbol),
    /// A name was defined but not initialized
    NotInitialized(Symbol),
    /// A value was called, but is not callable
    NotCallable(ConValue),
    /// A function was called with the wrong number of arguments
    ArgNumber { want: usize, got: usize },
    /// A pattern failed to match
    PatFailed(Box<Pat>),
    /// Fell through a non-exhaustive match
    MatchNonexhaustive,
    /// Explicit panic
    Panic(String, usize),
    /// Error produced by a Builtin
    BuiltinError(String),
}

impl std::error::Error for ErrorKind {}
impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::Return(value) => write!(f, "return {value}"),
            ErrorKind::Break(value) => write!(f, "break {value}"),
            ErrorKind::BadBreak(value) => write!(f, "rogue break: {value}"),
            ErrorKind::Continue => "continue".fmt(f),
            ErrorKind::StackUnderflow => "Stack underflow".fmt(f),
            ErrorKind::StackOverflow(id) => {
                write!(f, "Attempt to access <{id}> resulted in stack overflow.")
            }
            ErrorKind::ScopeExit => "Exited the last scope. This is a logic bug.".fmt(f),
            ErrorKind::TypeError(want, got) => {
                write!(f, "Incompatible types: wanted {want}, got {got}")
            }
            ErrorKind::NotIterable => "`in` clause of `for` loop did not yield an iterable".fmt(f),
            ErrorKind::NotIndexable => {
                write!(f, "expression cannot be indexed")
            }
            ErrorKind::OobIndex(idx, len) => {
                write!(f, "Index out of bounds: index was {idx}. but len is {len}")
            }
            ErrorKind::NotPlace => {
                write!(f, "expression does not refer to a place")
            }
            ErrorKind::NotDefined(value) => {
                write!(f, "{value} not bound.")
            }
            ErrorKind::NotInitialized(value) => {
                write!(f, "{value} bound, but not initialized")
            }
            ErrorKind::NotCallable(value) => {
                write!(f, "{value} is not callable.")
            }
            ErrorKind::ArgNumber { want, got } => {
                write!(
                    f,
                    "Expected {want} argument{}, got {got}",
                    if *want == 1 { "" } else { "s" }
                )
            }
            ErrorKind::PatFailed(pattern) => {
                write!(f, "Failed to match pattern {pattern}")
            }
            ErrorKind::MatchNonexhaustive => {
                write!(f, "Fell through a non-exhaustive match expression!")
            }
            ErrorKind::Panic(s, _depth) => write!(f, "{s}"),
            ErrorKind::BuiltinError(s) => write!(f, "{s}"),
        }
    }
}
