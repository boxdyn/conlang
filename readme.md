> "A constructed language is a language for communication between humans (i.e. not with or between computers)"  
> [Wikipedia, *Constructed Language*](https://en.wikipedia.org/wiki/Constructed_language)


# Conlang: Expression-Oriented Programming Language
The primary function of a high-level programming language is to communicate among programmers.
Translation into an executable program is secondary to understanding the code.

In this regard, Conlang is an esoteric art-lang whose syntax and semantics mirror several common
programming languages (to the point of near source-compatibility with a select handful of programs.)

The phonology and orthography of Conlang are identical to that of The Rust Programming Language,
which, while being my favorite production-grade programming language of all time, falls short
in its ability to expressively express expressions.

(The language's name is undeniably a misnomer.)


## Code examples

The file [dummy.cl](/dummy.cl) contains the earliest, and most approachable, look at Conlang syntax.
It still runs in the interpreter, and there are no plans to break compatibility with that file
in particular. Of course, it does not cover all language features; those language features unused
in the file are subject to change.


A minor goal of the project is to keep the syntax *roughly* compatible with Rust, so regex-based
code highlighters will highlight the code in an acceptable manner. As such, Conlang's code examples
are marked-down as "\`\`\`rust", as follows:


```rust
/// This is the entrypoint to a Conlang program.
///
/// Functions are declared with a name, argument list,
/// optional return type, and an expression.
fn main ()
    println("Hello, world!");

/// Function arguments are patterns
fn println(..args: [String]) {
    for arg in args {
        print(arg)
    }
    print('\n')
}
```


The language is currently in a state of flux: major developments are happening in private as we
prepare to replace the AST, parser, and all subsequent passes with newer, simpler, and remarkably
more flexible implementations.


## Development Goals
### Immediate Goals:
- [ ] Remove statements as a syntactic position
  - [ ] There can be only one (expression per translation unit)
  - [ ] Update the [Grammatical grammar](/grammar.ebnf) to match expectations
- [ ] Carefully evaluate all situations where a value can be returned, and figure out what value

### Short Goals:
- [ ] Rebuild the type inference machinery, with The Technology
- [ ] Expand pattern-binding to cover every way a name can be declared
- [ ] Implement a bytecode VM for standard library development and comptime
- [ ] Typeclass polymorphism. All functions f(self) are "extension methods", but we crave MORE

### Long Goals:
- [ ] Compile to LLVM IR, for that sweet, sweet portability.
- [ ] Create a best-in-class standard library
  - [ ] Consider runtime type information
- [ ] Port the compiler to Conlang
