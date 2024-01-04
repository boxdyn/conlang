// Calculate Fibonacci numbers

fn main() -> i128 {
    print("fib(10): ", fib(10));
}

/// Implements the classic recursive definition of fib()
fn fib(a: i128) -> i128 {
    if a > 1 {
        fib(a - 1) + fib(a - 2)
    } else {
        1
    }
}
