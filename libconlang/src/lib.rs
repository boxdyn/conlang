//! Conlang is an expression-based programming language
#![warn(clippy::all)]

pub mod token {
    //! Stores a component of a file as a type and span
    use std::ops::Range;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Type {
        Comment,
        Identifier,
        // Keywords
        KwElse,
        KwFor,
        KwFn,
        KwIf,
        KwIn,
        KwLet,
        KwWhile,
        // Literals
        LitInteger,
        LitFloat,
        LitString,
        // Delimiters
        LCurly,
        RCurly,
        LBrack,
        RBrack,
        LParen,
        RParen,
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
        /// Skips whitespace in the text
        fn skip_whitespace(&mut self) {
            if let Some(len) = Rule::new(self.text()).and_any(Rule::whitespace).end() {
                self.cursor += len
            }
        }
        /// Advances the cursor and produces a token
        fn produce_token(&mut self, ty: Type, len: usize) -> Option<Token> {
            let start = self.cursor;
            self.cursor += len;
            Some(Token::new(ty, start, self.cursor))
        }
        /// Gets a slice of text beginning at the cursor
        fn text(&self) -> &str {
            &self.text[self.cursor..]
        }
        // classifies a single arbitrary token
        pub fn any(&mut self) -> Option<Token> {
            None.or_else(|| self.comment())
                .or_else(|| self.keyword())
                .or_else(|| self.identifier())
                .or_else(|| self.literal())
                .or_else(|| self.delimiter())
        }
        pub fn keyword(&mut self) -> Option<Token> {
            None.or_else(|| self.kw_else())
                .or_else(|| self.kw_for())
                .or_else(|| self.kw_fn())
                .or_else(|| self.kw_if())
                .or_else(|| self.kw_in())
                .or_else(|| self.kw_let())
                .or_else(|| self.kw_while())
        }
        pub fn literal(&mut self) -> Option<Token> {
            None.or_else(|| self.lit_string())
                .or_else(|| self.lit_float())
                .or_else(|| self.lit_integer())
        }
        pub fn delimiter(&mut self) -> Option<Token> {
            None.or_else(|| self.l_brack())
                .or_else(|| self.r_brack())
                .or_else(|| self.l_curly())
                .or_else(|| self.r_curly())
                .or_else(|| self.l_paren())
                .or_else(|| self.r_paren())
        }
        // functions for lexing individual tokens
        // comments
        pub fn comment(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(Type::Comment, Rule::new(self.text()).comment().end()?)
        }
        // keywords
        pub fn kw_else(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(Type::KwElse, Rule::new(self.text()).str("else").end()?)
        }
        pub fn kw_for(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(Type::KwFor, Rule::new(self.text()).str("for").end()?)
        }
        pub fn kw_fn(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(Type::KwFn, Rule::new(self.text()).str("fn").end()?)
        }
        pub fn kw_if(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(Type::KwIf, Rule::new(self.text()).str("if").end()?)
        }
        pub fn kw_in(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(Type::KwIn, Rule::new(self.text()).str("in").end()?)
        }
        pub fn kw_let(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(Type::KwLet, Rule::new(self.text()).str("let").end()?)
        }
        pub fn kw_while(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(Type::KwWhile, Rule::new(self.text()).str("while").end()?)
        }
        // identifiers
        pub fn identifier(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(Type::Identifier, Rule::new(self.text()).identifier().end()?)
        }
        // literals
        pub fn lit_integer(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(Type::LitInteger, Rule::new(self.text()).integer().end()?)
        }
        pub fn lit_float(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(Type::LitFloat, Rule::new(self.text()).float().end()?)
        }
        pub fn lit_string(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(Type::LitString, Rule::new(self.text()).string().end()?)
        }
        // delimiters
        pub fn l_brack(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(Type::LBrack, Rule::new(self.text()).char('[').end()?)
        }
        pub fn r_brack(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(Type::RBrack, Rule::new(self.text()).char(']').end()?)
        }
        pub fn l_curly(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(Type::LCurly, Rule::new(self.text()).char('{').end()?)
        }
        pub fn r_curly(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(Type::RCurly, Rule::new(self.text()).char('}').end()?)
        }
        pub fn l_paren(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(Type::LParen, Rule::new(self.text()).char('(').end()?)
        }
        pub fn r_paren(&mut self) -> Option<Token> {
            self.skip_whitespace();
            self.produce_token(Type::RParen, Rule::new(self.text()).char(')').end()?)
        }
    }

