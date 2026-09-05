#!/bin/false
//! Silly non-type-safe implementation of printf
//! This'll have to get removed later, but it's a bit of fun :P


/// Outputs the `radix` form of `n` to the `stream`
fn radix_by_chars(stream: &fn(char), n: i128, radix: i64) {
    fn recurse(stream: &fn(char), n: i128, radix: i64) = if n > 0 {
        recurse(stream, n / radix, radix);
        stream(as_digit(n % radix))
    }

    if radix < 2 panic("Invalid base: ", radix);
    match n {
        0 => stream('0');
        ..0 => (stream('-'); recurse(stream, -n, radix));
        _ => recurse(stream, n, radix);
    }
}

fn format_bool(stream: &fn(char), arg: bool) = {
    for c in if arg "true" else "false" stream(c);
}

/// Base formatting functionality: outputs a formatted char sequence,
/// one at a time, to the `stream`.
fn vformat(stream: &fn(char), format: str, args: (..)) {
    let buf = format.chars();
    fn take() = if let [chr, ..rest] = buf Some(buf = rest; chr) else None;
    fn arg() {
        let arg, ..rest = args else panic("Not enough args for ", format);
        args = rest;
        arg
    }
    loop match take() {
        Some('%') => match take() {
            Some('%') => stream('%');
            // TODO: float formatting
            Some('s') => for c in (arg() as str) stream(c);
            Some('c') => stream(arg() as char);
            Some('i') => radix_by_chars(stream, arg() as i64, 10);
            Some('u') => radix_by_chars(stream, arg() as u64, 10);
            Some('b') => radix_by_chars(stream, arg() as u128, 2);
            Some('o') => radix_by_chars(stream, arg() as u128, 8);
            Some('d') => radix_by_chars(stream, arg() as u128, 10);
            Some('x') => radix_by_chars(stream, arg() as u128, 16);
            Some('~') => radix_by_chars(stream, arg() as u128, 36);
            Some('B') => format_bool(stream, arg() as bool);
            Some('D') => arg().display(stream);
            specifier => panic("Can't format ", args, " with ", specifier);
        };
        Some(c) => stream(c);
        _ => break;
    }
}

/// vformat, but it takes its args variadically.
fn format(stream: &fn(str), format: str, ..args) = vformat(stream, format, args);

/// Prints the formatted output to stdout
fn printf(format: &str, ..args) = vformat(putchar, format, args);

/// Prints the formatted output to stdout, followed by a newline
fn printfn(format: &str, ..args) = (vformat(putchar, format, args); putchar('\n'));

/// Prints the formatted output into a string
fn sprintf(format: &str, ..args) {
    let out = "";
    vformat((|c| out += c), format, args);
    out
}

/// Gets the length (in codepoints) of the formatted string
fn formatted_length(format: &str, ..args) {
    let out = 0;
    vformat((|_| out += 1), format, args);
    out
}
