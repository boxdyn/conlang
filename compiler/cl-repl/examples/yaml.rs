//! Pretty prints a conlang AST in yaml

use cl_ast::Expr;
use cl_lexer::Lexer;
use cl_parser::Parser;
use cl_structures::intern::interned::Symbol;
use repline::{Response, error::Error as RlError};

fn main() -> Result<(), RlError> {
    repline::read_and("\x1b[33m", "y> ", " > ", |line| {
        let mut parser = Parser::new(Lexer::new(Symbol::default(), line));
        let code = parser.parse::<Expr>(0)?;

        Yamler::new().yaml(&code);
        println!();
        Ok(Response::Accept)
    })
}

pub use yamler::Yamler;
pub mod yamler {
    use crate::yamlify::Yamlify;
    use std::{
        io::Write,
        ops::{Deref, DerefMut},
    };
    #[derive(Debug, Default)]
    pub struct Yamler {
        depth: usize,
        needs_indent: bool,
    }

    impl Yamler {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn indent(&mut self) -> Section<'_> {
            Section::new(self)
        }

        /// Prints a [Yamlify] value
        #[inline]
        pub fn yaml<T: Yamlify>(&mut self, yaml: &T) -> &mut Self {
            yaml.yaml(self);
            self
        }

        fn newline(&mut self) -> &mut Self {
            if !self.needs_indent {
                println!();
            }
            self.needs_indent = true;
            self
        }

        fn increase(&mut self) {
            self.depth += 1;
        }

        fn decrease(&mut self) {
            self.depth -= 1;
        }

        fn print_indentation(&mut self, writer: &mut impl Write) {
            if !self.needs_indent {
                return;
            }
            for _ in 0..self.depth {
                let _ = write!(writer, "  ");
            }
            self.needs_indent = false
        }

        /// Prints a section header and increases indentation
        pub fn key(&mut self, name: impl Yamlify) -> Section<'_> {
            self.print_indentation(&mut std::io::stdout().lock());
            name.yaml(self);
            println!(":");
            self.needs_indent = true;
            self.newline().indent()
        }

        /// Prints a yaml key value pair: `- name: "value"`
        pub fn pair<D: Yamlify, T: Yamlify>(&mut self, name: D, value: T) -> &mut Self {
            self.print_indentation(&mut std::io::stdout().lock());
            self.key(name).value(value);
            self
        }

        /// Prints a yaml scalar value: `"name"``
        pub fn value<D: Yamlify>(&mut self, value: D) -> &mut Self {
            self.print_indentation(&mut std::io::stdout().lock());
            value.yaml(self);
            self.newline()
        }

        pub fn list<D: Yamlify>(&mut self, list: &[D]) -> &mut Self {
            for value in list {
                self.print_indentation(&mut std::io::stdout().lock());
                print!("- ");
                self.yaml(value).newline();
            }
            self
        }
    }

    /// Tracks the start and end of an indented block (a "section")
    pub struct Section<'y> {
        yamler: &'y mut Yamler,
    }

    impl<'y> Section<'y> {
        pub fn new(yamler: &'y mut Yamler) -> Self {
            yamler.increase();
            Self { yamler }
        }
    }

    impl Deref for Section<'_> {
        type Target = Yamler;
        fn deref(&self) -> &Self::Target {
            self.yamler
        }
    }
    impl DerefMut for Section<'_> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.yamler
        }
    }

    impl Drop for Section<'_> {
        fn drop(&mut self) {
            let Self { yamler } = self;
            yamler.decrease();
        }
    }
}

pub mod yamlify {
    use super::yamler::Yamler;
    use cl_ast::{
        AstTypes,
        types::{Literal, Path, Symbol},
        *,
    };
    use cl_structures::span::Span;

    pub trait Yamlify {
        fn yaml(&self, y: &mut Yamler);
    }

    impl Yamlify for Expr {
        fn yaml(&self, y: &mut Yamler) {
            match self {
                Self::Omitted => y.yaml(&"Omitted"),
                Self::Id(path) => y.yaml(path),
                Self::MetId(id) => y.yaml(id),
                Self::Lit(lit) => y.yaml(lit),
                Self::Use(item) => y.yaml(item),
                Self::Bind(bind) => y.yaml(bind),
                Self::Make(make) => y.yaml(make),
                Self::Op(op, annos) => y.pair(op, annos),
            };
        }
    }

    impl Yamlify for Pat {
        fn yaml(&self, y: &mut Yamler) {
            match self {
                Self::Ignore => y.yaml(&"_"),
                Self::Never => y.yaml(&"!"),
                Self::MetId(id) => y.pair("meta", id),
                Self::Name(name) => y.pair("named", name),
                Self::Value(body) => y.pair("constant", body),
                Self::Op(op, pats) => y.pair(op, pats),
            };
        }
    }

    impl Yamlify for Bind {
        fn yaml(&self, y: &mut Yamler) {
            let Self(op, gens, pat, exprs) = self;
            y.key(op)
                .pair("generics", gens)
                .pair("pattern", pat)
                .pair("body", exprs);
        }
    }

    impl Yamlify for Make {
        fn yaml(&self, y: &mut Yamler) {
            let Self(ty, arms) = self;
            y.pair(ty, arms);
        }
    }

    impl Yamlify for MakeArm {
        fn yaml(&self, y: &mut Yamler) {
            let Self(name, expr) = self;
            y.pair(name, expr);
        }
    }

    impl Yamlify for Use {
        fn yaml(&self, y: &mut Yamler) {
            match self {
                Self::Glob => y.yaml(&"*"),
                Self::Name(name) => y.yaml(name),
                Self::Alias(from, to) => y.pair(from, to),
                Self::Path(name, rest) => y.pair(name, rest),
                Self::Tree(items) => y.yaml(items),
            };
        }
    }

    impl<T: Yamlify + Annotation, A: AstTypes> Yamlify for At<T, A>
    where A::Annotation: Yamlify
    {
        fn yaml(&self, y: &mut Yamler) {
            let Self(t, _) = self;
            y.yaml(t);
        }
    }

    impl<T: Yamlify> Yamlify for Option<T> {
        fn yaml(&self, y: &mut Yamler) {
            if let Some(v) = self {
                y.yaml(v);
            } else {
                y.yaml(&"");
            }
        }
    }
    impl<T: Yamlify> Yamlify for Box<T> {
        fn yaml(&self, y: &mut Yamler) {
            y.yaml(&**self);
        }
    }
    impl<T: Yamlify> Yamlify for Vec<T> {
        fn yaml(&self, y: &mut Yamler) {
            y.list(self);
        }
    }
    impl Yamlify for () {
        fn yaml(&self, _y: &mut Yamler) {}
    }

    impl<T: Yamlify> Yamlify for &T {
        fn yaml(&self, y: &mut Yamler) {
            (*self).yaml(y)
        }
    }

    macro_rules! scalar {
        ($($t:ty),*$(,)?) => {
            $(impl Yamlify for $t {
                fn yaml(&self, _y: &mut Yamler) {
                    print!("{self}");
                }
            })*
        };
    }

    macro_rules! debug_scalar {
        ($($t:ty),*$(,)?) => {
            $(impl Yamlify for $t {
                fn yaml(&self, _y: &mut Yamler) {
                    print!("{self:?}");
                }
            })*
        };
    }

    scalar! {
        bool, char, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, str, &str, String,
        Symbol, Path, Literal
    }

    debug_scalar!(Op, BindOp, PatOp, Span);
}
