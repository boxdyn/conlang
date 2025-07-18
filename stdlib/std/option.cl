//! The optional type, representing the presence or absence of a thing.
use super::preamble::*;

pub enum Option<T> {
    Some(T),
    None,
}

impl<T> Option<T> {
    // pub fn is_some(self) -> bool {
    //     match self {
    //         Option::Some(_) => true,
    //         Option::None() => false,
    //     }
    // }
    // pub fn is_none(self: &Self) -> bool {
    //     match self {
    //         Option::Some(_) => false,
    //         Option::None() => true,
    //     }
    // }
    // /// Maps from one option space to another
    // pub fn map<U>(self: Self, f: fn(T) -> U) -> Option<U> {
    //     match self {
    //         Option::Some(value) => Option::Some(f(value)),
    //         Option::None() => Option::None(),
    //     }
    // }

    // pub fn and_then<U>(self: Self, f: fn(T) -> Option<U>) -> Option<U> {
    //     match self {
    //         Option::Some(value) => f(value),
    //         Option::None() => Option::None(),
    //     }
    // }
}
