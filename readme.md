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


##  Stability
Conlang is currently in a state of flux. Its grammar is currently
defined by the implementation of a hybrid Pratt/recursive descent
parser, rather than by a formal grammar. Critical information,
such as the precedence of operators, is subject to change without
notice as the language matures.

The language will *visually* stay the same -- there is an explicit
project goal to keep all existing Conlang code parsing "correctly"
-- but the semantics of the language will likely remain unstable
for the forseeable future.


## Building Conlang from Source

### External dependencies
Conlang relies on a very limited number of external dependencies,
which are available on `crates.io`:
- First-party dependencies:
  - `Repline`: A readline-like library which powers Conlang's REPLs
  - `cl-arena`: Arena allocators for nightly Rust
- Third-party dependencies:
  - `unicode-ident`: Efficient `XID_START` and `XID_CONTINUE` predicates

The dependency tree of Conlang is *intentionally limited* to keep
compile times low and make eventual self-hosting easier.
It may, at some point in the future, rely on a large compiler
backend like `LLVM` for cross-platform support.


### Unstable Rust
Conlang is (at the moment) a pure-Rust project, but it uses some unstable Rust features:
- `decl_macro` for better, more hygienic declarative macros (`macro do_thing (...) {...}`)
- `dropck_eyepatch` for containers which may be dropped after their contents' lifetimes
- `rev_into_inner` and `string_into_chars` to address ownership challenges in the `cl-interpret`er

These features require the use of a `nightly` Rust toolchain. If you use Rustup, you can download
one with
```sh
rustup toolchain install nightly
```
or by building the project with the `+nightly` flag:
```sh
cargo +nightly build
# or
cargo +nightly run
```

For users without access to rustup, you can currently (though it isn't officially supported)
enable unstable features on stable Rust with the `RUSTC_BOOTSTRAP` environment variable:
```sh
RUSTC_BOOTSTRAP=1 cargo build
# or
RUSTC_BOOTSTRAP=1 cargo run
```

To install Conlang, then, you can use `cargo install`:
```sh
cargo +nightly install --path .
# or
RUSTC_BOOTSTRAP=1 cargo install --path .
```


## Programming in Conlang
While Conlang, as a fledgling language, does not have the platform
integration of larger, more complete projects, and its interpreter
is naive and very slow, the language itself is complete enough to
write fun and engaging little programs with.

Conlang's interpreter comes with a variety of native-code builtins
such as `print(...)`, `println(...)`, `string.chars()`, etc.,
as well as Conlang-code utilities like `sqrt(f64)` and `hex(u64)`.

The full list of interpreter builtins, user-defined constants and
functions can be displayed with the `dump()` builtin.


### Expressions
In Conlang, everything is an expression! See the [tutorial](/src/tutorial.cl) for more information.


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
    putchar('\n')
}
```


### Code examples

The file [dummy.cl](/dummy.cl) contains the earliest, and most approachable, look at Conlang syntax.
It still runs in the interpreter, and there are no plans to break compatibility with that file
in particular. Of course, it does not cover all language features; those language features unused
in the file are subject to change.


A minor goal of the project is to keep the syntax *roughly* compatible with Rust, so regex-based
code highlighters will highlight the code in an acceptable manner. As such, Conlang's code examples
are marked-down as "\`\`\`rust", as follows:



## Development Goals
For Conlang to be considered complete, it needs to satisfy a variety of whimsical and practical
criteria. This section tracks the overall progress toward those criteria.

### Immediate Goals:
- [x] Remove statements as a syntactic position
  - [x] There can be only one (expression per translation unit)
  - [ ] Update the [Grammatical grammar](/grammar.ebnf) to match expectations
- [ ] Carefully evaluate all situations where a value can be returned, and figure out what value

### Short Goals:
- [ ] Perform name resolution ahead-of-time
  - [x] Expand pattern-binding to cover every way a name can be declared
  - [ ] Bind identifiers to places in scopes, and keep those bindings stable
  - [ ] Transform the AST into a form which references places instead of names
- [ ] Rebuild the type inference machinery, with The Technology
- [ ] Implement a bytecode VM for standard library development and comptime
- [ ] Typeclass polymorphism. All functions f(self) are "extension methods", but we crave MORE

### Long Goals:
- [ ] Compile to LLVM IR, for that sweet, sweet portability.
- [ ] Create a best-in-class standard library
  - [ ] Implement fully-featured compile-time reflection, with code generation
  - [ ] Implement run-time reflection, without code generation
- [ ] Port the compiler to Conlang
