//! Conlang is an expression-based programming language
#![warn(clippy::all)]

pub mod token {
    //! Stores a component of a file as a type and span
    use std::ops::Range;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Type {
        Invalid,
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
        // Compound punctuation
        Lsh,
        Rsh,
        AndAnd,
        OrOr,
        NotNot,
        CatEar,
        EqEq,
        NotEq,
        StarEq,
        DivEq,
        AddEq,
        SubEq,
        AndEq,
        OrEq,
        XorEq,
        LshEq,
        RshEq,
        Arrow,
        FatArrow,
        // Simple punctuation
        Semi,
        Dot,
        Star,
        Div,
        Plus,
        Minus,
        Rem,
        Bang,
        Eq,
        Lt,
        Gt,
        Amp,
        Bar,
        Xor,
        Hash,
        At,
        Colon,
        Backslash,
        Question,
        Comma,
        Tilde,
        Grave,
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
        /// Advances the cursor and produces a token from a provided [Rule] function
        fn map_rule<F>(&mut self, rule: F, ty: Type) -> Option<Token>
        where F: Fn(Rule) -> Rule {
            self.skip_whitespace();
            let start = self.cursor;
            self.cursor += Rule::new(self.text()).and(rule).end()?;
            Some(Token::new(ty, start, self.cursor))
        }
        /// Gets a slice of text beginning at the cursor
        fn text(&self) -> &str {
            &self.text[self.cursor..]
        }
        // classifies a single arbitrary token
        /// Returns the result of the rule with the highest precedence, if any matches
        pub fn any(&mut self) -> Option<Token> {
            None.or_else(|| self.comment())
                .or_else(|| self.keyword())
                .or_else(|| self.identifier())
                .or_else(|| self.literal())
                .or_else(|| self.delimiter())
                .or_else(|| self.punctuation())
                .or_else(|| self.invalid())
        }
        /// Attempts to produce a Keyword
        pub fn keyword(&mut self) -> Option<Token> {
            None.or_else(|| self.kw_else())
                .or_else(|| self.kw_for())
                .or_else(|| self.kw_fn())
                .or_else(|| self.kw_if())
                .or_else(|| self.kw_in())
                .or_else(|| self.kw_let())
                .or_else(|| self.kw_while())
        }
        /// Attempts to produce a [Type::LitString], [Type::LitFloat], or [Type::LitInteger]
        pub fn literal(&mut self) -> Option<Token> {
            None.or_else(|| self.lit_string())
                .or_else(|| self.lit_float())
                .or_else(|| self.lit_integer())
        }
        /// Evaluates delimiter rules
        pub fn delimiter(&mut self) -> Option<Token> {
            None.or_else(|| self.l_brack())
                .or_else(|| self.r_brack())
                .or_else(|| self.l_curly())
                .or_else(|| self.r_curly())
                .or_else(|| self.l_paren())
                .or_else(|| self.r_paren())
        }
        /// Evaluates punctuation rules
        pub fn punctuation(&mut self) -> Option<Token> {
            None.or_else(|| self.lsh())
                .or_else(|| self.rsh())
                .or_else(|| self.and_and())
                .or_else(|| self.or_or())
                .or_else(|| self.not_not())
                .or_else(|| self.cat_ear())
                .or_else(|| self.eq_eq())
                .or_else(|| self.not_eq())
                .or_else(|| self.star_eq())
                .or_else(|| self.div_eq())
                .or_else(|| self.add_eq())
                .or_else(|| self.sub_eq())
                .or_else(|| self.and_eq())
                .or_else(|| self.or_eq())
                .or_else(|| self.xor_eq())
                .or_else(|| self.lsh_eq())
                .or_else(|| self.rsh_eq())
                .or_else(|| self.arrow())
                .or_else(|| self.fatarrow())
                .or_else(|| self.semi())
                .or_else(|| self.dot())
                .or_else(|| self.star())
                .or_else(|| self.div())
                .or_else(|| self.plus())
                .or_else(|| self.sub())
                .or_else(|| self.rem())
                .or_else(|| self.bang())
                .or_else(|| self.eq())
                .or_else(|| self.lt())
                .or_else(|| self.gt())
                .or_else(|| self.amp())
                .or_else(|| self.bar())
                .or_else(|| self.xor())
                .or_else(|| self.hash())
                .or_else(|| self.at())
                .or_else(|| self.colon())
                .or_else(|| self.backslash())
                .or_else(|| self.question())
                .or_else(|| self.comma())
                .or_else(|| self.tilde())
                .or_else(|| self.grave())
        }
        pub fn unary_op(&mut self) -> Option<Token> {
            self.bang().or_else(|| self.sub())
        }
        // functions for lexing individual tokens
        pub fn invalid(&mut self) -> Option<Token> {
            self.map_rule(|r| r.invalid(), Type::Invalid)
        }
        // comments
        pub fn comment(&mut self) -> Option<Token> {
            self.map_rule(|r| r.comment(), Type::Comment)
        }
        // keywords
        pub fn kw_else(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("else"), Type::KwElse)
        }
        pub fn kw_for(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("for"), Type::KwFor)
        }
        pub fn kw_fn(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("fn"), Type::KwFn)
        }
        pub fn kw_if(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("if"), Type::KwIf)
        }
        pub fn kw_in(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("in"), Type::KwIn)
        }
        pub fn kw_let(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("let"), Type::KwLet)
        }
        pub fn kw_while(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("while"), Type::KwWhile)
        }
        // identifiers
        pub fn identifier(&mut self) -> Option<Token> {
            self.map_rule(|r| r.identifier(), Type::Identifier)
        }
        // literals
        pub fn lit_integer(&mut self) -> Option<Token> {
            self.map_rule(|r| r.integer(), Type::LitInteger)
        }
        pub fn lit_float(&mut self) -> Option<Token> {
            self.map_rule(|r| r.float(), Type::LitFloat)
        }
        pub fn lit_string(&mut self) -> Option<Token> {
            self.map_rule(|r| r.string(), Type::LitString)
        }
        // delimiters
        pub fn l_brack(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('['), Type::LBrack)
        }
        pub fn r_brack(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char(']'), Type::RBrack)
        }
        pub fn l_curly(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('{'), Type::LCurly)
        }
        pub fn r_curly(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('}'), Type::RCurly)
        }
        pub fn l_paren(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('('), Type::LParen)
        }
        pub fn r_paren(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char(')'), Type::RParen)
        }
        // compound punctuation
        pub fn lsh(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("<<"), Type::Lsh)
        }
        pub fn rsh(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str(">>"), Type::Rsh)
        }
        pub fn and_and(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("&&"), Type::AndAnd)
        }
        pub fn or_or(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("||"), Type::OrOr)
        }
        pub fn not_not(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("!!"), Type::NotNot)
        }
        pub fn cat_ear(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("^^"), Type::CatEar)
        }
        pub fn eq_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("=="), Type::EqEq)
        }
        pub fn not_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("!="), Type::NotEq)
        }
        pub fn star_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("*="), Type::StarEq)
        }
        pub fn div_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("/="), Type::DivEq)
        }
        pub fn add_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("+="), Type::AddEq)
        }
        pub fn sub_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("-="), Type::SubEq)
        }
        pub fn and_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("&="), Type::AndEq)
        }
        pub fn or_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("|="), Type::OrEq)
        }
        pub fn xor_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("^="), Type::XorEq)
        }
        pub fn lsh_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("<<="), Type::LshEq)
        }
        pub fn rsh_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str(">>="), Type::RshEq)
        }
        pub fn arrow(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("->"), Type::Arrow)
        }
        pub fn fatarrow(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("=>"), Type::FatArrow)
        }
        // simple punctuation
        pub fn semi(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char(';'), Type::Semi)
        }
        pub fn dot(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('.'), Type::Dot)
        }
        pub fn star(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('*'), Type::Star)
        }
        pub fn div(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('/'), Type::Div)
        }
        pub fn plus(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('+'), Type::Plus)
        }
        pub fn sub(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('-'), Type::Minus)
        }
        pub fn rem(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('%'), Type::Rem)
        }
        pub fn bang(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('!'), Type::Bang)
        }
        pub fn eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('='), Type::Eq)
        }
        pub fn lt(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('<'), Type::Lt)
        }
        pub fn gt(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('>'), Type::Gt)
        }
        pub fn amp(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('&'), Type::Amp)
        }
        pub fn bar(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('|'), Type::Bar)
        }
        pub fn xor(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('^'), Type::Xor)
        }
        pub fn hash(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('#'), Type::Hash)
        }
        pub fn at(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('@'), Type::At)
        }
        pub fn colon(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char(':'), Type::Colon)
        }
        pub fn question(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('?'), Type::Question)
        }
        pub fn comma(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char(','), Type::Comma)
        }
        pub fn tilde(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('~'), Type::Tilde)
        }
        pub fn grave(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('`'), Type::Grave)
        }
        pub fn backslash(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('\\'), Type::Backslash)
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
        /// Matches any sequence of non-whitespace characters
        pub fn invalid(self) -> Self {
            self.and_many(Self::not_whitespace)
        }
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
        /// Matches anything but whitespace
        pub fn not_whitespace(self) -> Self {
            self.char_fn(|c| !c.is_whitespace())
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
            let tok =
                f(&mut Lexer::new(input)).unwrap_or_else(|| panic!("Should be {ty:?}, {range:?}"));
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
        mod literal {
            use super::*;
            #[test]
            fn literal_class() {
                assert_whole_input_is_token("1_00000", Lexer::literal, Type::LitInteger);
                assert_whole_input_is_token("1.00000", Lexer::literal, Type::LitFloat);
                assert_whole_input_is_token("\"1.0\"", Lexer::literal, Type::LitString);
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
        }
        mod delimiter {
            use super::*;
            #[test]
            fn delimiter_class() {
                assert_whole_input_is_token("[", Lexer::delimiter, Type::LBrack);
                assert_whole_input_is_token("]", Lexer::delimiter, Type::RBrack);
                assert_whole_input_is_token("{", Lexer::delimiter, Type::LCurly);
                assert_whole_input_is_token("}", Lexer::delimiter, Type::RCurly);
                assert_whole_input_is_token("(", Lexer::delimiter, Type::LParen);
                assert_whole_input_is_token(")", Lexer::delimiter, Type::RParen);
            }
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
        mod punctuation {
            use super::*;
            mod compound {
                use super::*;

                #[test]
                fn lsh() {
                    assert_whole_input_is_token("<<", Lexer::lsh, Type::Lsh)
                }
                #[test]
                fn rsh() {
                    assert_whole_input_is_token(">>", Lexer::rsh, Type::Rsh)
                }
                #[test]
                fn and_and() {
                    assert_whole_input_is_token("&&", Lexer::and_and, Type::AndAnd)
                }
                #[test]
                fn or_or() {
                    assert_whole_input_is_token("||", Lexer::or_or, Type::OrOr)
                }
                #[test]
                fn not_not() {
                    assert_whole_input_is_token("!!", Lexer::not_not, Type::NotNot)
                }
                #[test]
                fn cat_ear() {
                    assert_whole_input_is_token("^^", Lexer::cat_ear, Type::CatEar)
                }
                #[test]
                fn eq_eq() {
                    assert_whole_input_is_token("==", Lexer::eq_eq, Type::EqEq)
                }
                #[test]
                fn not_eq() {
                    assert_whole_input_is_token("!=", Lexer::not_eq, Type::NotEq)
                }
                #[test]
                fn star_eq() {
                    assert_whole_input_is_token("*=", Lexer::star_eq, Type::StarEq)
                }
                #[test]
                fn div_eq() {
                    assert_whole_input_is_token("/=", Lexer::div_eq, Type::DivEq)
                }
                #[test]
                fn add_eq() {
                    assert_whole_input_is_token("+=", Lexer::add_eq, Type::AddEq)
                }
                #[test]
                fn sub_eq() {
                    assert_whole_input_is_token("-=", Lexer::sub_eq, Type::SubEq)
                }
                #[test]
                fn and_eq() {
                    assert_whole_input_is_token("&=", Lexer::and_eq, Type::AndEq)
                }
                #[test]
                fn or_eq() {
                    assert_whole_input_is_token("|=", Lexer::or_eq, Type::OrEq)
                }
                #[test]
                fn xor_eq() {
                    assert_whole_input_is_token("^=", Lexer::xor_eq, Type::XorEq)
                }
                #[test]
                fn lsh_eq() {
                    assert_whole_input_is_token("<<=", Lexer::lsh_eq, Type::LshEq)
                }
                #[test]
                fn rsh_eq() {
                    assert_whole_input_is_token(">>=", Lexer::rsh_eq, Type::RshEq)
                }
            }

            mod simple {
                use super::*;
                #[test]
                fn punctuation_class() {
                    assert_whole_input_is_token(";", Lexer::punctuation, Type::Semi);
                    assert_whole_input_is_token(".", Lexer::punctuation, Type::Dot);
                    assert_whole_input_is_token("*", Lexer::punctuation, Type::Star);
                    assert_whole_input_is_token("/", Lexer::punctuation, Type::Div);
                    assert_whole_input_is_token("+", Lexer::punctuation, Type::Plus);
                    assert_whole_input_is_token("-", Lexer::punctuation, Type::Minus);
                    assert_whole_input_is_token("%", Lexer::punctuation, Type::Rem);
                    assert_whole_input_is_token("!", Lexer::punctuation, Type::Bang);
                    assert_whole_input_is_token("=", Lexer::punctuation, Type::Eq);
                    assert_whole_input_is_token("<", Lexer::punctuation, Type::Lt);
                    assert_whole_input_is_token(">", Lexer::punctuation, Type::Gt);
                    assert_whole_input_is_token("&", Lexer::punctuation, Type::Amp);
                    assert_whole_input_is_token("|", Lexer::punctuation, Type::Bar);
                    assert_whole_input_is_token("^", Lexer::punctuation, Type::Xor);
                    assert_whole_input_is_token("#", Lexer::punctuation, Type::Hash);
                    assert_whole_input_is_token("@", Lexer::punctuation, Type::At);
                    assert_whole_input_is_token(":", Lexer::punctuation, Type::Colon);
                    assert_whole_input_is_token("?", Lexer::punctuation, Type::Question);
                    assert_whole_input_is_token(",", Lexer::punctuation, Type::Comma);
                    assert_whole_input_is_token("~", Lexer::punctuation, Type::Tilde);
                    assert_whole_input_is_token("`", Lexer::punctuation, Type::Grave);
                    assert_whole_input_is_token("\\", Lexer::punctuation, Type::Backslash);
                }
                // individual functions below
                #[test]
                fn semi() {
                    assert_whole_input_is_token(";", Lexer::semi, Type::Semi)
                }
                #[test]
                fn dot() {
                    assert_whole_input_is_token(".", Lexer::dot, Type::Dot)
                }
                #[test]
                fn star() {
                    assert_whole_input_is_token("*", Lexer::star, Type::Star)
                }
                #[test]
                fn div() {
                    assert_whole_input_is_token("/", Lexer::div, Type::Div)
                }
                #[test]
                fn plus() {
                    assert_whole_input_is_token("+", Lexer::plus, Type::Plus)
                }
                #[test]
                fn minus() {
                    assert_whole_input_is_token("-", Lexer::sub, Type::Minus)
                }
                #[test]
                fn rem() {
                    assert_whole_input_is_token("%", Lexer::rem, Type::Rem)
                }
                #[test]
                fn bang() {
                    assert_whole_input_is_token("!", Lexer::bang, Type::Bang)
                }
                #[test]
                fn eq() {
                    assert_whole_input_is_token("=", Lexer::eq, Type::Eq)
                }
                #[test]
                fn lt() {
                    assert_whole_input_is_token("<", Lexer::lt, Type::Lt)
                }
                #[test]
                fn gt() {
                    assert_whole_input_is_token(">", Lexer::gt, Type::Gt)
                }
                #[test]
                fn and() {
                    assert_whole_input_is_token("&", Lexer::amp, Type::Amp)
                }
                #[test]
                fn or() {
                    assert_whole_input_is_token("|", Lexer::bar, Type::Bar)
                }
                #[test]
                fn xor() {
                    assert_whole_input_is_token("^", Lexer::xor, Type::Xor)
                }
                #[test]
                fn hash() {
                    assert_whole_input_is_token("#", Lexer::hash, Type::Hash)
                }
                #[test]
                fn at() {
                    assert_whole_input_is_token("@", Lexer::at, Type::At)
                }
                #[test]
                fn colon() {
                    assert_whole_input_is_token(":", Lexer::colon, Type::Colon)
                }
                #[test]
                fn backslash() {
                    assert_whole_input_is_token("\\", Lexer::backslash, Type::Backslash)
                }
                #[test]
                fn question() {
                    assert_whole_input_is_token("?", Lexer::question, Type::Question)
                }
                #[test]
                fn comma() {
                    assert_whole_input_is_token(",", Lexer::comma, Type::Comma)
                }
                #[test]
                fn tilde() {
                    assert_whole_input_is_token("~", Lexer::tilde, Type::Tilde)
                }
                #[test]
                fn grave() {
                    assert_whole_input_is_token("`", Lexer::grave, Type::Grave)
                }
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
