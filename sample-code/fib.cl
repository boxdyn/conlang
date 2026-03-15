#!/usr/bin/env -S conlang -r false
// Calculate Fibonacci numbers

fn main() {
    for num in 0..=30 {
        println("fib(", num, ") = ", fibit(num))
    }
    for num in 0..=30 {
        println("fib(", num, ") = ", fib(num))
    }
}

/// The classic recursive definition of fib()
fn fib(a: i64) -> i64 {
    if a > 1 {
        fib(a - 1) + fib(a - 2)
    } else a
}

/// The classic iterative algorithm for fib()
fn fibit(n: i64) -> i64 {
    let a, b, c = 0, 1, 1;
    for _ in 0..n {
        a = b;
        b = c;
        c = a + b;
    } else a
}
