//! Conlang is an expression-based programming language
#![warn(clippy::all)]

pub mod token {
    //! Stores a component of a file as a type and span
    use std::ops::Range;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Type {
        Comment,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Token {
        ty: Type,
        head: usize,
        tail: usize,
    }
    impl Token {
        pub fn new(ty: Type, head: usize, tail: usize) -> Self {
            Self { ty, head, tail }
        }
        pub fn is_empty(&self) -> bool {
            self.tail == self.head
        }
        pub fn len(&self) -> usize {
            self.tail - self.head
        }
        // Gets the [Type] of the token
        pub fn ty(&self) -> Type {
            self.ty
        }
        // Gets the exclusive range of the token
        pub fn range(&self) -> Range<usize> {
            self.head..self.tail
        }
    }
}

pub mod ast {
    //! Stores functions, data structure definitions, etc.
}

pub mod lexer {
    //! Converts a text file into tokens
    use crate::token::{Token, Type};
    use lerox::Combinator;

    #[allow(dead_code)]
    pub struct Lexer<'t> {
        text: &'t str,
        cursor: usize,
    }
    /// Implements the non-terminals of a language
    impl<'t> Lexer<'t> {
        pub fn new(text: &'t str) -> Self {
            Self { text, cursor: 0 }
        }
        fn produce_token(&mut self, ty: Type, len: usize) -> Option<Token> {
            let start = self.cursor;
            self.cursor += len;
            Some(Token::new(ty, start, self.cursor))
        }
        // functions for lexing individual tokens
        pub fn line_comment(&mut self) -> Option<Token> {
            // line_comment := "//" ~ (^newline)*
            self.produce_token(
                Type::Comment,
                Rule::new(self.text)
                    .take_str("//")
                    .and_any(|rule| rule.take_except_char('\n'))
                    .end()?,
            )
        }
        pub fn block_comment(&mut self) -> Option<Token> {
            // block_comment := "/*" ~ (block_comment | all_but("*/"))* ~ "*/"
            self.produce_token(
                Type::Comment,
                Rule::new(self.text)
                    .take_str("/*")
                    .and_any(|rule| rule.take_except_str("*/"))
                    .take_str("*/")
                    .end()?,
            )
        }
        pub fn shebang_comment(&mut self) -> Option<Token> {
            // shebang_comment := "#!/" ~ (^newline)*
            self.produce_token(
                Type::Comment,
                Rule::new(self.text)
                    .take_str("#!/")
                    .and_any(|rule| rule.take_except_char('\n'))
                    .end()?,
            )
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Rule<'t> {
        text: &'t str,
        taken: usize,
        is_alright: bool,
    }
    impl<'t> Rule<'t> {
        pub fn new(text: &'t str) -> Self {
            Self { text, taken: 0, is_alright: true }
        }
        pub fn end(self) -> Option<usize> {
            self.is_alright.then_some(self.taken)
        }
    }

    impl<'t> Rule<'t> {
        pub fn take_char(self, c: char) -> Self {
            self.take(|this| this.text.starts_with(c), 1)
        }
        pub fn take_except_char(self, c: char) -> Self {
            self.take(|this| !this.text.starts_with(c), 1)
        }
        pub fn take_str(self, s: &str) -> Self {
            self.take(|this| this.text.starts_with(s), s.len())
        }
        pub fn take_except_str(self, s: &str) -> Self {
            self.take(|this| !this.text.starts_with(s), 1)
        }
        pub fn take_any(self) -> Self {
            self.take(|_| true, 1)
        }
        fn take(self, condition: impl Fn(&Self) -> bool, len: usize) -> Self {
            self.and(|this| match condition(&this) && !this.text.is_empty() {
                true => Self { text: &this.text[len..], taken: this.taken + len, ..this },
                false => Self { is_alright: false, ..this },
            })
        }
    }

    impl<'t> lerox::Combinable for Rule<'t> {
        fn is_alright(&self) -> bool {
            self.is_alright
        }
        fn alright(self) -> Self {
            Self { is_alright: true, ..self }
        }
    }
}

pub mod parser {
    //! Parses tokens into an AST
}

pub mod interpreter {
    //! Interprets an AST as a program
}

#[cfg(test)]
mod tests {
    mod token {
        use crate::token::*;
        #[test]
        fn token_type_is_stored() {
            let t = Token::new(Type::Comment, 0, 10);
            assert_eq!(t.ty(), Type::Comment);
        }
        #[test]
        fn token_range_is_stored() {
            let t = Token::new(Type::Comment, 0, 10);
            assert_eq!(t.range(), 0..10);
        }
    }
    mod ast {
        // TODO
    }
    mod lexer {
        use crate::{
            lexer::*,
            token::{Token, Type},
        };

        fn assert_whole_input_is_token<'t, F>(input: &'t str, operation: F, output_type: Type)
        where F: FnOnce(&mut Lexer<'t>) -> Option<Token> {
            assert_eq!(
                operation(&mut Lexer::new(input)),
                Some(Token::new(output_type, 0, input.len()))
            );
        }
        #[test]
        fn line_comment() {
            assert_whole_input_is_token("// this is a comment", Lexer::line_comment, Type::Comment);
        }
        #[test]
        #[should_panic]
        fn not_line_comment() {
            assert_whole_input_is_token("fn main() {}", Lexer::line_comment, Type::Comment);
        }
        #[test]
        fn block_comment() {
            assert_whole_input_is_token(
                "/* this is a comment */",
                Lexer::block_comment,
                Type::Comment,
            );
        }
        #[test]
        #[should_panic]
        fn not_block_comment() {
            assert_whole_input_is_token("fn main() {}", Lexer::block_comment, Type::Comment);
        }
        #[test]
        fn shebang_comment() {
            assert_whole_input_is_token(
                "#!/ this is a comment",
                Lexer::shebang_comment,
                Type::Comment,
            );
        }
        #[test]
        #[should_panic]
        fn not_shebang_comment() {
            assert_whole_input_is_token("fn main() {}", Lexer::shebang_comment, Type::Comment);
        }
    }
    mod parser {
        // TODO
    }
    mod interpreter {
        // TODO
    }
}
