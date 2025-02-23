//! # The Conlang Standard Library

pub mod preamble {
    pub use super::{num::*, str::str};
}

pub mod num;

pub mod str;

pub mod range;

#[cfg("test")]
mod test;
