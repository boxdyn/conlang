# Conlang: Expression-Oriented Programming Language
This project began out of a desire to merge functional-style control flow expressions 
with Python's fun for-else/while-else syntax. I fully intend to devote my spare time
to conlang for the forseeable future, and I livestream development on Twitch for one 
Friday each month.

## Immediate Goals:
- [x] Decide on a minimal set of keywords and operators to support
- [x] Lex an entire Rust source file (minus generics, paths, and lifetimes)
- [x] Write expression grammar
- [x] Write AST for expression grammar
- [x] Write parser for AST
- [ ] Create tests for parser (and AST)
- [ ] Parse `dummy.cl` into a valid AST
- [x] Pretty printer, for debugging
- [ ] Create minimal statement grammar
  - [ ] Variable definition statements
  - [ ] Function definition statements

## Short Goals:
- [ ] `for` loops and `while` loops can be used on the right-hand side of an assignment
- [ ] Data structures and sum-type enums
- [ ] Expression type-checker
- [ ] Trait/Interface system
- [ ] Tree-walk interpreter for prototyping and debugging
- [ ] Three-reference bytecode VM for standard library development

## Long Goals:
- [ ] Semicolons are NEVER given special treatment
- [ ] Compile to LLVM IR
- [ ] Create a standard library for the language, with Rust-like abstractions.
