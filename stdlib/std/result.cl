//! The Result type, indicating a fallible operation.
use super::preamble::*;

pub enum Result<T, E> {
    Ok(T),
    Err(E),
}

impl Result {
    pub fn is_ok(&self) -> bool {
        match self {
            Ok(_) => true,
            Err(_) => false,
        }
    }
    pub fn is_err(&self) -> bool {
        match self {
            Ok(_) => false,
            Err(_) => true,
        }
    }
    /// Maps the value inside the Result::Ok, leaving errors alone.
    // pub fn map<U>(self, f: fn(T) -> U) -> Result<U, E> {
    //     match self {
    //         Ok(t) => Ok(f(t)),
    //         Err(e) => Err(e),
    //     }
    // }
    /// Maps the value inside the Result::Err, leaving values alone.
    // pub fn map_err<F>(self, f: fn(E) -> F) -> Result<T, F> {
    //     match self {
    //         Ok(t) => Ok(t),
    //         Err(e) => Err(f(e)),
    //     }
    // }
}
