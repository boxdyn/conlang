// Calculate Fibonacci numbers

fn main() {
    for num in 0..=30 {
        println!("fib({num}) = {}", fib(num))
    }
}

/// Implements the classic recursive definition of fib()
fn fib(a: i64) -> i64 {
    if a > 1 { fib(a - 1) + fib(a - 2) } else { a }
}
