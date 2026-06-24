#!/usr/bin/env -S conlang ''

// should return (96, 95)
fn label_test() 'out
    for x in 1..=100
        for y in 1..=x
            if x + y > 190
                break 'out x, y;

fn assert_eq<T: Cmp>(expected: T, got: T, message: _)
    if expected != got panic(message,":\n", expected, " != ", got, "!")
    else println(message, "\nSuccess! (", expected, ")");

assert_eq((96, 95), label_test(), label_test)
