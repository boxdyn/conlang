//! Formats numbers in hexadecimal, octal, or binary
mod math;
// FIXME: modules
// use math::{min, count_leading_zeroes};

fn as_digit(n: u32) -> char = match n {
    ..10 => n + '0' as u32;
    _ => n - 10 + 'a' as u32;
} as char;

pub fn radix(n: i64, radix: i64) {
    fn recurse(n: i64, radix: i64) = if n == 0  "" else {
        recurse(n / radix, radix) + as_digit(n % radix)
    }

    match n {
        0 => "0";
        // TODO: breaks at i64::MIN
        ..0 => "-" + recurse(-n, radix);
        _ => recurse(n, radix);
    }
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
