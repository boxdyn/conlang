#!/usr/bin/env conlang-run
//! Square root approximation, and example applications

/// A really small nonzero number
const EPSILON: f64 = 8.8541878188 / 1000000000000.0;

/// Calcuates the absolute value of a number
fn f64_abs(n: f64) -> f64 {
    let n = n as f64
    if n < (0.0) { -n } else { n }
}

/// Square root approximation using Newton's method
fn sqrt(n: f64) -> f64 {
    let n = n as f64
    if n < 0.0 {
        return 0.0 / 0.0 // TODO: NaN constant
    }
    if n == 0.0 {
        return 0.0
    }

    let z = n
    loop {
        let adj = (z * z - n) / (2.0 * z)
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
        (-b + sqrt(b * b - 4.0 * a * c)) / 2.0 * a,
        (-b - sqrt(b * b - 4.0 * a * c)) / 2.0 * a,
    )
}

fn main() {
    for i in 0..10 {
        println("sqrt(",i,") ≅ ",sqrt(i as f64))
    }
    println("\nPythagorean Theorem")
    println("Hypotenuse of ⊿(5, 12): ", pythag(5.0, 12.0))

    println("\nQuadratic formula")
    println("Roots of 10x² + 4x - 1: ", quadratic(10.0, 44.0, -1.0))
}
