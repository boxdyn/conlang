#!/usr/bin/env conlang-run
//! Square root approximation, and example applications

/// A really small nonzero number
const EPSILON: f64 // = 8.854... * 10^-12
    = 88541878188 as f64
    / 10000000000 as f64
    / 1000000000000 as f64;

/// Calcuates the absolute value of a number
fn f64_abs(n: f64) -> f64 {
    let n = n as f64
    if n < (0 as f64) { -n } else { n }
}

/// Square root approximation using Newton's method
fn sqrt(n: f64) -> f64 {
    let n = n as f64
    if n < 0 as f64 {
        return 0 as f64 / 0 as f64 // TODO: NaN constant
    }
    if n == 0 as f64 {
        return 0 as f64
    }

    let z = n
    loop {
        let adj = (z * z - n) / (2 as f64 * z)
        z -= adj
        if adj.f64_abs() < EPSILON {
            break z;
        }
    }
}

/// Pythagorean theorem: a² + b² = c²
fn pythag(a: f64, b: f64) -> f64 {
    sqrt(a * a + b * b)
}

/// Quadratic formula: (-b ± √(b² - 4ac)) / 2a
fn quadratic(a: f64, b: f64, c: f64) -> (f64, f64) {
    let a = a as f64; let b = b as f64; let c = c as f64;
    (
        (-b + sqrt(b * b - 4 as f64 * a * c)) / 2 as f64 * a,
        (-b - sqrt(b * b - 4 as f64 * a * c)) / 2 as f64 * a,
    )
}

fn main() {
    for i in 0..10 {
        println("sqrt(",i,") ≅ ",sqrt(i))
    }
    println("\nPythagorean Theorem")
    println("Hypotenuse of ⊿(5, 12): ", pythag(5, 12))

    println("\nQuadratic formula")
    println("Roots of 10x² + 4x - 1: ", quadratic(10, 44, -1))
}
