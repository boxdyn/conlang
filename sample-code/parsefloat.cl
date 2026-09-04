#!/usr/bin/env -S conlang repl(|v| parf(v))

fn repl(f: (str) -> str) {
    loop print("out: ", match get_line(" in: ") {
        "clear\n" => "\x1b[H\x1b[2J\x1b[3J";
        "exit\n" => break;
        line => f(line) as str + "\n";
    });
}

/// A really really terrible float parser
/// TODO: arbitrary precision arithmetic
fn parf(string: str) -> f64 {
    let sign = 1.0;                   // Sign
    let (int, int_scale) = 0, 0;      // Integer component
    let (frac, frac_scale) = 0, 0;    // Fractional component
    let (power_sign, power) = 1.0, 0; // Exponent

    enum State { Int, Dec, Exp };
    let state = State::Int;
    for c in string match state, c {
        // Float ::= Int? ('.' Dec)? ('e' Exp)?
        // Int ::= '-'? ('0'..='9')*
        State::Int, '-' => sign *= -1.0;
        State::Int, '0'..='9' if int < 1 << 120 => 
            int = int * 10 + c as i128 - 0x30;
        State::Int, '0'..='9' => int_scale += 1;
        State::Int, '.' => state = State::Dec;
        State::Int, 'e' => state = State::Exp;

        // Dec ::= ('0'..='9')*
        State::Dec, '0'..='9' if frac < 1 << 120 => {
            frac = frac * 10 + c as i128 - 0x30;
            frac_scale += 1;
        }
        State::Dec, '0'..='9' => {};
        State::Dec, 'e' => state = State::Exp;

        // Exp ::= '-'? ('0'..='9')* 
        State::Exp, '-' => power_sign = -1.0;
        State::Exp, '0'..='9' => power = power * 10 + c as i128 - 0x30;
        _ => {}
    }

    let scale = 10.0.powf(power as f64 * power_sign);
    // assemble the float
    sign * scale * (
        int as f64 * 10.0.powf(int_scale as f64)
        + frac as f64 / 10.0.powf(frac_scale as f64)
    )
}

fn fold<T, U>(&iter: _, init: T, f: fn(T, U) -> T) -> T {
    for _u in iter init = f(init, _u) else init
}
