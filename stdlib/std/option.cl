//! The optional type, representing the presence or absence of a thing.
use super::preamble::*;

pub enum Option<T> {
    Some(T),
    None,
}

impl Option {
    pub fn is_some(self: &Self) -> bool {
        match self {
            Some(_) => true,
            None => false,
        }
    }
    pub fn is_none(self: &Self) -> bool {
        match self {
            Some(_) => false,
            None => true,
        }
    }
}
