mod token {
    use crate::token::*;
    #[test]
    fn token_has_type() {
        assert_eq!(Token::new(Type::Comment, 0, 10, 1, 1).ty(), Type::Comment);
        assert_eq!(
            Token::new(Type::Identifier, 0, 10, 1, 1).ty(),
            Type::Identifier
        );
    }
    #[test]
    fn token_has_range() {
        let t = Token::new(Type::Comment, 0, 10, 1, 1);
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
    mod identifier {
        use super::*;

        #[test]
        fn identifier() {
            assert_whole_input_is_token("valid_identifier", Lexer::identifier, Type::Identifier);
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
            assert_whole_input_is_token("1_00000", Lexer::literal, Type::Integer);
            assert_whole_input_is_token("1.00000", Lexer::literal, Type::Float);
            assert_has_type_and_range("\"1.0\"", Lexer::literal, Type::String, 1..4);
            assert_has_type_and_range("'\"'", Lexer::literal, Type::Character, 1..2);
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
                assert_has_type_and_range("0x1234", Lexer::integer, Type::Integer, 0..6);
                assert_has_type_and_range("0x1234 \"hello\"", Lexer::integer, Type::Integer, 0..6);
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
        mod float {
            use super::*;
            #[test]
            fn number_dot_number_is_float() {
                assert_whole_input_is_token("1.0", Lexer::float, Type::Float);
            }
            #[test]
            fn nothing_dot_number_is_float() {
                assert_whole_input_is_token(".0", Lexer::float, Type::Float);
            }
            #[test]
            #[should_panic]
            fn number_dot_nothing_is_not_float() {
                assert_whole_input_is_token("1.", Lexer::float, Type::Float);
            }
            #[test]
            #[should_panic]
            fn nothing_dot_nothing_is_not_float() {
                assert_whole_input_is_token(".", Lexer::float, Type::Float);
            }
        }
        mod string {
            use super::*;
            #[test]
            fn empty_string() {
                assert_has_type_and_range("\"\"", Lexer::string, Type::String, 1..1);
            }
            #[test]
            fn unicode_string() {
                assert_has_type_and_range("\"I 💙 🦈!\"", Lexer::string, Type::String, 1..13);
            }
            #[test]
            fn escape_string() {
                assert_has_type_and_range(
                    "\" \\\"This is a quote\\\" \"",
                    Lexer::string,
                    Type::String,
                    1..22,
                );
            }
        }
        mod char {
            use super::*;
            #[test]
            fn plain_char() {
                assert_has_type_and_range("'A'", Lexer::character, Type::Character, 1..2);
                assert_has_type_and_range("'a'", Lexer::character, Type::Character, 1..2);
                assert_has_type_and_range("'#'", Lexer::character, Type::Character, 1..2);
            }
            #[test]
            fn unicode_char() {
                assert_has_type_and_range("'ε'", Lexer::character, Type::Character, 1..3);
            }
            #[test]
            fn escaped_char() {
                assert_has_type_and_range("'\\n'", Lexer::character, Type::Character, 1..3);
            }
            #[test]
            #[should_panic]
            fn no_char() {
                assert_has_type_and_range("''", Lexer::character, Type::Character, 1..1);
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
            fn dot_dot() {
                assert_whole_input_is_token("..", Lexer::dot_dot, Type::DotDot)
            }
            #[test]
            fn dot_dot_eq() {
                assert_whole_input_is_token("..=", Lexer::dot_dot_eq, Type::DotDotEq)
            }
            #[test]
            fn lt_lt() {
                assert_whole_input_is_token("<<", Lexer::lt_lt, Type::LtLt)
            }
            #[test]
            fn gt_gt() {
                assert_whole_input_is_token(">>", Lexer::gt_gt, Type::GtGt)
            }
            #[test]
            fn amp_amp() {
                assert_whole_input_is_token("&&", Lexer::amp_amp, Type::AmpAmp)
            }
            #[test]
            fn bar_bar() {
                assert_whole_input_is_token("||", Lexer::bar_bar, Type::BarBar)
            }
            #[test]
            fn bang_bang() {
                assert_whole_input_is_token("!!", Lexer::bang_bang, Type::BangBang)
            }
            #[test]
            fn xor_xor() {
                assert_whole_input_is_token("^^", Lexer::xor_xor, Type::XorXor)
            }
            #[test]
            fn eq_eq() {
                assert_whole_input_is_token("==", Lexer::eq_eq, Type::EqEq)
            }
            #[test]
            fn gt_eq() {
                assert_whole_input_is_token(">=", Lexer::gt_eq, Type::GtEq)
            }
            #[test]
            fn lt_eq() {
                assert_whole_input_is_token("<=", Lexer::lt_eq, Type::LtEq)
            }
            #[test]
            fn bang_eq() {
                assert_whole_input_is_token("!=", Lexer::bang_eq, Type::BangEq)
            }
            #[test]
            fn star_eq() {
                assert_whole_input_is_token("*=", Lexer::star_eq, Type::StarEq)
            }
            #[test]
            fn slash_eq() {
                assert_whole_input_is_token("/=", Lexer::slash_eq, Type::SlashEq)
            }
            #[test]
            fn plus_eq() {
                assert_whole_input_is_token("+=", Lexer::plus_eq, Type::PlusEq)
            }
            #[test]
            fn minus_eq() {
                assert_whole_input_is_token("-=", Lexer::minus_eq, Type::MinusEq)
            }
            #[test]
            fn amp_eq() {
                assert_whole_input_is_token("&=", Lexer::amp_eq, Type::AmpEq)
            }
            #[test]
            fn bar_eq() {
                assert_whole_input_is_token("|=", Lexer::bar_eq, Type::BarEq)
            }
            #[test]
            fn xor_eq() {
                assert_whole_input_is_token("^=", Lexer::xor_eq, Type::XorEq)
            }
            #[test]
            fn lt_lt_eq() {
                assert_whole_input_is_token("<<=", Lexer::lt_lt_eq, Type::LtLtEq)
            }
            #[test]
            fn gt_gt_eq() {
                assert_whole_input_is_token(">>=", Lexer::gt_gt_eq, Type::GtGtEq)
            }
        }

        mod simple {
            use super::*;
            #[test]
            fn punctuation_class() {
                // go from least to most specific
                assert_whole_input_is_token(";", Lexer::punctuation, Type::Semi);
                assert_whole_input_is_token(".", Lexer::punctuation, Type::Dot);
                assert_whole_input_is_token("*", Lexer::punctuation, Type::Star);
                assert_whole_input_is_token("/", Lexer::punctuation, Type::Slash);
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
                assert_whole_input_is_token("<<", Lexer::punctuation, Type::LtLt);
                assert_whole_input_is_token(">>", Lexer::punctuation, Type::GtGt);
                assert_whole_input_is_token("&&", Lexer::punctuation, Type::AmpAmp);
                assert_whole_input_is_token("||", Lexer::punctuation, Type::BarBar);
                assert_whole_input_is_token("!!", Lexer::punctuation, Type::BangBang);
                assert_whole_input_is_token("^^", Lexer::punctuation, Type::XorXor);
                assert_whole_input_is_token("==", Lexer::punctuation, Type::EqEq);
                assert_whole_input_is_token(">=", Lexer::punctuation, Type::GtEq);
                assert_whole_input_is_token("<=", Lexer::punctuation, Type::LtEq);
                assert_whole_input_is_token("!=", Lexer::punctuation, Type::BangEq);
                assert_whole_input_is_token("*=", Lexer::punctuation, Type::StarEq);
                assert_whole_input_is_token("/=", Lexer::punctuation, Type::SlashEq);
                assert_whole_input_is_token("+=", Lexer::punctuation, Type::PlusEq);
                assert_whole_input_is_token("-=", Lexer::punctuation, Type::MinusEq);
                assert_whole_input_is_token("&=", Lexer::punctuation, Type::AmpEq);
                assert_whole_input_is_token("|=", Lexer::punctuation, Type::BarEq);
                assert_whole_input_is_token("^=", Lexer::punctuation, Type::XorEq);
                assert_whole_input_is_token("..", Lexer::punctuation, Type::DotDot);
                assert_whole_input_is_token("..=", Lexer::punctuation, Type::DotDotEq);
                assert_whole_input_is_token("<<=", Lexer::punctuation, Type::LtLtEq);
                assert_whole_input_is_token(">>=", Lexer::punctuation, Type::GtGtEq);
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
            fn slash() {
                assert_whole_input_is_token("/", Lexer::slash, Type::Slash)
            }
            #[test]
            fn plus() {
                assert_whole_input_is_token("+", Lexer::plus, Type::Plus)
            }
            #[test]
            fn minus() {
                assert_whole_input_is_token("-", Lexer::minus, Type::Minus)
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
            fn amp() {
                assert_whole_input_is_token("&", Lexer::amp, Type::Amp)
            }
            #[test]
            fn bar() {
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
