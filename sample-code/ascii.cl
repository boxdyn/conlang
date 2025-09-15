#!/usr/bin/env -S conlang -r false
//! Prints out the characters in the ASCII printable range
//! and the Latin-1 supplement in the format of a hex-dump

fn main () {
    ascii()
}

fn n_digit(n: u32) -> char {
    (if n > 9 {
        ('a' as u32) + n - 10
    } else {
        ('0' as u32) + n
    }) as char
}

fn in_range(&num: &u32, start: u32, end: u32) -> bool {
    (start <= num) && (num <= end )
}

fn ascii() {
    for row in 0..16 {
        for col in 0..16 {
            if col == 8 {
                print(' ')
            }
            print(n_digit(row), n_digit(col), ' ')
        }
        print(" │")
        for col in 0..16 {
            print(ascii_picture(row << 4 | col))
        }
        println("│")
    }
}

// Start of the C0 control pictures region
const CO_CONTROL_PICTURES: u32 = '\u{2400}' as u32;

fn ascii_picture(c: u32) -> char {
    if c < ' ' as u32 {                 // C0
        (CO_CONTROL_PICTURES + c) as char
    } else if c == 127 {                // C0:DEL
        '␡' // SYMBOL_FOR_DELETE
    } else if c.in_range(0x7f, 0xa0) {  // C1
        ' '
    } else c as char
}
