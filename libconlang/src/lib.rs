//! Conlang is an expression-based programming language
#![warn(clippy::all)]

pub mod token {
    //! Stores a component of a file as a type and span
    use std::ops::Range;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Type {
        Comment,
        Identifier,
        Integer,
        String,
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
        fn skip_whitespace(&mut self) {
            if let Some(len) = Rule::new(self.text()).and_any(Rule::whitespace).end() {
                self.cursor += len
            }
        }
        fn produce_token(&mut self, ty: Type, len: usize) -> Option<Token> {
            let start = self.cursor;
            self.cursor += len;
            Some(Token::new(ty, start, self.cursor))
        }
        fn text(&self) -> &str {
            &self.text[self.cursor..]
        }
        // functions for lexing individual tokens
        pub fn line_comment(&mut self) -> Option<Token> {
            // line_comment := "//" ~ (^newline)*
            self.skip_whitespace();
            self.produce_token(
                Type::Comment,
                Rule::new(self.text())
                    .str("//")
                    .and_any(|rule| rule.not_char('\n'))
                    .end()?,
            )
        }
        pub fn block_comment(&mut self) -> Option<Token> {
            // block_comment := "/*" ~ (block_comment | all_but("*/"))* ~ "*/"
            self.skip_whitespace();
            self.produce_token(
                Type::Comment,
                Rule::new(self.text())
                    .str("/*")
                    .and_any(|rule| rule.not_str("*/"))
                    .str("*/")
                    .end()?,
            )
        }
        pub fn shebang_comment(&mut self) -> Option<Token> {
            // shebang_comment := "#!/" ~ (^newline)*
            self.skip_whitespace();
            self.produce_token(
                Type::Comment,
                Rule::new(self.text())
                    .str("#!/")
                    .and_any(|rule| rule.not_char('\n'))
                    .end()?,
            )
        }
        pub fn identifier(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(
                Type::Identifier,
                Rule::new(self.text())
                    .char('_')
                    .or(Rule::xid_start)
                    .and_any(Rule::xid_continue)
                    .end()?,
            )
        }
        pub fn integer(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(
                Type::Integer,
                Rule::new(self.text())
                    .and_one_of(&[
                        &|rule| rule.str("0x").and_any(Rule::hex_digit),
                        &|rule| rule.str("0d").and_any(Rule::dec_digit),
                        &|rule| rule.str("0o").and_any(Rule::oct_digit),
                        &|rule| rule.str("0b").and_any(Rule::bin_digit),
                        &|rule| rule.and_many(Rule::dec_digit),
                    ])
                    .end()?,
            )
        }
        pub fn string(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(
                Type::String,
                Rule::new(self.text())
                    .char('"')
                    .and_any(|rule| rule.and(Rule::string_escape).or(|rule| rule.not_char('"')))
                    .char('"')
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
        pub fn remaining(&self) -> &str {
            self.text
        }
    }

    impl<'t> Rule<'t> {
        pub fn char_between(self, start: char, end: char) -> Self {
            self.char_fn(|c| start <= c && c <= end)
        }
        pub fn char(self, c: char) -> Self {
            self.has(|rule| rule.text.starts_with(c), 1)
        }
        pub fn str(self, s: &str) -> Self {
            self.has(|rule| rule.text.starts_with(s), s.len())
        }
        pub fn char_fn(self, f: impl Fn(char) -> bool) -> Self {
            self.and(|rule| match rule.text.strip_prefix(&f) {
                Some(text) => Self { text, taken: rule.taken + next_utf8(rule.text, 1), ..rule },
                None => Self { is_alright: false, ..rule },
            })
        }
        pub fn not_char(self, c: char) -> Self {
            self.has(|rule| !rule.text.starts_with(c), 1)
        }
        pub fn not_str(self, s: &str) -> Self {
            self.has(|rule| !rule.text.starts_with(s), 1)
        }
        pub fn any(self) -> Self {
            self.has(|_| true, 1)
        }
        pub fn whitespace(self) -> Self {
            self.char_fn(|c| c.is_whitespace())
        }
        pub fn xid_start(self) -> Self {
            use unicode_xid::UnicodeXID;
            self.char_fn(UnicodeXID::is_xid_start)
        }
        pub fn xid_continue(self) -> Self {
            use unicode_xid::UnicodeXID;
            self.char_fn(UnicodeXID::is_xid_continue)
        }
        pub fn hex_digit(self) -> Self {
            self.char_fn(|c| c.is_ascii_hexdigit())
        }
        pub fn dec_digit(self) -> Self {
            self.char_fn(|c| c.is_ascii_digit())
        }
        pub fn oct_digit(self) -> Self {
            self.char_between('0', '7')
        }
        pub fn bin_digit(self) -> Self {
            self.char_between('0', '1')
        }
        pub fn string_escape(self) -> Self {
            self.char('\\').and(Rule::any)
        }
        fn has(self, condition: impl Fn(&Self) -> bool, len: usize) -> Self {
            let len = next_utf8(self.text, len);
            self.and(|rule| match condition(&rule) && !rule.text.is_empty() {
                true => Self { text: &rule.text[len..], taken: rule.taken + len, ..rule },
                false => Self { is_alright: false, ..rule },
            })
        }
    }

    impl<'t> lerox::Combinator for Rule<'t> {
        fn is_alright(&self) -> bool {
            self.is_alright
        }
        fn into_alright(self) -> Self {
            Self { is_alright: true, ..self }
        }
    }

    /// Returns the index of the next unicode character, rounded up
    fn next_utf8(text: &str, mut index: usize) -> usize {
        index = index.min(text.len());
        while !text.is_char_boundary(index) {
            index += 1
        }
        index
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
        fn token_has_type() {
            assert_eq!(Token::new(Type::Comment, 0, 10).ty(), Type::Comment);
            assert_eq!(Token::new(Type::Identifier, 0, 10).ty(), Type::Identifier);
        }
        #[test]
        fn token_has_range() {
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

        fn assert_whole_input_is_token<'t, F>(input: &'t str, f: F, ty: Type)
        where F: FnOnce(&mut Lexer<'t>) -> Option<Token> {
            assert_has_type_and_len(input, f, ty, input.len())
        }
        fn assert_has_type_and_len<'t, F>(input: &'t str, f: F, ty: Type, len: usize)
        where F: FnOnce(&mut Lexer<'t>) -> Option<Token> {
            assert_eq!(Some(Token::new(ty, 0, len)), f(&mut Lexer::new(input)),)
        }

        mod comment {
            use super::*;

            #[test]
            fn line_comment() {
                assert_whole_input_is_token(
                    "// this is a comment",
                    Lexer::line_comment,
                    Type::Comment,
                );
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
        mod identifier {
            use super::*;

            #[test]
            fn identifier() {
                assert_whole_input_is_token(
                    "valid_identifier",
                    Lexer::identifier,
                    Type::Identifier,
                );
                assert_whole_input_is_token("_0", Lexer::identifier, Type::Identifier);
                assert_whole_input_is_token("_", Lexer::identifier, Type::Identifier);
            }
            #[test]
            fn unicode_identifier() {
                assert_whole_input_is_token("ζ_ζζζ_ζζζ_ζζζ", Lexer::identifier, Type::Identifier);
                assert_whole_input_is_token("_ζζζ_ζζζ_ζζζ_", Lexer::identifier, Type::Identifier);
            }
            #[test]
            #[should_panic]
            fn not_identifier() {
                assert_whole_input_is_token("123456789", Lexer::identifier, Type::Identifier);
            }
        }
        mod integer {
            use super::*;
            #[test]
            fn bare() {
                assert_whole_input_is_token("10010110", Lexer::integer, Type::Integer);
                assert_whole_input_is_token("12345670", Lexer::integer, Type::Integer);
                assert_whole_input_is_token("1234567890", Lexer::integer, Type::Integer);
            }
            #[test]
            fn base16() {
                assert_has_type_and_len("0x1234", Lexer::integer, Type::Integer, 6);
                assert_has_type_and_len("0x1234 \"hello\"", Lexer::integer, Type::Integer, 6);
            }
            #[test]
            fn base10() {
                assert_whole_input_is_token("0d1234", Lexer::integer, Type::Integer);
            }
            #[test]
            fn base8() {
                assert_whole_input_is_token("0o1234", Lexer::integer, Type::Integer);
            }
            #[test]
            fn base2() {
                assert_whole_input_is_token("0b1010", Lexer::integer, Type::Integer);
            }
        }
        mod string {
            use super::*;
            #[test]
            fn empty_string() {
                assert_whole_input_is_token("\"\"", Lexer::string, Type::String);
            }
            #[test]
            fn unicode_string() {
                assert_whole_input_is_token("\"I 💙 🦈!\"", Lexer::string, Type::String);
            }
            #[test]
            fn escape_string() {
                assert_whole_input_is_token(
                    r#"" \"This is a quote\" ""#,
                    Lexer::string,
                    Type::String,
                );
            }
        }
    }
    mod parser {
        // TODO
    }
    mod interpreter {
        // TODO
    }
}
