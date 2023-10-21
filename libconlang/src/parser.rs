//! Parses [tokens](super::token) into an [AST](super::ast)
use std::vec;

use super::{
    ast::preamble::*,
    lexer::Lexer,
    token::{Keyword, Token, Type},
};
use constr::ConstrTools;
use error::{Error, Reason::*, *};

pub mod error {
    use super::{Token, Type};
    use std::fmt::Display;

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub enum Reason {
        Expected(Type),
        NotIdentifier,
        NotOperator,
        NotLiteral,
        NotString,
        NotChar,
        NotBool,
        NotFloat,
        NotInt,
        FloatExponentOverflow,
        FloatMantissaOverflow,
        IntOverflow,
        NotBranch,
        IncompleteBranch,
        AllElseFailed,
        EndOfFile,
        PanicStackUnderflow,
        #[default]
        Unspecified,
    }
    use Reason::*;

    impl Display for Reason {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Expected(t) => write!(f, "Expected {t}"),
                Self::NotIdentifier => "Not an identifier".fmt(f),
                Self::NotOperator => "Not an operator".fmt(f),
                Self::NotLiteral => "Not a literal".fmt(f),
                Self::NotString => "Not a string".fmt(f),
                Self::NotChar => "Not a char".fmt(f),
                Self::NotBool => "Not a bool".fmt(f),
                Self::NotFloat => "Not a float".fmt(f),
                Self::FloatExponentOverflow => "Float exponent too large".fmt(f),
                Self::FloatMantissaOverflow => "Float mantissa too large".fmt(f),
                Self::NotInt => "Not an integer".fmt(f),
                Self::IntOverflow => "Integer too large".fmt(f),
                Self::IncompleteBranch => "Branch expression was incomplete".fmt(f),
                Self::NotBranch => "Expected branch expression".fmt(f),
                Self::AllElseFailed => "Did not match any rule".fmt(f),
                Self::EndOfFile => "Got end of file".fmt(f),
                Self::PanicStackUnderflow => "Could not recover from panic".fmt(f),
                Self::Unspecified => {
                    "Unspecified error. You are permitted to slap the code author.".fmt(f)
                }
            }
        }
    }

    /// [Parser](super::Parser) [Result]
    pub type PResult<T> = Result<T, Error>;
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct Error {
        reason: Reason,
        start: Option<Token>,
    }

    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if let Some(token) = self.start {
                write!(f, "{}:{}: ", token.line(), token.col())?;
            }
            write!(f, "{}", self.reason)
        }
    }

    macro error_impl($($fn:ident$(($($p:ident: $t:ty),*))?: $reason:expr),*$(,)?) {$(
    /// Creates an [Error] with this [Reason]:
    #[doc = concat!("[`", stringify!($reason), "`]")]
        pub fn $fn($($($p : $t),*)?) -> Self {
            Self { reason: $reason$(($($p)*))?, start: None }
        }
    )*}
    impl Error {
        pub fn token(self, start: Token) -> Self {
            Self { start: Some(start), ..self }
        }
        pub fn maybe_token(self, start: Option<Token>) -> Self {
            Self { start, ..self }
        }
        pub fn start(&self) -> Option<Token> {
            self.start
        }
        pub fn reason(self, reason: Reason) -> Self {
            Self { reason, ..self }
        }
        error_impl! {
            expected(e: Type): Expected,
            not_identifier: NotIdentifier,
            not_operator: NotOperator,
            not_literal: NotLiteral,
            not_string: NotString,
            not_char: NotChar,
            not_bool: NotBool,
            not_float: NotFloat,
            float_exponent_overflow: FloatExponentOverflow,
            float_mantissa_overflow: FloatMantissaOverflow,
            not_int: NotInt,
            int_overflow: IntOverflow,
            not_branch: NotBranch,
            all_else_failed: AllElseFailed,
            end_of_file: EndOfFile,
            panic_underflow: PanicStackUnderflow,
            unspecified: Unspecified,
        }
    }
}

/// The Parser performs recursive descent on the AST's grammar
/// using a provided [Lexer].
pub struct Parser<'t> {
    tokens: Vec<Token>,
    panic_stack: Vec<usize>,
    text: &'t str,
    curr: usize,
}
impl<'t> From<Lexer<'t>> for Parser<'t> {
    fn from(value: Lexer<'t>) -> Self {
        let (tokens, text) = value.consume();
        Self::new(tokens, text)
    }
}

