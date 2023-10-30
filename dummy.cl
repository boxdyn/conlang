// This is a Conlang file. 

// This is a function. It can be called with the call operator.
// The function called `main` is the program's entrypoint
fn main() {
    // An if expression is like the ternary conditional operator in C
    let y = if 10 < 50 {
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
    };
    // The same is true of `for` expressions!
    let w = for idx in 0..100 {
        if idx > 2 * 2 {
            break idx
        }
    } else {
        12345
    };
    // A block evaluates to its last expression,
    // or Empty if there is none
    (y, z, w) // (🦈, false, 5)
}
