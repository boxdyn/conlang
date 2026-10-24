#!/usr/bin/env -S conlang main()


fn main() {
    let shark_party = {[shark(); 1000].into_iter().fold("", add)};
    
    let _warmup = runtime_of(100, || for _ in shark_party.bytes());

    let eager = runtime_of(1000, || for _ in shark_party.to_char_array());
    println("for _ in shark_party.to_char_array() :> ", eager, " seconds per iteration");

    let bytes = runtime_of(1000, || for _ in shark_party.bytes());
    println("for _ in shark_party.bytes()         :> ", bytes, " seconds per iteration");

    let chars = runtime_of(1000, || for _ in shark_party.chars());
    println("for _ in shark_party.chars()         :> ", chars, " seconds per iteration");
}