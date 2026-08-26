#!/usr/bin/env -S conlang parfrepl() 

fn parfrepl() -> ! {
    loop print(" = ", match get_line(" +> ") {
        const "clear\n" => "\x1b[H\x1b[2J\x1b[3J";
        line => parf(line) as str + "\n";
    });
}

/// A really really terrible float parser
/// TODO: arbitrary precision arithmetic
fn parf(string: str) {
    let mut sign = 1.0;                   // Sign
    let mut (int, int_scale) = 0, 1.0;    // Integer component
    let mut (frac, frac_scale) = 0, 1.0;  // Fractional component
    let mut (power_sign, power) = 1.0, 0;  // Exponent

    enum State { Int, Dec, Exp };
    let state = State::Int;
    for c in string match state, c {
        State::Int, '-' => sign *= -1.0;
        State::Int, '0'..='9' if int < 1 << 120 => 
            int = int * 10 + c as i128 - 0x30;
        State::Int, '0'..='9' => int_scale *= 10.0;
        State::Int, '.' => state = State::Dec;
        State::Int, 'e' => state = State::Exp;

        State::Dec, '0'..='9' if int < 1 << 120 => {
            frac = frac * 10 + c as i128 - 0x30;
            frac_scale *= 10.0;
        }
        State::Dec, '0'..='9' => frac_scale *= 10.0;
        State::Dec, 'e' => state = State::Exp;

        State::Exp, '-' => power_sign = -1.0;
        State::Exp, '0'..='9' => power = power * 10 + c as i128 - 0x30;
        _ => {}
    }

    let scale = fold(0..power, 1.0, |v, _| v * 10.0);
    if power_sign < 0.0 scale = 1.0 / scale;
    sign * scale * (int as f64 * int_scale + frac as f64 * (1.0 / frac_scale))
}

fn fold<T, U>(&iter: _, init: T, f: fn(T, U) -> T) -> T {
    for _u in iter init = f(init, _u) else init
}
