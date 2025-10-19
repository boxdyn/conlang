use cl_ast::Path;

use crate::handle::Handle;
use core::fmt;

/// An error produced during type inference
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InferenceError {
    AnnotationEval(crate::type_expression::Error),
    FieldCount(Handle, usize, usize),
    NotFound(Path),
    Mismatch(Handle, Handle),
    Recursive(Handle, Handle),
    NoBreak,
    NoReturn,
}

impl std::error::Error for InferenceError {}
#[rustfmt::skip]
impl fmt::Display for InferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InferenceError::AnnotationEval(error) => write!(f, "{error}"),
            InferenceError::FieldCount(name, want, got) => {
                write!(f, 
                    "Struct {name} {} fields! Expected {want}, got {got}",
                    if want < got { "has too many" } else { "is missing" }
                )
            }
            InferenceError::NotFound(p) => write!(f, "Path not visible in scope: {p}"),
            InferenceError::Mismatch(a, b) => write!(f, "Type mismatch: {a:?} != {b:?}"),
            InferenceError::Recursive(_, _) => write!(f, "Recursive type!"),
            InferenceError::NoBreak => write!(f, "Encountered break outside loop!"),
            InferenceError::NoReturn => write!(f, "Encountered return outside function!"),
        }
    }
}

impl From<crate::type_expression::Error> for InferenceError {
    fn from(value: crate::type_expression::Error) -> Self {
        Self::AnnotationEval(value)
    }
}
