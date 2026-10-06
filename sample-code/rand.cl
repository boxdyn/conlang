#!/usr/bin/env -S conlang Xorshift::new(get_time_micros()).maze(80, 24)
//! Pseudo-random number generation using a xorshift algorithm

struct Xorshift {
    state: u64,
};

impl Xorshift {
    /// Constructs a new `Xorshift` with the provided `seed`
    fn new(seed: u64) = Self { state: seed };

    /// Advances to the next state
    fn next(self: &Xorshift) {
        self.state ^= self.state >> 7;
        self.state ^= self.state << 9;
        self.state ^= self.state >> 13;
    }

    /// Returns a pseudorandom 32-bit integer
    fn get_u32(self: &Xorshift) -> u32 {
        self.next();
        self.state as u32
    }

    /// Returns a pseudorandom byte
    fn get_u8(self: &Xorshift) -> u8 {
        self.next();
        self.state as u8
    }
}

// Prints a maze out of diagonal box drawing characters, ['╲', '╱']
fn maze(rng: &Xorshift, width: u64, height: u64) {
    let walls = ['\u{2571}', '\u{2572}'];
    rand_rect(rng, width, height, walls)
}

// Prints a rectangle with the provided walls
fn rand_rect(rng: &Xorshift, width: u64, height: u64, walls: [char]) {
    for _ in 0..height {
        for _ in 0..width {
            print(walls[rng.get_u32() % walls.len()])
        }
        println()
    }
}
