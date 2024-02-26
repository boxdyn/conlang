// Calculate Fibonacci numbers

fn main() -> i128 {
    let num = 10;
    print("fib(", num, "): ", fib(num));
}

/// Implements the classic recursive definition of fib()
fn fib(a: i128) -> i128 {
    if a > 1 {
        fib(a - 1) + fib(a - 2)
    } else {
        1
    }
}
