//! Parses [tokens](super::token) into an [AST](super::ast)

use super::{ast::preamble::*, lexer::Lexer, token::preamble::*};
use error::{Error, Reason::*, *};

pub mod error {
    use super::{Token, Type};
    use std::fmt::Display;

    /// The reason for the [Error]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub enum Reason {
        Expected(Type),
        Unexpected(Type),
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
                Self::Unexpected(t) => write!(f, "Unexpected {t} in bagging area"),
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
    /// An error produced by the [Parser](super::Parser).
    ///
    /// Contains a [Reason], and, optionally, a start [Token]
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct Error {
        reason: Reason,
        start: Option<Token>,
    }
    impl std::error::Error for Error {}
    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if let Some(token) = &self.start {
                write!(f, "{}:{}: ", token.line(), token.col())?;
            }
            write!(f, "{}", self.reason)
        }
    }

    macro error_impl($($fn:ident$(($($p:ident: $t:ty),*))?: $reason:expr),*$(,)?) {$(
        /// Creates an [Error] with this [Reason]:
        #[doc = concat!("[`", stringify!($reason), "`]")]
        #[allow(dead_code)]
        pub(crate) fn $fn($($($p : $t),*)?) -> Self {
            Self { reason: $reason$(($($p)*))?, start: None }
        }
    )*}
    impl Error {
        /// Provides an optional start [Token]
        pub fn token(self, start: Token) -> Self {
            Self { start: Some(start), ..self }
        }
        /// Optionally sets the start [Token]
        pub fn maybe_token(self, start: Option<Token>) -> Self {
            Self { start, ..self }
        }
        /// Gets a reference to the start [Token], if there is one
        pub fn start(&self) -> Option<&Token> {
            self.start.as_ref()
        }
        /// Gets the [Reason] for this error
        pub fn reason(&self) -> Reason {
            self.reason
        }
        /// Modifies the [Reason] of this error
        pub fn with_reason(self, reason: Reason) -> Self {
            Self { reason, ..self }
        }
        error_impl! {
            expected(e: Type): Expected,
            unexpected(e: Type): Unexpected,
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
            end_of_file: EndOfFile,
            panic_underflow: PanicStackUnderflow,
            unspecified: Unspecified,
        }
    }
}

/// The Parser performs recursive descent on the AST's grammar
/// using a provided [Lexer].
pub struct Parser {
    tokens: Vec<Token>,
    panic_stack: Vec<usize>,
    pub errors: Vec<Error>,
    cursor: usize,
}
impl<'t> From<Lexer<'t>> for Parser {
    fn from(value: Lexer<'t>) -> Self {
        let mut tokens = vec![];
        for result in value {
            match result {
                Ok(t) => tokens.push(t),
                Err(e) => println!("{e}"),
            }
        }
        Self::new(tokens)
    }
}

