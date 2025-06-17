//! Demonstrates the use of [read_and()]:
//!
//! The provided closure:
//! 1. Takes a line of input (a [String])
//! 2. Performs some calculation (using [FromStr])
//! 3. Returns a [Result] containing a [Response] or an [Err]

use repline::{error::Error as RlError, Repline, Response};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut rl = Repline::with_input(
        "fn main {\r\n\tprintln(\"Foo!\")\r\n}\r\n".as_bytes(),
        "\x1b[33m",
        "  >",
        " ?>",
    );
    while rl.read().is_ok() {}

    let mut rl = rl.swap_input(std::io::stdin());
    loop {
        let f = |_line| -> Result<_, RlError> { Ok(Response::Continue) };
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
            Err(e) => print!("\x1b[40G\x1b[A\x1bJ\x1b[91m{e}\x1b[0m\x1b[B"),
        }
    }
    Ok(())
}
