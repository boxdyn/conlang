#!/usr/bin/env -S conlang -r false
// Showcases `get_line` behavior by sorting stdin

fn in_range(this: Ord, min: Ord, max: Ord) -> bool {
    min < this && this < max
}

fn frequency(s: str) -> [i32; 128] {
    let letters = [0; 128];
    for letter in s {
        if (letter).in_range(' ', letters.len() as char) {
            letters[(letter as i32)] += 1;
        }
    }
    letters
}

fn plot_freq(freq: [i32; 128]) -> str {
    let buf = "";
    for idx in 0..len(freq) {
        for n in 0..(freq[idx]) {
            buf += idx as char;
        }
    }
    buf
}

const let msg: str = "letter_frequency.cl
Computes the frequency of ascii characters in a block of text, and prints it bucket-sorted.
Press Ctrl+D to quit.";

fn main() {
    println(msg);
    let lines = "";
    loop {
        let line = get_line("letters > ");
        if line == "" {
            break ();
        }
        lines += line;
    }

    let freq = frequency(lines);
    let plot = plot_freq(freq);
    println(plot)
}