    /// A lexer [Rule] matches patterns in text in a declarative manner
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
        /// Matches a block, line, or shebang comment
        pub fn comment(self) -> Self {
            self.and_either(Self::line_comment, Self::block_comment)
        }
        /// Matches a line or shebang comment
        fn line_comment(self) -> Self {
            // line_comment := ("//" | "#!/") (!newline)*
            self.str("//")
                .or(|r| r.str("#!/"))
                .and_any(|r| r.not_char('\n'))
        }
        /// Matches a block comment
        fn block_comment(self) -> Self {
            // block_comment := "/*" (block_comment | all_but("*/"))* "*/"
            self.str("/*")
                .and_any(|r| r.and_either(|f| f.block_comment(), |g| g.not_str("*/")))
                .str("*/")
        }
        /// Matches a Rust-style identifier
        pub fn identifier(self) -> Self {
            // identifier := ('_' | XID_START) ~ XID_CONTINUE*
            self.char('_')
                .or(Rule::xid_start)
                .and_any(Rule::xid_continue)
        }
        /// Matches a Rust-style base-prefixed int literal
        fn int_literal_kind(self, prefix: &str, digit: impl Fn(Self) -> Self) -> Self {
            // int_kind<Prefix, Digit> := Prefix '_'* Digit (Digit | '_')*
            self.str(prefix)
                .and_any(|r| r.char('_'))
                .and(&digit)
                .and_any(|r| r.and(&digit).or(|r| r.char('_')))
        }
        /// Matches a Rust-style integer literal
        pub fn integer(self) -> Self {
            // integer = (int_kind<0d, dec_digit> | int_kind<0x, hex_digit>
            //           | int_kind<0o, oct_digit> | int_kind<0b, bin_digit> | dec_digit (dec_digit | '_')*)
            self.and_one_of(&[
                &|rule| rule.int_literal_kind("0d", Rule::dec_digit),
                &|rule| rule.int_literal_kind("0x", Rule::hex_digit),
                &|rule| rule.int_literal_kind("0o", Rule::oct_digit),
                &|rule| rule.int_literal_kind("0b", Rule::bin_digit),
                &|rule| {
                    rule.dec_digit()
                        .and_any(|r| r.dec_digit().or(|r| r.char('_')))
                },
            ])
        }
        /// Matches a float literal
        // TODO: exponent form
        pub fn float(self) -> Self {
            self.and_any(Rule::dec_digit)
                .char('.')
                .and_many(Rule::dec_digit)
        }
        /// Matches one quote-delimited string literal
        pub fn string(self) -> Self {
            self.char('"').and_any(Rule::string_continue).char('"')
        }
        /// Matches one string escape sequence or non-`"` characcter
        pub fn string_continue(self) -> Self {
            self.and(Rule::string_escape).or(|rule| rule.not_char('"'))
        }
    }

    impl<'t> Rule<'t> {
        /// Matches a char lexicographically between start and end
        pub fn char_between(self, start: char, end: char) -> Self {
            self.char_fn(|c| start <= c && c <= end)
        }
        /// Matches a single char
        pub fn char(self, c: char) -> Self {
            self.has(|rule| rule.text.starts_with(c), 1)
        }
        /// Matches the entirety of a string slice
        pub fn str(self, s: &str) -> Self {
            self.has(|rule| rule.text.starts_with(s), s.len())
        }
        /// Matches a char based on the output of a function
        pub fn char_fn(self, f: impl Fn(char) -> bool) -> Self {
            self.and(|rule| match rule.text.strip_prefix(&f) {
                Some(text) => Self { text, taken: rule.taken + next_utf8(rule.text, 1), ..rule },
                None => Self { is_alright: false, ..rule },
            })
        }
        /// Matches a single char except c
        pub fn not_char(self, c: char) -> Self {
            self.has(|rule| !rule.text.starts_with(c), 1)
        }
        /// Matches a single char unless the text starts with s
        pub fn not_str(self, s: &str) -> Self {
            self.has(|rule| !rule.text.starts_with(s), 1)
        }
        // commonly used character classes
        /// Matches one of any character
        pub fn any(self) -> Self {
            self.has(|_| true, 1)
        }
        /// Matches one whitespace
        pub fn whitespace(self) -> Self {
            self.char_fn(|c| c.is_whitespace())
        }
        /// Matches one XID_START
        pub fn xid_start(self) -> Self {
            use unicode_xid::UnicodeXID;
            self.char_fn(UnicodeXID::is_xid_start)
        }
        /// Matches one XID_CONTINUE
        pub fn xid_continue(self) -> Self {
            use unicode_xid::UnicodeXID;
            self.char_fn(UnicodeXID::is_xid_continue)
        }
        /// Matches one hexadecimal digit
        pub fn hex_digit(self) -> Self {
            self.char_fn(|c| c.is_ascii_hexdigit())
        }
        /// Matches one decimal digit
        pub fn dec_digit(self) -> Self {
            self.char_fn(|c| c.is_ascii_digit())
        }
        /// Matches one octal digit
        pub fn oct_digit(self) -> Self {
            self.char_between('0', '7')
        }
        /// Matches one binary digit
        pub fn bin_digit(self) -> Self {
            self.char_between('0', '1')
        }
        /// Matches any string escape "\."
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
        use std::ops::Range;

        use crate::{
            lexer::*,
            token::{Token, Type},
        };

        fn assert_whole_input_is_token<'t, F>(input: &'t str, f: F, ty: Type)
        where F: FnOnce(&mut Lexer<'t>) -> Option<Token> {
            assert_has_type_and_range(input, f, ty, 0..input.len())
        }
        fn assert_has_type_and_range<'t, F>(input: &'t str, f: F, ty: Type, range: Range<usize>)
        where F: FnOnce(&mut Lexer<'t>) -> Option<Token> {
            let tok = f(&mut Lexer::new(input)).unwrap();
            assert_eq!(ty, tok.ty());
            assert_eq!(range, tok.range());
        }

        mod comment {
            use super::*;

            #[test]
            fn line_comment() {
                assert_whole_input_is_token("// comment!", Lexer::comment, Type::Comment);
            }
            #[test]
            #[should_panic]
            fn not_line_comment() {
                assert_whole_input_is_token("fn main() {}", Lexer::comment, Type::Comment);
            }
            #[test]
            fn block_comment() {
                assert_whole_input_is_token("/* comment! */", Lexer::comment, Type::Comment);
            }
            #[test]
            fn nested_block_comment() {
                assert_whole_input_is_token(
                    "/* a /* nested */ comment */",
                    Lexer::comment,
                    Type::Comment,
                );
            }
            #[test]
            #[should_panic]
            fn unclosed_nested_comment() {
                assert_whole_input_is_token(
                    "/* improperly /* nested */ comment",
                    Lexer::comment,
                    Type::Comment,
                );
            }
            #[test]
            #[should_panic]
            fn not_block_comment() {
                assert_whole_input_is_token("fn main() {}", Lexer::comment, Type::Comment);
            }
            #[test]
            fn shebang_comment() {
                assert_whole_input_is_token("#!/ comment!", Lexer::comment, Type::Comment);
            }
            #[test]
            #[should_panic]
            fn not_shebang_comment() {
                assert_whole_input_is_token("fn main() {}", Lexer::comment, Type::Comment);
            }
        }
        mod keyword {
            use super::*;
            #[test]
            fn kw_else() {
                assert_whole_input_is_token("else", Lexer::kw_else, Type::KwElse);
                assert_has_type_and_range("  else  ", Lexer::kw_else, Type::KwElse, 2..6);
            }
            #[test]
            fn kw_for() {
                assert_whole_input_is_token("for", Lexer::kw_for, Type::KwFor);
            }
            #[test]
            fn kw_fn() {
                assert_whole_input_is_token("fn", Lexer::kw_fn, Type::KwFn);
            }
            #[test]
            fn kw_if() {
                assert_whole_input_is_token("if", Lexer::kw_if, Type::KwIf);
            }
            #[test]
            fn kw_in() {
                assert_whole_input_is_token("in", Lexer::kw_in, Type::KwIn);
            }
            #[test]
            fn kw_let() {
                assert_whole_input_is_token("let", Lexer::kw_let, Type::KwLet);
            }
            #[test]
            fn kw_while() {
                assert_whole_input_is_token("while", Lexer::kw_while, Type::KwWhile);
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
                assert_whole_input_is_token("10010110", Lexer::lit_integer, Type::LitInteger);
                assert_whole_input_is_token("12345670", Lexer::lit_integer, Type::LitInteger);
                assert_whole_input_is_token("1234567890", Lexer::lit_integer, Type::LitInteger);
            }
            #[test]
            fn base16() {
                assert_has_type_and_range("0x1234", Lexer::lit_integer, Type::LitInteger, 0..6);
                assert_has_type_and_range(
                    "0x1234 \"hello\"",
                    Lexer::lit_integer,
                    Type::LitInteger,
                    0..6,
                );
            }
            #[test]
            fn base10() {
                assert_whole_input_is_token("0d1234", Lexer::lit_integer, Type::LitInteger);
            }
            #[test]
            fn base8() {
                assert_whole_input_is_token("0o1234", Lexer::lit_integer, Type::LitInteger);
            }
            #[test]
            fn base2() {
                assert_whole_input_is_token("0b1010", Lexer::lit_integer, Type::LitInteger);
            }
        }
        mod float {
            use super::*;
            #[test]
            fn number_dot_number_is_float() {
                assert_whole_input_is_token("1.0", Lexer::lit_float, Type::LitFloat);
            }
            #[test]
            fn nothing_dot_number_is_float() {
                assert_whole_input_is_token(".0", Lexer::lit_float, Type::LitFloat);
            }
            #[test]
            #[should_panic]
            fn number_dot_nothing_is_not_float() {
                assert_whole_input_is_token("1.", Lexer::lit_float, Type::LitFloat);
            }
            #[test]
            #[should_panic]
            fn nothing_dot_nothing_is_not_float() {
                assert_whole_input_is_token(".", Lexer::lit_float, Type::LitFloat);
            }
        }
        mod string {
            use super::*;
            #[test]
            fn empty_string() {
                assert_whole_input_is_token("\"\"", Lexer::lit_string, Type::LitString);
            }
            #[test]
            fn unicode_string() {
                assert_whole_input_is_token("\"I 💙 🦈!\"", Lexer::lit_string, Type::LitString);
            }
            #[test]
            fn escape_string() {
                assert_whole_input_is_token(
                    r#"" \"This is a quote\" ""#,
                    Lexer::lit_string,
                    Type::LitString,
                );
            }
        }
        mod delimiter {
            use super::*;
            #[test]
            fn l_brack() {
                assert_whole_input_is_token("[", Lexer::l_brack, Type::LBrack);
            }
            #[test]
            fn r_brack() {
                assert_whole_input_is_token("]", Lexer::r_brack, Type::RBrack);
            }
            #[test]
            fn l_curly() {
                assert_whole_input_is_token("{", Lexer::l_curly, Type::LCurly);
            }
            #[test]
            fn r_curly() {
                assert_whole_input_is_token("}", Lexer::r_curly, Type::RCurly);
            }

            #[test]
            fn l_paren() {
                assert_whole_input_is_token("(", Lexer::l_paren, Type::LParen);
            }
            #[test]
            fn r_paren() {
                assert_whole_input_is_token(")", Lexer::r_paren, Type::RParen);
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
