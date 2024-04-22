//! # The Conlang Standard Library

pub mod preamble {
    pub use super::num::*;
}

pub mod num;

#[cfg("test")]
mod test;
