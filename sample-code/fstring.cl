
fn f(__fmt: str) -> str {
    let __out = "";
    let __expr = "";
    let __depth = 0;
    for __c in chars(__fmt) {
        match __c {
            '{' => {
                __depth += 1
                if __depth <= 1 {
                    continue
                }
            },
            '}' => {
                __depth -= 1
                if __depth <= 0 {
                    __out += fmt(eval(__expr))
                    __expr = ""
                    continue
                }
            },
            ':' => if __depth == 1 {
                __out += __expr + ": "
            },
            _ => {}
        }
        if __depth > 0 {
            __expr += __c
        } else __out += __c;
    }
    __out
}