impl Parser {
    /// Create a new [Parser] from a list of [Tokens][1]
    /// and the [text](str) used to generate that list
    /// (as [Tokens][1] do not store their strings)
    ///
    /// [1]: Token
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, panic_stack: vec![], errors: vec![], cursor: 0 }
    }
    /// Parses the [start of an AST](Start)
    pub fn parse(&mut self) -> PResult<Start> {
        self.consume_comments();
        Ok(Start(self.program()?))
    }
    /// Parses only one expression
    pub fn parse_expr(&mut self) -> PResult<expression::Expr> {
        self.expr()
    }
    /// Peeks at the current token
    pub fn peek(&self) -> PResult<&Token> {
        self.tokens
            .get(self.cursor)
            .ok_or(Error::end_of_file().maybe_token(self.tokens.last().cloned()))
    }
    /// Consumes any number of consecutive comments
    fn consume_comments(&mut self) -> &mut Self {
        while let Ok(Type::Comment) = self.peek().map(|t| t.ty()) {
            self.cursor += 1;
        }
        self
    }
    /// Consumes the current token
    #[inline]
    fn consume(&mut self) -> &mut Self {
        self.cursor += 1;
        self.consume_comments();
        self
    }
}
/// Panicking
#[allow(dead_code)]
impl Parser {
    /// Records the current position on the panic stack
    fn mark(&mut self) -> &mut Self {
        self.panic_stack.push(self.cursor);
        self
    }
    /// Erases a recorded position from the panic stack
    fn unmark(&mut self) -> &mut Self {
        self.panic_stack.pop();
        self
    }
    /// Unwinds the panic stack one step
    fn unwind(&mut self) -> PResult<&mut Self> {
        let v = self.panic_stack.pop().ok_or(Error::panic_underflow())?;
        self.cursor = v;
        Ok(self)
    }
    /// Advances forward until a token with type [`t`](Type) is encountered
    fn advance_until(&mut self, t: Type) -> PResult<&mut Self> {
        while self.matches(t).is_err() {
            self.check_eof()
                .map_err(|e| e.with_reason(Expected(t)))?
                .consume();
        }
        Ok(self)
    }
    /// Marks the current position, and unwinds the panic stack if `f` fails.
    fn attempt<F, R>(&mut self, f: F) -> PResult<R>
    where F: FnOnce(&mut Self) -> PResult<R> {
        self.mark();
        let out = f(self);
        match out {
            Ok(_) => self.unmark(),
            Err(_) => self.unwind()?,
        };
        out
    }
}
/// Helpers
impl Parser {
    /// Returns an error if the end of input has been reached
    fn check_eof(&mut self) -> PResult<&mut Self> {
        if self.cursor < self.tokens.len() {
            Ok(self)
        } else {
            Err(Error::end_of_file().maybe_token(self.tokens.last().cloned()))
        }
    }
    /// Peeks at the next token if it has the expected [Type]
    fn matches(&mut self, t: Type) -> PResult<&Token> {
        let token = self.check_eof()?.peek().expect("self should not be eof");
        if token.ty() != t {
            Err(Error::expected(t).token(token.clone()))?
        }
        Ok(token)
    }
    /// Consumes, without returning, a token with the given [Keyword], or returns an error.
    ///
    /// Useful if you only want to check the existence of a [Keyword]
    fn keyword(&mut self, keyword: Keyword) -> PResult<&mut Self> {
        self.consume_type(Type::Keyword(keyword))
    }
    /// Consumes, without returning, a token with the given [Type], or returns an error.
    ///
    /// Useful if you only want to check the existence of a token.
    fn consume_type(&mut self, t: Type) -> PResult<&mut Self> {
        self.matches(t)?;
        Ok(self.consume())
    }
    #[doc(hidden)]
    fn todo_error(&mut self, l: u32, c: u32, s: &str) -> Error {
        eprintln!("TODO: {s}:{l}:{c}");
        Error::unspecified().token(self.peek().unwrap().clone())
    }
}
/// TODO: Remove `ptodo*`
macro ptodo_err($self:expr $(, $t:expr)*) {
    $($t;)*
    $self.todo_error(line!(), column!(), file!())
}
macro ptodo($self:expr $(, $t:expr)*) {
    $($t;)*
    Err(ptodo_err!($self))
}

