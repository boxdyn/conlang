#!/usr/local/bin/conlang
// This is a Conlang file. Conlang is an expression-based language designed for maximum flexibility etc. etc. whatever

// This is a function. It can be called with the call operator.
// The function called `main` is the program's entrypoint
fn main() {
    let x = 100;

    // An if expression is like the ternary conditional operator in C
    let y = if x < 50 {
        0
    } else {
        x
    };
    
    // A `for` expression is like the for-else construct in Python, but it returns a value via the `break` keyword
    let z = for i in 0..y {
        // do a thing repeatedly
        break true
    } else {
        false
    };
    // TODO: decide how to do IO
}
