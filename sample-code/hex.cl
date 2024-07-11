//! Formats numbers in hexadecimal, octal, or binary
mod math;

// TODO: casting and/or conversion
const HEX_LUT: Array = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
];

pub fn hex(n: u64) {
    let out = "0x";
    for xd in min(count_leading_zeroes(n) / 4, 15)..16 {
        out += HEX_LUT[(n >> (15 - xd) * 4) & 0xf]
    }
    out
}

pub fn oct(n: u64) {
    let out = "0o";
    for xd in min((count_leading_zeroes(n) + 2) / 3, 21)..22 {
        out += HEX_LUT[(n >> max(63 - (3 * xd), 0)) & 7]
    }
    out
}

pub fn bin(n: u64) {
    let out = "0b";
    for xd in min(count_leading_zeroes(n), 63)..64 {
        out += HEX_LUT[(n >> 63 - xd) & 1]
    }
    out
}
