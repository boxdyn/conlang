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
        NotLiteral,
        NotString,
        NotChar,
        NotBool,
        NotFloat,
        FloatExponentOverflow,
        FloatMantissaOverflow,
        NotInt,
        IntOverflow,
        NotControlFlow,
        NotBranch,
        EndOfFile,
        #[default]
        Unspecified,
    }
    use Reason::*;

    impl Display for Reason {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Expected(t) => write!(f, "Expected {t}"),
                Self::NotIdentifier => Display::fmt("Not an identifier", f),
                Self::NotLiteral => Display::fmt("Not a literal", f),
                Self::NotString => Display::fmt("Not a string", f),
                Self::NotChar => Display::fmt("Not a char", f),
                Self::NotBool => Display::fmt("Not a bool", f),
                Self::NotFloat => Display::fmt("Not a float", f),
                Self::FloatExponentOverflow => Display::fmt("Float exponent too large", f),
                Self::FloatMantissaOverflow => Display::fmt("Float mantissa too large", f),
                Self::NotInt => Display::fmt("Not an integer", f),
                Self::IntOverflow => Display::fmt("Integer too large", f),
                Self::NotControlFlow => Display::fmt("Control flow expression was incomplete", f),
                Self::NotBranch => Display::fmt("Branch expression was incomplete", f),
                Self::EndOfFile => Display::fmt("Got end of file", f),
                Self::Unspecified => Display::fmt(
                    "Unspecified error. You are permitted to slap the code author.",
                    f,
                ),
            }
        }
    }

    /// [Parser] [Result]
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
            not_literal: NotLiteral,
            not_string: NotString,
            not_char: NotChar,
            not_bool: NotBool,
            not_float: NotFloat,
            float_exponent_overflow: FloatExponentOverflow,
            float_mantissa_overflow: FloatMantissaOverflow,
            not_int: NotInt,
            int_overflow: IntOverflow,
            not_control_flow: NotControlFlow,
            not_branch: NotBranch,
            end_of_file: EndOfFile,
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
        self.tokens.get(self.curr).ok_or(Error::end_of_file())
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
    pub fn unwind(&mut self) -> Option<usize> {
        let out = self.panic_stack.pop();
        if let Some(v) = out {
            self.curr = v;
        }
        out
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
            Err(Error::end_of_file())
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
        self.consume_type(lhs)?;
        let out = mid(self)?;
        self.consume_type(rhs)?;
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
    pub fn identifier(&mut self) -> PResult<Identifier> {
        let token = *self
            .matches(Type::Identifier)
            .map_err(|e| Error::not_identifier().maybe_token(e.start()))?;
        Ok(Identifier(self.consume().text[&token].into()))
    }
    pub fn literal(&mut self) -> PResult<literal::Literal> {
        use literal::Literal::*;
        use Keyword::{False, True};
        let tok = self.peek()?;
        match tok.ty() {
            Type::Float => self.float().map(Float),
            Type::Integer => self.int::<10>().map(Int),
            Type::String => self.string().map(String),
            Type::Character => self.char().map(Char),
            Type::Keyword(True | False) => self.bool().map(Bool),
            _ => Err(Error::not_literal().token(*tok)),
        }
    }
    pub fn float(&mut self) -> PResult<literal::Float> {
        ptodo!(self)
    }
    pub fn int<const BASE: u32>(&mut self) -> PResult<u128> {
        let token = *self.matches(Type::Integer)?;
        u128::from_str_radix(&self.consume().text[&token], BASE)
            .map_err(|_| Error::not_int().token(token))
    }
    pub fn string(&mut self) -> PResult<String> {
        let range = self
            .matches(Type::String)
            .map_err(|e| e.reason(NotString))?
            .range();
        Ok(self.consume().text[range].chars().unescape().collect())
    }
    pub fn char(&mut self) -> PResult<char> {
        let token = *self.matches(Type::Character)?;
        self.consume().text[&token]
            .chars()
            .unescape()
            .next()
            .ok_or(Error::not_char().token(token))
    }
    pub fn bool(&mut self) -> PResult<bool> {
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
    pub fn expr(&mut self) -> PResult<expression::Expr> {
        use expression::Expr;
        Ok(Expr { ignore: self.ignore()? })
    }
    pub fn if_not_expr(&mut self, matches: Type) -> PResult<Option<expression::Expr>> {
        if self.peek()?.ty() == matches {
            Ok(None)
        } else {
            Some(self.expr()).transpose()
        }
    }
    pub fn block(&mut self) -> PResult<expression::Block> {
        self.delimited(Type::LCurly, |p| p.if_not_expr(Type::RCurly), Type::RCurly)
            .map(|e| expression::Block { expr: e.map(Box::new) })
    }
    pub fn group(&mut self) -> PResult<expression::Group> {
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
    pub fn primary(&mut self) -> PResult<expression::Primary> {
        use expression::Primary;
        self.identifier()
            .map(Primary::Identifier)
            .or_else(|_| self.literal().map(Primary::Literal))
            .or_else(|_| self.block().map(Primary::Block))
            .or_else(|_| self.group().map(Primary::Group))
            .or_else(|_| self.flow().map(Primary::Branch))
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
/// pub fn function_name(&mut self) -> PResult<ret::Value> { ... }
/// ```
macro binary ($($f:ident: $Ret:ty = $a:ident, $b:ident);*$(;)?) {$(
    pub fn $f (&mut self) -> PResult<$Ret> {
        let (first, mut others) = (self.$a()?, vec![]);
        while let Some(op) = self.$b() {
            others.push((op, self.$a()?));
        }
        Ok(<$Ret>::new(first, others))
    }
)*}
/// # [Arithmetic and Logical Subexpressions](math)
impl<'t> Parser<'t> {
    binary! {
        //name   returns         operands operators
        ignore:  math::Ignore  = assign,  ignore_op;
        assign:  math::Assign  = compare, assign_op;
        compare: math::Compare = logic,   compare_op;
        logic:   math::Logic   = bitwise, logic_op;
        bitwise: math::Bitwise = shift,   bitwise_op;
        shift:   math::Shift   = term,    shift_op;
        term:    math::Term    = factor,  term_op;
        factor:  math::Factor  = unary,   factor_op;
    }
    pub fn unary(&mut self) -> PResult<math::Unary> {
        let mut ops = vec![];
        while let Some(op) = self.unary_op() {
            ops.push(op)
        }
        Ok(math::Unary(ops, self.primary()?))
    }
}
macro operator_impl($($(#[$m:meta])*$f:ident: $Ret:ty),*$(,)*) {$(
    $(#[$m])* pub fn $f(&mut self) -> Option<$Ret> {
        let out: Option<$Ret> = self.peek().ok()?.ty().into();
        if out.is_some() { self.consume(); }
        out
    }
)*}
/// # [Operators](operator)
impl<'t> Parser<'t> {
    operator_impl! {
        ignore_op:  operator::Ignore,
        compare_op: operator::Compare,
        assign_op:  operator::Assign,
        logic_op:   operator::Logic,
        bitwise_op: operator::Bitwise,
        shift_op:   operator::Shift,
        term_op:    operator::Term,
        factor_op:  operator::Factor,
        unary_op:   operator::Unary,
    }
}
/// # [Control Flow](control)
impl<'t> Parser<'t> {
    pub fn flow(&mut self) -> PResult<control::Flow> {
        use control::Flow;
        use Keyword::{Break, Continue, For, If, Return, While};
        let token = self.peek()?;
        match token.ty() {
            Type::Keyword(While) => self.parse_while().map(Flow::While),
            Type::Keyword(For) => self.parse_for().map(Flow::For),
            Type::Keyword(If) => self.parse_if().map(Flow::If),
            Type::Keyword(Break) => self.parse_break().map(Flow::Break),
            Type::Keyword(Return) => self.parse_return().map(Flow::Return),
            Type::Keyword(Continue) => self.parse_continue().map(Flow::Continue),
            _ => Err(Error::not_branch().token(*token)),
        }
    }
    pub fn parse_if(&mut self) -> PResult<control::If> {
        self.keyword(Keyword::If)?;
        Ok(control::If {
            cond: self.expr()?.into(),
            body: self.block()?,
            else_: self.parse_else()?,
        })
    }
    pub fn parse_while(&mut self) -> PResult<control::While> {
        self.keyword(Keyword::While)?;
        Ok(control::While {
            cond: self.expr()?.into(),
            body: self.block()?,
            else_: self.parse_else()?,
        })
    }
    pub fn parse_for(&mut self) -> PResult<control::For> {
        self.keyword(Keyword::For)?;
        Ok(control::For {
            var: self.identifier()?,
            iter: { self.keyword(Keyword::In)?.expr()?.into() },
            body: self.block()?,
            else_: self.parse_else()?,
        })
    }
    pub fn parse_else(&mut self) -> PResult<Option<control::Else>> {
        // it's fine for `else` to be missing entirely
        self.keyword(Keyword::Else)
            .ok()
            .map(|p| Ok(control::Else { block: p.block()? }))
            .transpose()
    }
    pub fn parse_break(&mut self) -> PResult<control::Break> {
        Ok(control::Break { expr: self.keyword(Keyword::Break)?.expr()?.into() })
    }
    pub fn parse_return(&mut self) -> PResult<control::Return> {
        Ok(control::Return { expr: self.keyword(Keyword::Return)?.expr()?.into() })
    }
    pub fn parse_continue(&mut self) -> PResult<control::Continue> {
        ptodo!(self)
    }
}
