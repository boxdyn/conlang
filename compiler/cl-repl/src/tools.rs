use cl_token::Token;
/// Prints a token in the particular way [cl-repl](crate) does
pub fn print_token(t: &Token) {
    println!(
        "{:02}:{:02}: {:#19?} │{}│",
        t.span.path, t.span.head, t.kind, t.lexeme,
    )
}