impl<'t> Parser<'t> {
    /// Create a new [Parser] from a list of [Tokens][1]
    /// and the [text](str) used to generate that list
    /// (as [Tokens][1] do not store their strings)
    ///
    /// [1]: Token
    pub fn new(tokens: Vec<Token>, text: &'t str) -> Self {
        Self { tokens, text, panic_stack: vec![], curr: 0 }
    }
    /// Parse the [start of an AST](Start)
    pub fn parse(&mut self) -> PResult<Start> {
        self.consume_comments();
        Ok(Start(self.expr()?))
    }
    /// Consumes any consecutive comments
    fn consume_comments(&mut self) -> &mut Self {
        while let Ok(Type::Comment) = self.peek().map(|t| t.ty()) {
            self.curr += 1;
        }
        self
    }
    /// Consume the current token
    #[inline]
    pub fn consume(&mut self) -> &mut Self {
        self.curr += 1;
        self.consume_comments();
        self
    }
    /// Peek at the current token
    pub fn peek(&self) -> PResult<&Token> {
        self.tokens
            .get(self.curr)
            .ok_or(Error::end_of_file().maybe_token(self.tokens.last().copied()))
    }
    /// Records the current position on the panic stack
    pub fn mark(&mut self) -> &mut Self {
        self.panic_stack.push(self.curr);
        self
    }
    /// Erases a recorded position from the panic stack
    pub fn unmark(&mut self) -> &mut Self {
        self.panic_stack.pop();
        self
    }
    /// Unwinds the panic stack one step
    pub fn unwind(&mut self) -> PResult<&mut Self> {
        let v = self.panic_stack.pop().ok_or(Error::panic_underflow())?;
        self.curr = v;
        Ok(self)
    }
    pub fn advance_until(&mut self, t: Type) -> PResult<&mut Self> {
        while self.matches(t).is_err() {
            self.check_eof()
                .map_err(|e| e.reason(Expected(t)))?
                .consume();
        }
        Ok(self)
    }
}
/// Helpers
impl<'t> Parser<'t> {
    fn consume_type(&mut self, t: Type) -> PResult<&mut Self> {
        self.matches(t)?;
        Ok(self.consume())
    }
    fn check_eof(&mut self) -> PResult<&mut Self> {
        if self.curr < self.tokens.len() {
            Ok(self)
        } else {
            Err(Error::end_of_file().maybe_token(self.tokens.last().copied()))
        }
    }
    fn todo_error(&mut self, l: u32, c: u32, s: &str) -> Error {
        eprintln!("TODO: {s}:{l}:{c}");
        Error::unspecified().token(*self.peek().unwrap())
    }
    fn matches(&mut self, e: Type) -> PResult<&Token> {
        let t = self.check_eof()?.peek().expect("self should not be eof");
        if t.ty() != e {
            Err(Error::expected(e).token(*t))?
        }
        Ok(t)
    }
    fn keyword(&mut self, keyword: Keyword) -> PResult<&mut Self> {
        self.consume_type(Type::Keyword(keyword))
    }
    fn delimited<F, R>(&mut self, lhs: Type, mid: F, rhs: Type) -> PResult<R>
    where F: Fn(&mut Self) -> PResult<R> {
        self.consume_type(lhs)?.mark();
        let out = match mid(self) {
            Ok(out) => out,
            Err(e) => {
                eprintln!("{e}");
                // Jump back in time and try to re-parse from the next brace
                self.unwind()?.advance_until(lhs)?.mark();
                return self.delimited(lhs, mid, rhs);
            }
        };
        self.consume_type(rhs)?.unmark();
        Ok(out)
    }
}
macro ptodo_err($self:expr $(, $t:expr)*) {
    $($t;)*
    $self.todo_error(line!(), column!(), file!())
}
macro ptodo($self:expr $(, $t:expr)*) {
    $($t;)*
    Err(ptodo_err!($self))
}