/// # Terminals and Pseudo-Terminals
impl Parser {
    /// Parses an [Identifier]
    fn identifier(&mut self) -> PResult<Identifier> {
        let out = match self.matches(Type::Identifier)?.data() {
            Data::Identifier(id) => Identifier(id.to_string()),
            _ => Err(Error::not_identifier())?,
        };
        self.consume();
        Ok(out)
    }
    /// Parses a [Literal](literal::Literal)
    fn literal(&mut self) -> PResult<literal::Literal> {
        use literal::Literal::*;
        use Keyword::{False, True};
        let token = self.peek()?;
        match token.ty() {
            Type::Float => self.float().map(Float),
            Type::Integer => self.int().map(Int),
            Type::String => self.string().map(String),
            Type::Character => self.char().map(Char),
            Type::Keyword(True | False) => self.bool().map(Bool),
            _ => Err(Error::not_literal().token(token.clone())),
        }
    }
    /// Parses a [floating point literal](literal::Float)
    fn float(&mut self) -> PResult<literal::Float> {
        ptodo!(self)
    }
    /// Parses an [integer literal](u128)
    ///
    /// u128 was chosen for this, since it stores the largest integer precision Rust natively
    /// supports. Conlang doesn't currently plan to support arbitrary-width arithmetic anyway.
    fn int(&mut self) -> PResult<u128> {
        let out = match self.matches(Type::Integer)?.data() {
            Data::Integer(i) => *i,
            _ => Err(Error::not_int())?,
        };
        self.consume();
        Ok(out)
    }
    /// Parses a [string literal](String)
    fn string(&mut self) -> PResult<String> {
        let out = match self.matches(Type::String)?.data() {
            Data::String(s) => s.clone(),
            _ => Err(Error::not_string())?,
        };
        self.consume();
        Ok(out)
    }
    /// Parses a [character literal](char)
    fn char(&mut self) -> PResult<char> {
        let out = match self.matches(Type::Character)?.data() {
            Data::Character(c) => *c,
            _ => Err(Error::not_char())?,
        };
        self.consume();
        Ok(out)
    }
    /// Parses a [boolean literal](bool)
    fn bool(&mut self) -> PResult<bool> {
        use Keyword::{False, True};
        let token = self.peek()?;
        let out = match token.ty() {
            Type::Keyword(False) => false,
            Type::Keyword(True) => true,
            _ => Err(Error::not_bool().token(token.clone()))?,
        };
        self.consume();
        Ok(out)
    }
}
/// Statements
impl Parser {
    /// Parses a series of [statements](Stmt)
    fn program(&mut self) -> PResult<Program> {
        let mut out = vec![];
        while self.check_eof().is_ok() {
            out.push(self.stmt()?);
        }
        Ok(Program(out))
    }
    /// Parses a single [statement](Stmt)
    fn stmt(&mut self) -> PResult<Stmt> {
        let token = self.peek()?;
        match token.ty() {
            Type::Keyword(Keyword::Let) => self.let_stmt().map(Stmt::Let),
            _ => {
                let out = Stmt::Expr(self.expr()?);
                self.consume_type(Type::Semi)?;
                Ok(out)
            }
        }
    }
    /// Parses a [Let] statement
    fn let_stmt(&mut self) -> PResult<Let> {
        let out = Let {
            mutable: self.consume().keyword(Keyword::Mut).is_ok(),
            name: self.identifier()?,
            ty: self
                .consume_type(Type::Colon)
                .and_then(Self::identifier)
                .ok(),
            init: self.consume_type(Type::Eq).and_then(Self::expr).ok(),
        };
        self.consume_type(Type::Semi)?;
        Ok(out)
    }
    // /// Parses a [Function] statement
    // fn function_stmt(&mut self) -> PResult<Function> {
    // }
}
/// Expressions
impl Parser {
    /// Parses an [expression](expression::Expr)
    fn expr(&mut self) -> PResult<expression::Expr> {
        use expression::Expr;
        Ok(Expr(self.assign()?))
    }
    /// Parses a [block expression](expression::Block)
    fn block(&mut self) -> PResult<expression::Block> {
        use expression::{Block, Expr};
        let mut statements = vec![];
        let mut expr: Option<Box<Expr>> = None;
        self.consume_type(Type::LCurly)?;
        // tHeRe Is No PlAcE iN yOuR gRaMmAr WhErE bOtH aN eXpReSsIoN aNd A sTaTeMeNt ArE eXpEcTeD
        while self.consume_type(Type::RCurly).is_err() {
            match self.expr() {
                Ok(e) if self.consume_type(Type::Semi).is_ok() => statements.push(Stmt::Expr(e)),
                Ok(e) => {
                    expr = Some(Box::new(e));
                    self.consume_type(Type::RCurly)?;
                    break;
                }
                Err(_) => statements.push(self.stmt()?),
            }
        }
        Ok(Block { statements, expr })
    }
    /// Parses a [group expression](expression::Group)
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
    /// Parses a [primary expression](expression::Primary)
    fn primary(&mut self) -> PResult<expression::Primary> {
        use expression::Primary;
        let token = self.peek()?;
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
            e => Err(Error::unexpected(e).token(token.clone()))?,
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
    #[doc = concat!("Parses a(n) [", stringify!($f), " operation](math::Operation::Binary) expression")]
    fn $f (&mut self) -> PResult<math::Operation> {
        use math::{Operation, Binary};
        let (first, mut other) = (self.$a()?, vec![]);
        while let Ok(op) = self.$b() {
            other.push((op, self.$a()?));
        }
        Ok(if other.is_empty() { first } else {
            Operation::Binary(Binary { first: first.into(), other })
        })
    }
)*}
/// # [Arithmetic and Logical Subexpressions](math)
impl Parser {
    fn assign(&mut self) -> PResult<math::Operation> {
        use math::{Assign, Operation};
        let next = self.compare()?;
        let Ok(operator) = self.assign_op() else {
            return Ok(next);
        };
        let Operation::Primary(expression::Primary::Identifier(target)) = next else {
            return Ok(next);
        };
        Ok(Operation::Assign(Assign {
            target,
            operator,
            init: self.assign()?.into(),
        }))
    }
    binary! {
        // name   operands operators
        compare = range,   compare_op;
        range   = logic,   range_op;
        logic   = bitwise, logic_op;
        bitwise = shift,   bitwise_op;
        shift   = term,    shift_op;
        term    = factor,  term_op;
        factor  = unary,   factor_op;
    }
    /// Parses a [unary operation](math::Operation::Unary) expression
    fn unary(&mut self) -> PResult<math::Operation> {
        use math::{Operation, Unary};
        let mut operators = vec![];
        while let Ok(op) = self.unary_op() {
            operators.push(op)
        }
        if operators.is_empty() {
            return self.primary_operation();
        }
        Ok(Operation::Unary(Unary {
            operators,
            operand: self.primary_operation()?.into(),
        }))
    }
    /// Parses a [primary operation](math::Operation::Primary) expression
    fn primary_operation(&mut self) -> PResult<math::Operation> {
        Ok(math::Operation::Primary(self.primary()?))
    }
}
macro operator_impl ($($(#[$m:meta])* $f:ident : {$($type:pat => $op:ident),*$(,)?})*) {
    $($(#[$m])* fn $f(&mut self) -> PResult<operator::Binary> {
        use operator::Binary;
        let token = self.peek()?;
        let out = Ok(match token.ty() {
            $($type => Binary::$op,)*
            _ => Err(Error::not_operator().token(token.clone()))?,
        });
        self.consume();
        out
    })*
}
/// # [Operators](operator)
impl Parser {
    operator_impl! {
        /// Parses a [factor operator](operator)
        factor_op: {
            Type::Star => Mul,
            Type::Slash => Div,
            Type::Rem => Rem,
        }
        /// Parses a [term operator](operator)
        term_op: {
            Type::Plus => Add,
            Type::Minus => Sub,
        }
        /// Parses a [shift operator](operator)
        shift_op: {
            Type::LtLt => Lsh,
            Type::GtGt => Rsh,
        }
        /// Parses a [bitwise operator](operator)
        bitwise_op: {
            Type::Amp => BitAnd,
            Type::Bar => BitOr,
            Type::Xor => BitXor,
        }
        /// Parses a [logic operator](operator)
        logic_op: {
            Type::AmpAmp => LogAnd,
            Type::BarBar => LogOr,
            Type::XorXor => LogXor,
        }
        /// Parses a [range operator](operator)
        range_op: {
            Type::DotDot => RangeExc,
            Type::DotDotEq => RangeInc,
        }
        /// Parses a [compare operator](operator)
        compare_op: {
            Type::Lt => Less,
            Type::LtEq => LessEq,
            Type::EqEq => Equal,
            Type::BangEq => NotEq,
            Type::GtEq => GreaterEq,
            Type::Gt => Greater,
        }
    }
    /// Parses an [assign operator](operator::Assign)
    fn assign_op(&mut self) -> PResult<operator::Assign> {
        use operator::Assign;
        let token = self.peek()?;
        let out = Ok(match token.ty() {
            Type::Eq => Assign::Assign,
            Type::PlusEq => Assign::AddAssign,
            Type::MinusEq => Assign::SubAssign,
            Type::StarEq => Assign::MulAssign,
            Type::SlashEq => Assign::DivAssign,
            Type::RemEq => Assign::RemAssign,
            Type::AmpEq => Assign::BitAndAssign,
            Type::BarEq => Assign::BitOrAssign,
            Type::XorEq => Assign::BitXorAssign,
            Type::LtLtEq => Assign::ShlAssign,
            Type::GtGtEq => Assign::ShrAssign,
            _ => Err(Error::not_operator().token(token.clone()))?,
        });
        self.consume();
        out
    }
    /// Parses a [unary operator](operator::Unary)
    fn unary_op(&mut self) -> PResult<operator::Unary> {
        use operator::Unary;
        let token = self.peek()?;
        let out = Ok(match token.ty() {
            Type::AmpAmp => Unary::RefRef,
            Type::Amp => Unary::Ref,
            Type::Star => Unary::Deref,
            Type::Minus => Unary::Neg,
            Type::Bang => Unary::Not,
            Type::At => Unary::At,
            Type::Hash => Unary::Hash,
            Type::Tilde => Unary::Tilde,
            _ => Err(Error::not_operator().token(token.clone()))?,
        });
        self.consume();
        out
    }
}
/// # [Control Flow](control)
impl Parser {
    /// Parses a [control flow](control::Flow) expression
    fn flow(&mut self) -> PResult<control::Flow> {
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
            e => Err(Error::unexpected(e).token(token.clone()))?,
        }
        .map_err(|e| e.with_reason(IncompleteBranch))
    }
    /// Parses an [if](control::If) expression
    fn parse_if(&mut self) -> PResult<control::If> {
        self.keyword(Keyword::If)?;
        Ok(control::If {
            cond: self.expr()?.into(),
            body: self.block()?,
            else_: self.parse_else()?,
        })
    }
    /// Parses a [while](control::While) expression
    fn parse_while(&mut self) -> PResult<control::While> {
        self.keyword(Keyword::While)?;
        Ok(control::While {
            cond: self.expr()?.into(),
            body: self.block()?,
            else_: self.parse_else()?,
        })
    }
    /// Parses a [for](control::For) expression
    fn parse_for(&mut self) -> PResult<control::For> {
        self.keyword(Keyword::For)?;
        Ok(control::For {
            var: self.identifier()?,
            iter: { self.keyword(Keyword::In)?.expr()?.into() },
            body: self.block()?,
            else_: self.parse_else()?,
        })
    }
    /// Parses an [else](control::Else) sub-expression
    fn parse_else(&mut self) -> PResult<Option<control::Else>> {
        // it's fine for `else` to be missing entirely
        self.keyword(Keyword::Else)
            .ok()
            .map(|p| Ok(control::Else { block: p.block()? }))
            .transpose()
    }
    /// Parses a [break](control::Break) expression
    fn parse_break(&mut self) -> PResult<control::Break> {
        Ok(control::Break { expr: self.keyword(Keyword::Break)?.expr()?.into() })
    }
    /// Parses a [return](control::Return) expression
    fn parse_return(&mut self) -> PResult<control::Return> {
        Ok(control::Return { expr: self.keyword(Keyword::Return)?.expr()?.into() })
    }
    /// Parses a [continue](control::Continue) expression
    fn parse_continue(&mut self) -> PResult<control::Continue> {
        self.keyword(Keyword::Continue)?;
        Ok(control::Continue)
    }
}
