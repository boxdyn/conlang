//! The IO module contains functions for interacting with
//! input and output streams
use super::str::str;

/// Immediately causes program execution to stop
#[builtin = "panic"]
fn panic(..args) -> !;

/// Prints whatever you give it
#[builtin = "print"]
fn print(..args);

/// Prints whatever you give it, followed by a newline
#[builtin = "println"]
fn println(..args);

/// Debug-prints a thing, returning it
#[builtin = "dbg"]
fn dbg(arg) -> _ arg