/// # Terminals and Pseudo-Terminals
impl<'t> Parser<'t> {
    fn identifier(&mut self) -> PResult<Identifier> {
        let token = *self
            .matches(Type::Identifier)
            .map_err(|e| Error::not_identifier().maybe_token(e.start()))?;
        Ok(Identifier(self.consume().text[&token].into()))
    }
    fn literal(&mut self) -> PResult<literal::Literal> {
        use literal::Literal::*;
        use Keyword::{False, True};
        let tok = self.peek()?;
        match tok.ty() {
            Type::Float => self.float().map(Float),
            Type::Integer => self.int().map(Int),
            Type::String => self.string().map(String),
            Type::Character => self.char().map(Char),
            Type::Keyword(True | False) => self.bool().map(Bool),
            _ => Err(Error::not_literal().token(*tok)),
        }
    }
    fn float(&mut self) -> PResult<literal::Float> {
        ptodo!(self)
    }
    fn int(&mut self) -> PResult<u128> {
        let token = *self.matches(Type::Integer)?;
        self.consume().text[&token]
            .chars()
            .parse_int::<u128>()
            .next()
            .ok_or(Error::not_int().token(token))
    }
    fn string(&mut self) -> PResult<String> {
        let range = self
            .matches(Type::String)
            .map_err(|e| e.reason(NotString))?
            .range();
        Ok(self.consume().text[range].chars().unescape().collect())
    }
    fn char(&mut self) -> PResult<char> {
        let token = *self.matches(Type::Character)?;
        self.consume().text[&token]
            .chars()
            .unescape()
            .next()
            .ok_or(Error::not_char().token(token))
    }
    fn bool(&mut self) -> PResult<bool> {
        use Keyword::{False, True};
        let token = self.peek()?;
        let out = match token.ty() {
            Type::Keyword(False) => false,
            Type::Keyword(True) => true,
            _ => Err(Error::not_bool().token(*token))?,
        };
        self.consume();
        Ok(out)
    }
}
/// Expressions
impl<'t> Parser<'t> {
    fn expr(&mut self) -> PResult<expression::Expr> {
        use expression::Expr;
        Ok(Expr { ignore: self.ignore()? })
    }
    fn block(&mut self) -> PResult<expression::Block> {
        self.delimited(Type::LCurly, |p| p.expr(), Type::RCurly)
            .map(|e| expression::Block { expr: Box::new(e) })
    }
    fn group(&mut self) -> PResult<expression::Group> {
        use expression::Group;
        let t = self.consume_type(Type::LParen)?.peek()?;
        match t.ty() {
            Type::RParen => {
                self.consume();
                Ok(Group::Empty)
            }
            _ => {
                let out = self.expr().map(|expr| Group::Expr(expr.into()));
                self.consume_type(Type::RParen)?;
                out
            }
        }
    }
    fn primary(&mut self) -> PResult<expression::Primary> {
        use expression::Primary;
        let token = *self.peek()?;
        match token.ty() {
            Type::Identifier => self.identifier().map(Primary::Identifier),
            Type::String
            | Type::Character
            | Type::Integer
            | Type::Float
            | Type::Keyword(Keyword::True | Keyword::False) => self.literal().map(Primary::Literal),
            Type::LCurly => self.block().map(Primary::Block),
            Type::LParen => self.group().map(Primary::Group),
            Type::Keyword(_) => self.flow().map(Primary::Branch),
            _ => Err(Error::all_else_failed().token(token))?,
        }
    }
}

/// Helper macro for math parsing subexpressions with production
/// ```ebnf
/// Ret = a (b a)*
/// ```
/// # Examples
/// ```rust,ignore
/// binary!{
///     function_name: ret::Value = parse_operands, parse_operators;
/// }
/// ```
/// becomes
/// ```rust,ignore
/// fn function_name(&mut self) -> PResult<ret::Value> { ... }
/// ```
macro binary ($($f:ident = $a:ident, $b:ident);*$(;)?) {$(
    fn $f (&mut self) -> PResult<math::Operation> {
        let (first, mut others) = (self.$a()?, vec![]);
        while let Ok(op) = self.$b() {
            others.push((op, self.$a()?));
        }
        Ok(if others.is_empty() { first } else {
            math::Operation::binary(first, others)
        })
    }
)*}
/// # [Arithmetic and Logical Subexpressions](math)
impl<'t> Parser<'t> {
    binary! {
        //name    operands operators
        ignore  = assign,  ignore_op;
        assign  = compare, assign_op;
        compare = logic,   compare_op;
        logic   = bitwise, logic_op;
        bitwise = shift,   bitwise_op;
        shift   = term,    shift_op;
        term    = factor,  term_op;
        factor  = unary,   factor_op;
    }

