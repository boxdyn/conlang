//! Pseudo-random number generation using a LFSR algorithm

static state: u64 = 0xdeadbeefdeadbeef;

pub fn seed(seed: u64) {
    state = seed;
}

pub fn lfsr_next() {
    state ^= state >> 7;
    state ^= state << 9;
    state ^= state >> 13;
}

/// Returns a pseudorandom byte
pub fn rand() -> u8 {
    for _ in 0..8 {
        lfsr_next()
    }
    state & 0xff
}

// Prints a maze out of diagonal box drawing characters, ['╲', '╱']
fn mazel(width: u64, height: u64) {
    let walls = ['\u{2571}', '\u{2572}'];
    rand_rect(width, height, walls)
}

// Prints a rectangle with the provided walls
fn rand_rect(width: u64, height: u64, walls: [char; 2]) {
    for _ in 0..height {
        for _ in 0..width {
            print(walls[rand() % 2])
        }
        println()
    }
}
