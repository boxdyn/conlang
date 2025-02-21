//! Here's a menu I prepared earlier!
//!
//! Constructs a [Repline] and repeatedly runs the provided closure on the input strings,
//! obeying the closure's [Response].

use std::error::Error;

use crate::{error::Error as RlError, repline::Repline};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// Control codes for the [prebaked menu](read_and)
pub enum Response {
    /// Accept the line, and save it to history
    Accept,
    /// Reject the line, and clear the buffer
    Deny,
    /// End the loop
    Break,
    /// Gather more input and try again
    Continue,
}

/// Implements a basic menu loop using an embedded [Repline].
///
/// Repeatedly runs the provided closure on the input strings,
/// obeying the closure's [Response].
///
/// Captures and displays all user [Error]s.
///
/// # Keybinds
/// - `Ctrl+C` exits the loop
/// - `Ctrl+D` clears the input, but *runs the closure* with the old input
pub fn read_and<F>(color: &str, begin: &str, again: &str, mut f: F) -> Result<(), RlError>
where F: FnMut(&str) -> Result<Response, Box<dyn Error>> {
    let mut rl = Repline::new(color, begin, again);
    loop {
        let line = match rl.read() {
            Err(RlError::CtrlC(_)) => break,
            Err(RlError::CtrlD(line)) => {
                rl.deny();
                line
            }
            Ok(line) => line,
            Err(e) => Err(e)?,
        };
        print!("\x1b[G\x1b[J");
        match f(&line) {
            Ok(Response::Accept) => rl.accept(),
            Ok(Response::Deny) => rl.deny(),
            Ok(Response::Break) => break,
            Ok(Response::Continue) => continue,
            Err(e) => rl.print_inline(format_args!("\x1b[40G\x1b[91m{e}\x1b[0m"))?,
        }
    }
    Ok(())
}
