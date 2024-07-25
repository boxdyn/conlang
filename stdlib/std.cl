//! # The Conlang Standard Library

pub mod preamble {
    pub use super::{num::*, str::str};
}

pub mod num;

pub mod str;

#[cfg("test")]
mod test;
