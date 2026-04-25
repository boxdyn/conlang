#!/usr/bin/env -S conlang -r false
//! A simple five-function pn calculator

enum Expr {
    Atom(f64),
    Op(char, [Expr]),
}


/// executes an expression
fn execute(&expr: Expr) -> f64 {
    match expr {
        Expr::Atom(value) => value;
        Expr::Op('*', [lhs, rhs]) => execute(lhs) * execute(rhs);
        Expr::Op('/', [lhs, rhs]) => execute(lhs) / execute(rhs);
        Expr::Op('%', [lhs, rhs]) => execute(lhs) % execute(rhs);
        Expr::Op('+', [lhs, rhs]) => execute(lhs) + execute(rhs);
        Expr::Op('-', [lhs, rhs]) => execute(lhs) - execute(rhs);
        Expr::Op('>', [lhs, rhs]) => (execute(lhs) as u64 >> execute(rhs) as u64) as f64;
        Expr::Op('<', [lhs, rhs]) => (execute(lhs) as u64 << execute(rhs) as u64) as f64;
        Expr::Op('-', [lhs]) => - execute(lhs);
        other => {
            panic("Unknown operation: " + fmt(other))
        }
    }
}


/// Pretty-prints an expression
fn fmt_expr(expr: Expr) -> str {
    match expr {
        Expr::Atom(value) => fmt(value);
        Expr::Op(operator, [lhs, rhs]) => fmt('(', fmt_expr(lhs), ' ', operator, ' ', fmt_expr(rhs), ')');
        Expr::Op(operator, [rhs]) => fmt(operator, fmt_expr(rhs));
        _ => println("Unexpected expr: ", expr);
    }
}
fn print_expr(expr: Expr) {
    println(fmt_expr(expr))
}


/// Parses expressions
fn parse(&line: &[char], power: i32) -> (Expr, [char]) {
    fn map(&(expr, line): (Expr, [char]), f: fn(Expr) -> Expr) -> (Expr, [char]) {
        (f(expr), line)
    }

    line = space(line);

    let lhs, line = match line {
        ['0'..='9', ..] => number(line);
        ['(', ..rest] => match parse(rest, Power::None) {
            expr, [')', ..rest] => expr, rest;
            expr, rest => panic(fmt("Expected ')', got ", expr, ", ", rest));
        };
        [op, ..rest] => parse(rest, pre_bp(op)).map(|lhs| Expr::Op(op, [lhs]));
        other => panic("Unexpected end of input: ", other);
    };

    while let [op, ..rest] = space(line) {
        let (before, after) = inf_bp(op);
        if before < power {
            break;
        };
        let v = parse(rest, after).map(|rhs| Expr::Op(op, [lhs, rhs]));
        lhs = v.0;
        line = v.1;
    };
    lhs, line
}

fn number(&line: [char]) -> (Expr, [char]) {
    let value = 0.0;
    while (let [first, ..rest] = line) && (let '0'..='9' = first) {
        value = value * 10.0 + (first as f64 - '0' as f64);
        line = rest;
        // (value, line) = (value * 10.0 + (first as f64 - '0' as f64), rest)
    } else (Expr::Atom(value), line)
}

fn space(line: [char]) -> [char] {
    loop match line {
        [' ', ..rest] => line = rest;
        ['\n', ..rest] => line = rest;
        [..] => break line;
    }
}

enum Power {
    None,
    Shift,
    Factor,
    Term,
    Exponent,
    Unary,
}

fn inf_bp(op: char) -> (i32, i32) {
    (|x| (2 * x, 2 * x + 1))(
    match op {
        '*' => Power::Term;
        '/' => Power::Term;
        '%' => Power::Term;
        '+' => Power::Factor;
        '-' => Power::Factor;
        '>' => Power::Shift;
        '<' => Power::Shift;
        _ => Power::None;
    } as i32)
}

fn pre_bp(op: char) -> i32 {
    (|x| 2 * x + 1)(
    match op {
        '-' => Power::Unary;
        _ => panic("Unknown unary operator: " + op);
    } as i32)
}

fn main() {
    loop {
        let line = get_line("calc > ");
        let (expr, rest) = line.chars().parse(0);

        println(fmt_expr(expr), " -> ", execute(expr));
    }
}
