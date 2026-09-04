#!/bin/false
//! Conlang functions which are loaded into the default REPL
//!
//! Contains generally-useful functions which don't require
//! special functionality that the Conlang interpreter
//! doesn't yet have.

// Note: module inlining is NOT performed for this file.

enum Option<T> { Some(T), None }
enum Result<T, E> { Ok(T), Err(E) }
let Some, None, Ok, Err = {
    Option::Some, Option::None, Result::Ok, Result::Err
} // TODO: implement `use`

// TODO: accurate floating point number parsing

impl f64 {
    /// Alias for the `f64_to_bits` builtin
    let to_bits: (f64) -> u64 = __f64_to_bits;
    /// Alias for the `f64_from_bits` builtin
    let from_bits: (u64) -> f64 = __f64_from_bits;
    let parse: (str) -> f64 = __f64_parse;
    let sin: (f64) -> f64 = __f64_sin;
    let cos: (f64) -> f64 = __f64_cos;
    let tan: (f64) -> f64 = __f64_tan;

    /// The half-circle constant
    let PI = from_bits(0x400921fb54442d18);
    /// The golden ratio
    let PHI = from_bits(0x3ff9e3779b97f4a8);
    /// A really small number
    let SMOL = from_bits(0x0010000000000000);
    /// A really big number
    let LORG = from_bits(0x7fefffffffffffff);
    /// The most big number
    let INF = from_bits(0x7ff0000000000000);
    /// The most insidious number
    let NaN = from_bits(0x7ff8000000000000);
    /// Alias for the most insidious number
    let NAN = NaN;

    /// Gets the sign of `n` as an integer
    pub fn sign(n: f64) = f64::to_bits(n) >> 63;
    /// Gets the exponent of `n` as an integer
    pub fn exponent(n: f64) = (f64::to_bits(n) >> 52 & 0x7ff) - 1023;
    /// Gets the significand (mantissa) of `n` as an integer
    pub fn significand(n: f64) = f64::to_bits(n) & (1 << 52) - 1;
    /// Constructs a value from (sign, exponent, significand) form
    pub fn from_parts(sign: i64, exp: i64, significand: i64) = f64::from_bits(
        (sign & 1) << 63 | (exp + 1023 & 0x7ff) << 52 | (significand & (1 << 52) - 1)
    );
}

/// Returns the larger of `a` and `b`
pub fn max<T: Cmp>(a: T, b: T) -> T = if a < b b else a;

/// Returns the smaller of `a` and `b`
pub fn min<T: Cmp>(a: T, b: T) -> T = if a > b b else a;

/// Computes the square root of `n`
pub fn sqrt(mut n: f64) -> f64 {
    // NOTE: within a function body, we don't (currently) have access to `impl` members.
    if n < 0.0 return f64::NaN; // TODO: re-uppercase
    if n == 0.0 return 0.0;
    let z, err = n, f64::INF;
    while let adj = (z * z - n) / (2.0 * z) && f64::exponent(adj) < f64::exponent(err) {
        z -= adj;
        err = adj;
    }
    z
}

/// Returns the [char] representation of the `n`+1-ary digit `n` (up to base-36)
pub fn as_digit(n: u32) -> char = match n {
    ..10 => n + '0' as u32;
    _ => n - 10 + 'a' as u32;
} as char;

/// Formats `n` as a string in base `radix`
pub fn radix(n: i64, radix: i64) {
    fn recurse(n: i64, radix: i64) = if n == 0 "" else {
        recurse(n / radix, radix) + as_digit(n % radix)
    }

    match n {
        0 => "0";
        // TODO: breaks at i64::MIN
        ..0 => "-" + recurse(-n, radix);
        _ => recurse(n, radix);
    }
}

/// Counts the leading zeroes in `n`
pub fn count_leading_zeroes(n: u128) -> u128 {
    let mut xd = u128::BITS;
    if n < 0 return 0;
    while n != 0 {
        xd -= 1;
        n >>= 1;
    }
    xd
}

/// Formats `n` as a hexadecimal literal (0xf...)
pub fn hex(n: u128) {
    let out = "0x";
    for xd in min(count_leading_zeroes(n) / 4, 31)..32 {
        out += as_digit(n >> (31 - xd) * 4 & 0xf)
    }
    out
}

/// Formats `n` as an octal literal (0o7...)
pub fn oct(n: u128) {
    let out = "0o";
    for xd in min((count_leading_zeroes(n) + 1) / 3, 42)..43 {
        out += as_digit(n >> (42 - xd) * 3 & 7)
    }
    out
}

/// Formats `n` as a binary literal (0b1...)
pub fn bin(n: u128) {
    let out = "0b";
    for xd in min(count_leading_zeroes(n), 127)..128 {
        out += as_digit(n >> 127 - xd & 1)
    }
    out
}

/// Returns a shark
pub fn shark() = '\u{1f988}';
