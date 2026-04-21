#!/usr/bin/env -S conlang-repl -r false
//! Implements the [Deadfish][df] language
//! ```
//! iissiiisdsdddddddddddddddddddddddddddddd
//! dddddddddddddddddddddddddddddddddddddddd
//! dddddddddddddddddddddddddddddddddddddddd
//! dddddddddddddddddddddddddddddddddddddddd
//! dddddddddddddddddddddddddddddddddd u ;
//! ```
//!
//! [df]: https://esolangs.org/wiki/Deadfish

fn main() {
    println("-- Deadfish interpreter in Conlang");

    let value = 0;
    loop {
        let printed = false;
        for c in get_line(">> ") {
            value = match c {
                'i' => value + 1;
                'd' => value - 1;
                's' => value * value;
                'o' => (printed = true; print(value); continue);
                'u' => (printed = true; print(value as char); continue);
                ';' => 0;
                'h' => return println();
                _ => continue;
            }
            /* Make sure x is not greater then [sic] 256 */
            value = if (value == -1 || value == 256) 0 else value
        } else if printed println()
    }
}
