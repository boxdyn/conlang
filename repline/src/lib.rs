//! A small pseudo-multiline editing library

mod editor;
mod iter;
mod raw;

pub mod error;
pub mod prebaked;
pub mod repline;

pub use error::Error;
pub use prebaked::{read_and, Response};
pub use repline::Repline;
