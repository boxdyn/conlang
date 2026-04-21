#!/usr/bin/env -S conlang-repl -r false

fn debug(value) {
    println(value);
    value
}

fn main() {
    fn clear() = println("\x1b[H\x1b[3J\x1b[2JBrainfuck:");
    clear();

    let lines = "";
    loop match get_line(if lines.len() "  > " else " ,> ") {
        "\n" => (debug(compile(lines)).run(); lines = "");
        "clear\n" => (clear(); lines = "");
        line => lines += line;
    }
}

/// Brainfuck operators:
/// 
/// >  Move the pointer to the right
/// <  Move the pointer to the left
/// +  Increment the memory cell at the pointer
/// -  Decrement the memory cell at the pointer
/// .  Output the character signified by the cell at the pointer
/// ,  Input a character and store it in the cell at the pointer
/// [  Jump past the matching ] if the cell at the pointer is 0
/// ]  Jump back to the matching [ if the cell at the pointer is nonzero
enum Brainfuck {
    // > Move the pointer to the right (`n` cells)
    Right(usize),
    // < Move the pointer to the left (`n` cells)
    Left(usize),
    // + Increment the memory cell at the pointer (by `n`)
    Inc(usize),
    // - Decrement the memory cell at the pointer (by `n`)
    Dec(usize),
    // . Output the character signified by the cell at the pointer
    Out(usize),
    // , Input a character and store it in the cell at the pointer
    In,
    // [ Jump past the matching ] if the cell at the pointer is 0
    Jz(usize),
    // ] Jump back to the matching [ if the cell at the pointer is nonzero
    Jnz(usize),
}

fn compile(program: str) {
    let stack, fucks = [], [];
    for c in program fucks.join(match c {
        '>' => Brainfuck::Right(1);
        '<' => Brainfuck::Left(1);
        '+' => Brainfuck::Inc(1);
        '-' => Brainfuck::Dec(1);
        '.' => Brainfuck::Out(1);
        ',' => Brainfuck::In;
        '[' => {
            stack.push(fucks.len());
            Brainfuck::Jz(-1 as usize)
        };
        ']' => {
            let begin = stack.pop();
            fucks[begin] = Brainfuck::Jz(fucks.len());
            Brainfuck::Jnz(begin)
        };
        _ => continue;
    });
    if stack.len() panic("Stack ", stack, " not empty!");
    fucks
}

// may panic if list index out of range
fn last<T>(list: &[T]) = list[list.len() - 1];

// Joins an existing Brainfuck with a new Brainfuck, if it's possible
fn join(list: &[Brainfuck], op: Brainfuck) = if list.len() {
    list[list.len() - 1] = match (list[list.len() - 1], op) {
        Brainfuck::Right(n), Brainfuck::Right(m) => Brainfuck::Right(n + m);
        Brainfuck::Left(n), Brainfuck::Left(m) => Brainfuck::Left(n + m);
        Brainfuck::Inc(n), Brainfuck::Inc(m) => Brainfuck::Inc(n + m);
        Brainfuck::Dec(n), Brainfuck::Dec(m) => Brainfuck::Dec(n + m);
        Brainfuck::Out(n), Brainfuck::Out(m) => Brainfuck::Out(n + m);
        value, _ => (list.push(op); value);
    }
} else list.push(op);

/// Runs a compiled Brainfuck program
fn run(fucks: [Brainfuck]) {
    let tape = [0; 0x10000];
    let head = tape.len() / 2;
    let pc = -1;

    while (pc += 1; pc < fucks.len()) match fucks[pc] {
        Brainfuck::Right(n) => head += n;
        Brainfuck::Left(n) => head -= n;
        Brainfuck::Inc(n) => tape[head] = (tape[head] + n) as u8;
        Brainfuck::Dec(n) => tape[head] = (tape[head] - n) as u8;
        Brainfuck::Out(n) => for _ in 0..n print(tape[head] as char);
        Brainfuck::In => tape[head] = get_line(" > ")[0] as u8;
        Brainfuck::Jz(goto) => if tape[head] == 0 { pc = goto };
        Brainfuck::Jnz(goto) => if tape[head] != 0 { pc = goto };
    }
    println()
}
