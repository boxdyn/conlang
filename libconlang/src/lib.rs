//! Conlang is an expression-based programming language with similarities to Rust
#![warn(clippy::all)]
#![feature(decl_macro)]
pub mod token;

pub mod ast;

pub mod lexer;

pub mod parser;

pub mod pretty_printer {
    use super::ast::preamble::*;
    use std::{
        fmt::Display,
        io::{stdout, Result as IOResult, StdoutLock, Write},
    };
    pub trait PrettyPrintable {
        fn print(&self);
        fn write(&self, into: impl Write) -> IOResult<()>;
    }
    impl PrettyPrintable for Start {
        fn print(&self) {
            let _ = self.walk(&mut Printer::default());
        }
        fn write(&self, into: impl Write) -> IOResult<()> {
            self.walk(&mut Printer::from(into))
        }
    }

    #[derive(Debug)]
    pub struct Printer<W: Write> {
        level: u32,
        writer: W,
    }
    impl<'t> Default for Printer<StdoutLock<'t>> {
        fn default() -> Self {
            Self { level: 0, writer: stdout().lock() }
        }
    }
    impl<W: Write> From<W> for Printer<W> {
        fn from(writer: W) -> Self {
            Self { level: 0, writer }
        }
    }
    impl<W: Write> Printer<W> {
        fn pad(&mut self) -> IOResult<&mut Self> {
            for _ in 0..self.level * 4 {
                write!(self.writer, " ")?;
            }
            Ok(self)
        }
        fn newline(&mut self) -> IOResult<&mut Self> {
            writeln!(self.writer)?;
            self.pad()
        }
        fn put(&mut self, d: impl Display) -> IOResult<&mut Self> {
            write!(self.writer, "{d} ")?;
            Ok(self)
        }
        /// Increase the indentation level by 1
        fn indent(&mut self) -> &mut Self {
            self.level += 1;
            self
        }
        fn dedent(&mut self) -> &mut Self {
            self.level -= 1;
            self
        }
    }
    macro visit_math($self:expr, $expr:expr) {{
        $expr.0.walk($self)?;
        for (op, target) in &$expr.1 {
            op.walk($self)?;
            target.walk($self)?;
        }
        Ok(())
    }}
    impl<W: Write> Visitor<IOResult<()>> for Printer<W> {
        fn visit_ignore(&mut self, expr: &math::Ignore) -> IOResult<()> {
            expr.0.walk(self)?;
            for (op, target) in &expr.1 {
                op.walk(self)?;
                target.walk(self.newline()?)?;
            }
            Ok(())
        }
        fn visit_assign(&mut self, expr: &math::Assign) -> IOResult<()> {
            visit_math!(self, expr)
        }
        fn visit_compare(&mut self, expr: &math::Compare) -> IOResult<()> {
            visit_math!(self, expr)
        }
        fn visit_logic(&mut self, expr: &math::Logic) -> IOResult<()> {
            visit_math!(self, expr)
        }
        fn visit_bitwise(&mut self, expr: &math::Bitwise) -> IOResult<()> {
            visit_math!(self, expr)
        }
        fn visit_shift(&mut self, expr: &math::Shift) -> IOResult<()> {
            visit_math!(self, expr)
        }
        fn visit_term(&mut self, expr: &math::Term) -> IOResult<()> {
            visit_math!(self, expr)
        }
        fn visit_factor(&mut self, expr: &math::Factor) -> IOResult<()> {
            visit_math!(self, expr)
        }
        fn visit_unary(&mut self, expr: &math::Unary) -> IOResult<()> {
            for op in &expr.0 {
                op.walk(self)?;
            }
            expr.1.walk(self)
        }
        fn visit_ignore_op(&mut self, op: &operator::Ignore) -> IOResult<()> {
            self.put(match op {
                operator::Ignore::Ignore => "\x08;",
            })
            .map(drop)
        }
        fn visit_compare_op(&mut self, op: &operator::Compare) -> IOResult<()> {
            self.put(match op {
                operator::Compare::Less => "<",
                operator::Compare::LessEq => "<=",
                operator::Compare::Equal => "==",
                operator::Compare::NotEq => "!=",
                operator::Compare::GreaterEq => ">=",
                operator::Compare::Greater => ">",
            })
            .map(drop)
        }
        fn visit_assign_op(&mut self, op: &operator::Assign) -> IOResult<()> {
            self.put(match op {
                operator::Assign::Assign => "=",
                operator::Assign::AddAssign => "+=",
                operator::Assign::SubAssign => "-=",
                operator::Assign::MulAssign => "*=",
                operator::Assign::DivAssign => "/=",
                operator::Assign::BitAndAssign => "&=",
                operator::Assign::BitOrAssign => "|=",
                operator::Assign::BitXorAssign => "^=",
                operator::Assign::ShlAssign => "<<=",
                operator::Assign::ShrAssign => ">>=",
            })
            .map(drop)
        }
        fn visit_logic_op(&mut self, op: &operator::Logic) -> IOResult<()> {
            self.put(match op {
                operator::Logic::LogAnd => "&&",
                operator::Logic::LogOr => "||",
                operator::Logic::LogXor => "^^",
            })
            .map(drop)
        }
        fn visit_bitwise_op(&mut self, op: &operator::Bitwise) -> IOResult<()> {
            self.put(match op {
                operator::Bitwise::BitAnd => "&",
                operator::Bitwise::BitOr => "|",
                operator::Bitwise::BitXor => "^",
            })
            .map(drop)
        }
        fn visit_shift_op(&mut self, op: &operator::Shift) -> IOResult<()> {
            self.put(match op {
                operator::Shift::Lsh => "<<",
                operator::Shift::Rsh => ">>",
            })
            .map(drop)
        }
        fn visit_term_op(&mut self, op: &operator::Term) -> IOResult<()> {
            self.put(match op {
                operator::Term::Add => "+",
                operator::Term::Sub => "-",
            })
            .map(drop)
        }
        fn visit_factor_op(&mut self, op: &operator::Factor) -> IOResult<()> {
            self.put(match op {
                operator::Factor::Mul => "*",
                operator::Factor::Div => "/",
                operator::Factor::Rem => "%",
            })
            .map(drop)
        }
        fn visit_unary_op(&mut self, op: &operator::Unary) -> IOResult<()> {
            self.put(match op {
                operator::Unary::Deref => "*",
                operator::Unary::Ref => "&",
                operator::Unary::Neg => "-",
                operator::Unary::Not => "!",
                operator::Unary::At => "@",
                operator::Unary::Hash => "#",
                operator::Unary::Tilde => "~",
            })
            .map(drop)
        }

