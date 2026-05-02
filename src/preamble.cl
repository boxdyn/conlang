#!/bin/false
// Note: module inlining is NOT performed for this file.

pub mod builtin_preamble {
    //! Conlang functions which are loaded into the default REPL
    //! 
    //! Contains generally-useful functions which don't require
    //! special functionality that the Conlang interpreter
    //! doesn't yet have.

    pub fn max<T: Cmp>(a: T, b: T) -> T = if a < b b else a;
    pub fn min<T: Cmp>(a: T, b: T) -> T = if a > b b else a;
    pub fn sqrt(mut n: f64) -> f64 {
        const let EPSILON: f64 = 8.8541878188 / 1000000000000.0;
        if n < 0.0 return f64::NaN; // TODO: re-uppercase
        if n == 0.0 return 0.0;
        let z = n;
        loop {
            let adj = (z * z - n) / (2.0 * z);
            z -= adj;
            if (if adj >= 0.0 adj else -adj) < EPSILON break z;
        }
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
    pub fn count_leading_zeroes(n: u64) -> u64 {
        let mut xd = u64::BITS;
        if n < 0 return 0;
        while n != 0 {
            xd -= 1;
            n >>= 1;
        }
        xd
    }
    pub fn hex(n: u64) {
        let out = "0x";
        for xd in min(count_leading_zeroes(n) / 4, 15)..16 {
            out += as_digit((n >> (15 - xd) * 4) & 0xf)
        }
        out
    }
    pub fn oct(n: u64) {
        let out = "0o";
        for xd in min((count_leading_zeroes(n) + 2) / 3, 21)..22 {
            out += as_digit((n >> max(63 - (3 * xd), 0)) & 7)
        }
        out
    }
    pub fn bin(n: u64) {
        let out = "0b";
        for xd in min(count_leading_zeroes(n), 63)..64 {
            out += as_digit((n >> 63 - xd) & 1)
        }
        out
    }
    pub fn shark() = '\u{1f988}';
}