    fn unary(&mut self) -> PResult<math::Operation> {
        let mut operators = vec![];
        while let Ok(op) = self.unary_op() {
            operators.push(op)
        }
        Ok(math::Operation::Unary { operators, operand: self.primary()? })
    }
}
macro operator_impl ($($(#[$m:meta])* $f:ident : {$($type:pat => $op:ident),*$(,)?})*) {
    $($(#[$m])* fn $f(&mut self) -> PResult<operator::Binary> {
        use operator::Binary;
        let token = *self.peek()?;
        let out = Ok(match token.ty() {
            $($type => Binary::$op,)*
            _ => Err(Error::not_operator().token(token))?,
        });
        self.consume();
        out
    })*
}
/// # [Operators](operator)
impl<'t> Parser<'t> {
    operator_impl! {
        factor_op: {
            Type::Star => Mul,
            Type::Slash => Div,
            Type::Rem => Rem,
        }
        term_op: {
            Type::Plus => Add,
            Type::Minus => Sub,
        }
        shift_op: {
            Type::LtLt => Lsh,
            Type::GtGt => Rsh,
        }
        bitwise_op: {
            Type::Amp => BitAnd,
            Type::Bar => BitOr,
            Type::Xor => BitXor,
        }
        logic_op: {
            Type::AmpAmp => LogAnd,
            Type::BarBar => LogOr,
            Type::XorXor => LogXor,
        }
        compare_op: {
            Type::Lt => Less,
            Type::LtEq => LessEq,
            Type::EqEq => Equal,
            Type::BangEq => NotEq,
            Type::GtEq => GreaterEq,
            Type::Gt => Greater,
        }
        assign_op: {
            Type::Eq => Assign,
            Type::PlusEq => AddAssign,
            Type::MinusEq => SubAssign,
            Type::StarEq => MulAssign,
            Type::SlashEq => DivAssign,
            Type::RemEq => RemAssign,
            Type::AmpEq => BitAndAssign,
            Type::BarEq => BitOrAssign,
            Type::XorEq => BitXorAssign,
            Type::LtLtEq => ShlAssign,
            Type::GtGtEq => ShrAssign,
        }
        ignore_op: {
            Type::Semi => Ignore,
        }

    }
    /// Parse a [unary operator](operator::Unary)
    fn unary_op(&mut self) -> PResult<operator::Unary> {
        use operator::Unary;
        let token = *self.peek()?;
        let out = Ok(match token.ty() {
            Type::AmpAmp => Unary::RefRef,
            Type::Amp => Unary::Ref,
            Type::Star => Unary::Deref,
            Type::Minus => Unary::Neg,
            Type::Bang => Unary::Not,
            Type::At => Unary::At,
            Type::Hash => Unary::Hash,
            Type::Tilde => Unary::Tilde,
            _ => Err(Error::not_operator().token(token))?,
        });
        self.consume();
        out
    }
}
/// # [Control Flow](control)
impl<'t> Parser<'t> {
    fn flow(&mut self) -> PResult<control::Flow> {
        use control::Flow;
        use Keyword::{Break, Continue, For, If, Return, While};
        let token = *self.peek()?;
        match token.ty() {
            Type::Keyword(While) => self.parse_while().map(Flow::While),
            Type::Keyword(For) => self.parse_for().map(Flow::For),
            Type::Keyword(If) => self.parse_if().map(Flow::If),
            Type::Keyword(Break) => self.parse_break().map(Flow::Break),
            Type::Keyword(Return) => self.parse_return().map(Flow::Return),
            Type::Keyword(Continue) => self.parse_continue().map(Flow::Continue),
            _ => Err(Error::all_else_failed().token(token)),
        }
        .map_err(|e| e.reason(IncompleteBranch).token(token))
    }
    fn parse_if(&mut self) -> PResult<control::If> {
        self.keyword(Keyword::If)?;
        Ok(control::If {
            cond: self.expr()?.into(),
            body: self.block()?,
            else_: self.parse_else()?,
        })
    }
    fn parse_while(&mut self) -> PResult<control::While> {
        self.keyword(Keyword::While)?;
        Ok(control::While {
            cond: self.expr()?.into(),
            body: self.block()?,
            else_: self.parse_else()?,
        })
    }
    fn parse_for(&mut self) -> PResult<control::For> {
        self.keyword(Keyword::For)?;
        Ok(control::For {
            var: self.identifier()?,
            iter: { self.keyword(Keyword::In)?.expr()?.into() },
            body: self.block()?,
            else_: self.parse_else()?,
        })
    }
    fn parse_else(&mut self) -> PResult<Option<control::Else>> {
        // it's fine for `else` to be missing entirely
        self.keyword(Keyword::Else)
            .ok()
            .map(|p| Ok(control::Else { block: p.block()? }))
            .transpose()
    }
    fn parse_break(&mut self) -> PResult<control::Break> {
        Ok(control::Break { expr: self.keyword(Keyword::Break)?.expr()?.into() })
    }
    fn parse_return(&mut self) -> PResult<control::Return> {
        Ok(control::Return { expr: self.keyword(Keyword::Return)?.expr()?.into() })
    }
    fn parse_continue(&mut self) -> PResult<control::Continue> {
        self.keyword(Keyword::Continue)?;
        Ok(control::Continue)
    }
}