        fn visit_if(&mut self, expr: &control::If) -> IOResult<()> {
            expr.cond.walk(self.put("if")?)?;
            expr.body.walk(self)?;
            if let Some(e) = &expr.else_ {
                e.walk(self)?
            }
            Ok(())
        }
        fn visit_while(&mut self, expr: &control::While) -> IOResult<()> {
            expr.cond.walk(self.put("while")?)?;
            expr.body.walk(self)?;
            if let Some(e) = &expr.else_ {
                e.walk(self)?
            }
            Ok(())
        }
        fn visit_for(&mut self, expr: &control::For) -> IOResult<()> {
            expr.var.walk(self.put("for")?)?;
            expr.iter.walk(self.put("in")?)?;
            expr.body.walk(self)?;
            self.visit_block(&expr.body)?;
            if let Some(e) = &expr.else_ {
                e.walk(self)?
            }
            Ok(())
        }
        fn visit_else(&mut self, expr: &control::Else) -> IOResult<()> {
            expr.block.walk(self.put("else")?)
        }
        fn visit_continue(&mut self, _expr: &control::Continue) -> IOResult<()> {
            self.put("continue").map(drop)
        }
        fn visit_break(&mut self, expr: &control::Break) -> IOResult<()> {
            expr.expr.walk(self.put("break")?)
        }
        fn visit_return(&mut self, expr: &control::Return) -> IOResult<()> {
            expr.expr.walk(self.put("return")?)
        }

        fn visit_identifier(&mut self, ident: &Identifier) -> IOResult<()> {
            self.put(&ident.0).map(drop)
        }
        fn visit_string_literal(&mut self, string: &str) -> IOResult<()> {
            self.put("\"")?.put(string)?.put("\"").map(drop)
        }
        fn visit_char_literal(&mut self, char: &char) -> IOResult<()> {
            self.put(char).map(drop)
        }
        fn visit_bool_literal(&mut self, bool: &bool) -> IOResult<()> {
            self.put(bool).map(drop)
        }
        fn visit_float_literal(&mut self, float: &literal::Float) -> IOResult<()> {
            self.put(float.sign)?
                .put(float.exponent)?
                .put(float.mantissa)
                .map(drop)
        }
        fn visit_int_literal(&mut self, int: &u128) -> IOResult<()> {
            self.put(int).map(drop)
        }

        fn visit_block(&mut self, expr: &expression::Block) -> IOResult<()> {
            self.put('{')?.indent().newline()?.visit_expr(&expr.expr)?;
            self.dedent().newline()?.put('}').map(drop)
        }

        fn visit_group(&mut self, expr: &expression::Group) -> IOResult<()> {
            self.put('(')?;
            self.visit_expr(&expr.expr)?;
            self.put(')').map(drop)
        }
    }
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
                    assert_has_type_and_range(
                        "0x1234 \"hello\"",
                        Lexer::integer,
                        Type::Integer,
                        0..6,
                    );
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
                        1..22
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
                fn lsh() {
                    assert_whole_input_is_token("<<", Lexer::lsh, Type::Lsh)
                }
                #[test]
                fn rsh() {
                    assert_whole_input_is_token(">>", Lexer::rsh, Type::Rsh)
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
                fn gt_eq() {
                    assert_whole_input_is_token(">=", Lexer::gt_eq, Type::GtEq)
                }
                #[test]
                fn lt_eq() {
                    assert_whole_input_is_token("<=", Lexer::lt_eq, Type::LtEq)
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
