#!/usr/bin/env -S conlang main()
//! Implements a Truth Machine

fn main (n)
    match n {
        1 => loop print(1);
        n => println(n);
    }
