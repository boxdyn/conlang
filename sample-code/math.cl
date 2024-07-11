//! Useful math functions

pub fn max(a: T, b: T) -> T {
    (if a < b { b } else { a })
}

pub fn min(a: T, b: T) -> T {
    (if a > b { b } else { a })
}

pub fn count_leading_zeroes(n: u64) -> u64 {
    let mut xd = 64;
    if n < 0 {
        return 0;
    }
    while n != 0 {
        xd -= 1;
        n >>= 1;
    }
    xd
}

pub fn count_trailing_zeroes(n: u64) -> u64 {
    let mut xd = 0;
    if n == 0 {
        64
    } else if n < 0 {
        0
    } else {
        while n & 1 == 0 {
            xd += 1;
            n >>= 1;
        }
        xd
    }
}
