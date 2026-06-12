
/// Outputs the `radix` form of `n` to the `stream`
pub fn radix_by_chars(stream: &fn(char), n: i128, radix: i64) {
    fn recurse(stream: &fn(char), n: i128, radix: i64) = if n > 0 {
        recurse(stream, n / radix, radix);
        stream(as_digit(n % radix))
    }

    if radix < 2 panic("Invalid base: ", radix);
    match n {
        0 => stream("0");
        ..0 => (stream("-"); recurse(stream, -n, radix));
        _ => recurse(stream, n, radix);
    }
}

/// Base formatting functionality: outputs a formatted char sequence,
/// one at a time, to the `stream`.
fn pub vformat(stream: &fn(char), format: str, args: (..)) {
    let buf = format.chars();
    fn take() = if (let [chr, ..rest] = buf) Some(buf = rest; chr) else None;
    fn arg() {
        let (arg, ..rest) = args else panic("Not enough args for ", format);
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
            specifier, args => panic("Can't format ", args, " with ", specifier);
        };
        Some(c) => stream(c);
        _ => break;
    }
}

fn pub format(stream: &fn(str), format: str, ..args) = vformat(stream, format, args);

fn pub printf(format: &str, ..args) = vformat(putchar, format, args);
fn pub printfn(format: &str, ..args) = (vformat(putchar, format, args); putchar('\n'));

fn pub sprintf(format: &str, ..args) {
    let out = "";
    vformat((|c| out += c), format, args);
    out
}

fn pub formatted_length(format: &str, ..args) {
    let out = 0;
    vformat((|_| out += 1), format, args);
    out
}
