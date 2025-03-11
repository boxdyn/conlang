//! The [Error] type represents any error thrown by the [Environment](super::Environment)

use cl_ast::{Pattern, Sym};
use cl_structures::span::Span;

use super::convalue::ConValue;

pub type IResult<T> = Result<T, Error>;

/// Represents any error thrown by the [Environment](super::Environment)
#[derive(Clone, Debug)]
pub enum Error {
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
    /// Exited the last scope
    ScopeExit,
    /// Type incompatibility
    // TODO: store the type information in this error
    TypeError,
    /// In clause of For loop didn't yield a Range
    NotIterable,
    /// A value could not be indexed
    NotIndexable,
    /// An array index went out of bounds
    OobIndex(usize, usize),
    /// An expression is not assignable
    NotAssignable,
    /// A name was not defined in scope before being used
    NotDefined(Sym),
    /// A name was defined but not initialized
    NotInitialized(Sym),
    /// A value was called, but is not callable
    NotCallable(ConValue),
    /// A function was called with the wrong number of arguments
    ArgNumber { want: usize, got: usize },
    /// A pattern failed to match
    PatFailed(Box<Pattern>),
    /// Fell through a non-exhaustive match
    MatchNonexhaustive,
    /// Error produced by a Builtin
    BuiltinDebug(String),
    /// Error with associated line information
    WithSpan(Box<Error>, Span),
}

impl Error {
    /// Adds a [Span] to this [Error], if there isn't already a more specific one.
    pub fn with_span(self, span: Span) -> Self {
        match self {
            Self::WithSpan(..) => self,
            err => Self::WithSpan(Box::new(err), span),
        }
    }
}

impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Return(value) => write!(f, "return {value}"),
            Error::Break(value) => write!(f, "break {value}"),
            Error::BadBreak(value) => write!(f, "rogue break: {value}"),
            Error::Continue => "continue".fmt(f),
            Error::StackUnderflow => "Stack underflow".fmt(f),
            Error::ScopeExit => "Exited the last scope. This is a logic bug.".fmt(f),
            Error::TypeError => "Incompatible types".fmt(f),
            Error::NotIterable => "`in` clause of `for` loop did not yield an iterable".fmt(f),
            Error::NotIndexable => {
                write!(f, "expression cannot be indexed")
            }
            Error::OobIndex(idx, len) => {
                write!(f, "Index out of bounds: index was {idx}. but len is {len}")
            }
            Error::NotAssignable => {
                write!(f, "expression is not assignable")
            }
            Error::NotDefined(value) => {
                write!(f, "{value} not bound. Did you mean `let {value};`?")
            }
            Error::NotInitialized(value) => {
                write!(f, "{value} bound, but not initialized")
            }
            Error::NotCallable(value) => {
                write!(f, "{value} is not callable.")
            }
            Error::ArgNumber { want, got } => {
                write!(
                    f,
                    "Expected {want} argument{}, got {got}",
                    if *want == 1 { "" } else { "s" }
                )
            }
            Error::PatFailed(pattern) => {
                write!(f, "Failed to match pattern {pattern}")
            }
            Error::MatchNonexhaustive => {
                write!(f, "Fell through a non-exhaustive match expression!")
            }
            Error::BuiltinDebug(s) => write!(f, "DEBUG: {s}"),
            Error::WithSpan(e, span) => write!(f, "{}..{}: {e}", span.head, span.tail),
        }
    }
}
