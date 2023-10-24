#!/usr/local/bin/conlang
// This is a Conlang file. Conlang is an expression-based language designed for maximum flexibility etc. etc. whatever

// This is a function. It can be called with the call operator.
// The function called `main` is the program's entrypoint
fn main() {
    let x = 100;

    // An if expression is like the ternary conditional operator in C
    let y = if x < 50 {
        "\u{1f988}"
    } else {
        "x"
    };
    
    // A `while` expression is like the while-else construct in Python,
    // but it returns a value via the `break` keyword
    let z = while false {
        // do a thing repeatedly
        break true
    // If `while` does not `break`, fall through to the `else` expression
    } else {
        false
    }
    // The same is true of `for` expressions!
    let w = for idx in 0..100 {
        break idx
    } else {
        12345
    }
    // TODO: decide how to do IO
}
