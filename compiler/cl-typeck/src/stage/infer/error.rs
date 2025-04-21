use cl_ast::Path;

use crate::handle::Handle;
use core::fmt;

/// An error produced during type inference
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InferenceError {
    AnnotationEval(crate::type_expression::Error),
    NotFound(Path),
    Mismatch(Handle, Handle),
    Recursive(Handle, Handle),
}

impl std::error::Error for InferenceError {}
impl fmt::Display for InferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InferenceError::AnnotationEval(error) => write!(f, "{error}"),
            InferenceError::NotFound(p) => write!(f, "Path not visible in scope: {p}"),
            InferenceError::Mismatch(a, b) => write!(f, "Type mismatch: {a:?} != {b:?}"),
            InferenceError::Recursive(_, _) => write!(f, "Recursive type!"),
        }
    }
}
