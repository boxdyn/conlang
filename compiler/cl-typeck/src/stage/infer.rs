//! Implements type unification, used by the Hindley-Milner type inference algorithm
//!
//! Inspired by [rust-hindley-milner][1] and [hindley-milner-python][2]
//!
//! [1]: https://github.com/tcr/rust-hindley-milner/
//! [2]: https://github.com/rob-smallshire/hindley-milner-python

pub mod engine;

pub mod inference;

pub mod error;
