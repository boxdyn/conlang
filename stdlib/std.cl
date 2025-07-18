//! # The Conlang Standard Library

pub mod preamble {
    pub use super::{
        io::*,
        num::*,
        option::Option,
        range::{RangeExc, RangeInc},
        result::Result,
        str::*,
    };
}

pub mod ffi;

pub mod io;

pub mod num;

pub mod str;

pub mod option;

pub mod result;

pub mod range;

// #[cfg("test")]
// mod test;
