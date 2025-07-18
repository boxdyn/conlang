//! TODO: give conland a string type
use super::num::{char, u8};

#[lang = "str"]
type str = [u8];

#[builtin = "chars"]
fn chars(s: &str) -> [char];

#[builtin = "fmt"]
fn fmt(..args) -> &str;
