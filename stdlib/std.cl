//! # The Conlang Standard Library

pub mod preamble {
    pub use super::{
        num::*,
        option::Option,
        range::{RangeExc, RangeInc},
        result::Result,
        str::str,
    };
}

pub mod num;

pub mod str;

pub mod option;

pub mod result;

pub mod range;

#[cfg("test")]
mod test;
