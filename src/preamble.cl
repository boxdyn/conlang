#!/bin/false
// Note: module inlining is NOT performed for this file.

pub mod builtin_preamble {
    //! Conlang functions which are loaded into the default REPL
    //!
    //! Contains generally-useful functions which don't require
    //! special functionality that the Conlang interpreter
    //! doesn't yet have.

    enum Option<T> { Some(T), None }
    enum Result<T, E> { Ok(T), Err(E) }
    let Some, None, Ok, Err = {
        Option::Some, Option::None, Result::Ok, Result::Err
    } // TODO: implement `use`

    // TODO: accurate floating point number parsing
    const (
        /// The half-circle constant
        let PI = f64_from_bits(0x400921fb54442d18);
        /// The golden ratio
        let PHI = f64_from_bits(0x3ff9e3779b97f4a8);
        /// A really small number
        let SMOL = f64_from_bits(0x0010000000000000);
        /// A really big number
        let LORG = f64_from_bits(0x7fefffffffffffff);
        /// The most big number
        let INF = f64_from_bits(0x7ff0000000000000);
    );

    pub fn f64_sign(n: f64) = f64_to_bits(n) >> 63;
    pub fn exponent(n: f64) = (f64_to_bits(n) >> 52 & 0x7ff) - 1023;
    pub fn significand(n: f64) = f64_to_bits(n) & (1 << 52) - 1;
    pub fn f64_from(sign: i64, exp: i64, significand: i64) = f64_from_bits(
        (sign & 1) << 63 | (exp + 1023 & 0x7ff) << 52 | (significand & (1 << 52) - 1)
    );

    pub fn max<T: Cmp>(a: T, b: T) -> T = if a < b b else a;
    pub fn min<T: Cmp>(a: T, b: T) -> T = if a > b b else a;
    pub fn sqrt(mut n: f64) -> f64 {
        if n < 0.0 return f64::NaN; // TODO: re-uppercase
        if n == 0.0 return 0.0;
        let z, err = n, f64::INF;
        while let adj = (z * z - n) / (2.0 * z) && adj < err {
            z -= adj;
            err = adj;
        }
        z
    }

    pub fn as_digit(n: u32) -> char = match n {
        ..10 => n + '0' as u32;
        _ => n - 10 + 'a' as u32;
    } as char;
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
    pub fn count_leading_zeroes(n: u128) -> u128 {
        let mut xd = u128::BITS;
        if n < 0 return 0;
        while n != 0 {
            xd -= 1;
            n >>= 1;
        }
        xd
    }
    pub fn hex(n: u128) {
        let out = "0x";
        for xd in min(count_leading_zeroes(n) / 4, 31)..32 {
            out += as_digit(n >> (31 - xd) * 4 & 0xf)
        }
        out
    }
    pub fn oct(n: u128) {
        let out = "0o";
        for xd in min((count_leading_zeroes(n) + 1) / 3, 42)..43 {
            out += as_digit(n >> (42 - xd) * 3 & 7)
        }
        out
    }
    pub fn bin(n: u128) {
        let out = "0b";
        for xd in min(count_leading_zeroes(n), 127)..128 {
            out += as_digit(n >> 127 - xd & 1)
        }
        out
    }
    pub fn shark() = '\u{1f988}';
}
