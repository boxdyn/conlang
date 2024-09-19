//! Demonstrates the use of [read_and()]:
//! 
//! The provided closure:
//! 1. Takes a line of input (a [String])
//! 2. Performs some calculation (using [FromStr])
//! 3. Returns a [Result] containing a [Response] or an [Err]

use repline::{prebaked::read_and, Response};
use std::{error::Error, str::FromStr};

fn main() -> Result<(), Box<dyn Error>> {
    read_and("\x1b[33m", "  >", " ?>", |line| {
        println!("-> {:?}", f64::from_str(line.trim())?);
        Ok(Response::Accept)
    })?;
    Ok(())
}
