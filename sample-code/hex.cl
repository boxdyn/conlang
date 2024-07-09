pub fn hex(mut n: u64) {
    let hex_lut = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
    ];
    for xd in 0..16 {
        let n = n >> (15 - xd) * 4;
        if n != 0 {
            print(hex_lut[n & 0xf])
        }
    }
    println()
}
