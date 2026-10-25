#!/usr/bin/env -S conlang main()


fn main() {
    let shark_party = {[shark(); 1000].into_iter().fold("", add)};
    let a_party = {['a'; 4000].into_iter().fold("", add)};
    let warmup = runtime_of(500, || for _ in shark_party.bytes());
    println("Warmup complete: ", warmup * 500.0, " seconds");

    let eager = runtime_of(1000, || for _ in shark_party.to_char_array());
    println("for _ in shark_party.to_char_array() :> ", eager, " seconds per iteration");

    let bytes = runtime_of(1000, || for _ in shark_party.bytes());
    println("for _ in shark_party.bytes()         :> ", bytes, " seconds per iteration");

    let chars = runtime_of(1000, || for _ in shark_party.chars());
    println("for _ in shark_party.chars()         :> ", chars, " seconds per iteration");

    let eager = runtime_of(1000, || for _ in a_party.to_char_array());
    println("for _ in a_party.to_char_array()     :> ", eager, " seconds per iteration");

    let bytes = runtime_of(1000, || for _ in a_party.bytes());
    println("for _ in a_party.bytes()             :> ", bytes, " seconds per iteration");

    let chars = runtime_of(1000, || for _ in a_party.chars());
    println("for _ in a_party.chars()             :> ", chars, " seconds per iteration");
}