//! Implements format string evaluation in weak Conlang

/// Formats a string
#[rustfmt::skip]
fn f(__fmt: &str) -> &str {
    let __out, __expr, __label, __depth = "", "", "", 0;
    for __c in chars(__fmt) {
        match __c {
            '{' => {
                __depth += 1;
                if __depth <= 1 {
                    continue
                }
            }
            '}' => {
                __depth -= 1;
                if __depth <= 0 {
                    __out = fmt(__out, __label, eval(__expr));
                    __expr = "";
                    __label = "";
                    // TODO: pattern-style destructuring
                    // (__expr, __label) = ("", "");
                    continue
                }
            }
            ':' => if __depth == 1 && __label.len() == 0 {
                __label = __expr + __c;
                continue
            }
            '=' => if __depth == 1 && __label.len() == 0 {
                __label = __expr + __c;
                continue
            }
            _ => {}
        }
        match (__depth, __label.len()) {
            0, _ => __out += __c;
            _, 0 => __expr += __c;
            _, _ => __label += __c;
        }
    }
    __out
}
