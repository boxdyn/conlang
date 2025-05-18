#!/usr/bin/env -S conlang-run 
//! A simple five-function pn calculator

// TODO: enum constructors in the interpreter
struct Atom(f64);
struct Op(char, [Expr]);

enum Expr {
    Atom(f64),
    Op(char, [Expr]),
}


// Evaluates an expression
fn eval(expr: Expr) -> isize {
    match expr {
        Atom(value) => value,
        Op('*', [lhs, rhs]) => eval(lhs) * eval(rhs),
        Op('/', [lhs, rhs]) => eval(lhs) / eval(rhs),
        Op('%', [lhs, rhs]) => eval(lhs) % eval(rhs),
        Op('+', [lhs, rhs]) => eval(lhs) + eval(rhs),
        Op('-', [lhs, rhs]) => eval(lhs) - eval(rhs),
        Op('-', [lhs]) => - eval(lhs),
        Op(other, ..rest) => {
            panic("ERROR: Unknown operator: " + other)
        },
        other => {
            println(other);
            panic("ERROR: Unknown operation ^")
        }
    }
}


/// Parses expressions
fn parse(line: [char], power: i32) -> (Expr, [char]) {
    fn map((expr, line): (Expr, [char]), f: fn(Expr) -> Expr) -> (Expr, [char]) {
        (f(expr), line)
    }

    line = space(line);

    let (lhs, line) = match line {
        ['0'..='9', ..] => number(line),
        [op, ..rest] => {
            parse(rest, pre_bp(op)).map(|lhs| Op(op, [lhs]))
        },
        _ => panic("Unexpected end of input"),
    };

    while let [op, ..rest] = space(line) {
        let (before, after) = inf_bp(op);
        if before < power {
            break;
        };
        (lhs, line) = parse(rest, after).map(|rhs| Op(op, [lhs, rhs]));
    };
    (lhs, line)
}

fn number(line: [char]) -> (Expr, [char]) {
    let value = 0.0;
    while (let [first, ..rest] = line) && (let '0'..='9' = first) {
        value = value * 10.0 + (first as f64 - '0' as f64);
        line = rest;
    };
    (Atom(value), line)
}

fn space(line: [char]) -> [char] {
    match line {
        [' ', ..rest] => space(rest),
        line => line
    }
}

fn inf_bp(op: char) -> (i32, i32) {
    (|x| (2 * x, 2 * x + 1))(
    match op {
        '*' => 2,
        '/' => 2,
        '%' => 2,
        '+' => 1,
        '-' => 1,
        _ => -1,
    })
}

fn pre_bp(op: char) -> i32 {
    (|x| 2 * x + 1)(
    match op {
        '-' => 9,
        _ => -1,
    })
}
