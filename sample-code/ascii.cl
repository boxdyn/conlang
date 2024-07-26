//! Prints out the characters in the ASCII printable range
//! in the format of a hex-dump

// TODO: Convenient stuff like this in the stdlib
const HEX_LUT: [char; 16] = [
    '0', '1', '2', '3', //
    '4', '5', '6', '7', //
    '8', '9', 'a', 'b', //
    'c', 'd', 'e', 'f', //
];

fn ascii() {
    for row in 0..6 {
        for col in 0..16 {
            if col == 8 {
                print(' ')
            }
            let i = 0x20 + row * 16 + col
            print(HEX_LUT[(i >> 4) & 0xf], HEX_LUT[i & 0xf], ' ')
        }
        print(" |")
        for col in 0..16 {
            let i = 0x20 + row * 16 + col
            print(i as char)
        }
        println("|")
    }
}
