//! This is the Conlang syntax tutorial example
# // Tutorial helper functions
# fn do_not() = 0;
# fn condition() = false;
# fn is_special<T>(_value: T) = false;
# fn default_value() = 10;

/// `let` expressions try to bind variables
let x = 1, 2, 3;
/// ...and return `true` if they succeeded!
if (let 1, two, 3 = x) {
    println(two) // prints "2"
}


/// `fn` expressions bind functions to names
fn do_thing(x: i32, y: i32) -> i32 {
    x + y
}
fn do_thing_1(x: i32, y: i32) -> i32 = x + y; // Alt syntax

/// ...and return a function object!
let do_thing_2 = fn (x: i32, y: i32) = x + y;
let do_thing_3 = |x: i32, y: i32| x + y; // Alt syntax

/// You can invoke that function by `call`ing it:
do_thing(10, 20);
do_thing_1(10, 20);
do_thing_2(10, 20);
do_thing_3(10, 20);


/// `return` expressions end the containing function,
fn first_even(numbers: &[i32]) -> i32 {
    for index in 0..numbers.len() {
        // and can be used *anywhere an expression is valid!*
        if numbers[index] % 2 == 0 return index
    }
    -1
}


/// `type`, `struct`, and `enum` expressions bind types to names
enum Option<T> {
    Some(T),
    None,
}

/// ...and return a type-info object!
//     Note: this *also* introduces the name `Foo` in scope.
let ti = struct Foo { x: i32, y: i32 };
ti::x == i32 /* true */;


/// `if` expressions branch execution based on a value:
if condition() {
    do_thing(10, 20)
} else {
    do_not()
}

/// ... and return the result of whichever branch executed!
let v = if condition() {
    do_thing(10, 20)
} else {
    do_not()
}

/// The `pass` and `fail` expressions don't have to be blocks:
let v = if condition() do_thing(10, 20) else do_not();
// but if the `pass` expression starts with something that can be
// an infix operator (i.e. `+`, `-`, `(...)`, ...), it'll
// become part of the condition instead.
//
// Let your heart guide you.


/// `loop` expressions run the same calculation multiple times:
// loop print(1); // prints infinite `1`s

/// ...and return the value passed to `break`!
let v = loop break 10;


/// `while` expressions loop while a condition is true:
while condition() print(1);

/// ...and return *either* the value passed to `break`,
///    if there is one, or the result of executing their
///    `else` branch!
let v = while condition() {
    /// do calculation... Maybe `break value`...
    let next_value = 0;
    if is_special(next_value) break next_value
} else {
    default_value() // executed if the body didn't `break`
}

/// `for` expressions loop over an iterator, such as a range
for value in 1..=100 {
    println(value)
}

/// ...and have the same `break` semantics as `while` loops!
let v = for value in 1..=100 {
    if is_special(value) break value
} else default_value();


/// "Do" expressions discard the result of evaluating the
/// previous expression, and return the result of the
/// following expression:
let v = (
    print("I did this!"); // "I did this!"
    "and then returned this!"
);
print(v); // "and then returned this!"

// You'll notice that this entire tutorial section
// has been separated by `do` operators (semicolons.)


/// "Block" expressions introduce a local scope, in which
/// new names can be bound and later discarded.
/// They are otherwise equivalent to parentheses.
{
    let explanation = "This is a block expression!";
    print(explanation);
    // If a closing brace follows a `do` operator,
    // the block evaluates to the unit type/value.
}

/// After a block-expression, the parser *may* insert a
/// semicolon if it makes sense to do so.
print("So, even though there was no ';', this still works!");
