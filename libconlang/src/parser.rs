//! Parses [tokens](super::token) into an [AST](super::ast)
#![deprecated]
#![allow(deprecated)]
use super::{ast::preamble::*, lexer::Lexer, token::preamble::*};
use error::{Error, *};

pub mod error {
    use super::{Token, Type};
    use std::fmt::Display;

    pub trait WrapError {
        /// Wraps this error in a parent [Error]
        fn wrap(self, parent: Error) -> Self;
    }
    impl WrapError for Error {
        fn wrap(self, parent: Error) -> Self {
            Self { child: Some(self.into()), ..parent }
        }
    }
    impl<T> WrapError for Result<T, Error> {
        fn wrap(self, parent: Error) -> Self {
            self.map_err(|e| e.wrap(parent))
        }
    }

    /// The reason for the [Error]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub enum Reason {
        Expected(Type),
        Unexpected(Type),
        NotPathSegment(Type),
        NotIdentifier,
        NotStatement,
        NotLet,
        NotFnDecl,
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
                Self::NotPathSegment(t) => write!(f, "{t} not a path segment"),
                Self::NotIdentifier => "Not an identifier".fmt(f),
                Self::NotStatement => "Not a statement".fmt(f),
                Self::NotLet => "Not a let statement".fmt(f),
                Self::NotFnDecl => "Not a valid function declaration".fmt(f),
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
        child: Option<Box<Self>>,
        start: Option<Token>,
    }
    impl std::error::Error for Error {}
    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if let Some(child) = &self.child {
                write!(f, "{child}: ")?;
            }
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
            Self { reason: $reason$(($($p)*))?, child: None, start: None }
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
        error_impl! {
            expected(e: Type): Expected,
            unexpected(e: Type): Unexpected,
            not_path_segment(e: Type): NotPathSegment,
            not_identifier: NotIdentifier,
            not_statement: NotStatement,
            not_let: NotLet,
            not_fn_decl: NotFnDecl,
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
    /// Resets the parser, so it can be reused
    pub fn reset(&mut self) -> &mut Self {
        *self = Self::new(std::mem::take(&mut self.tokens));
        self
    }
    /// Parses the [start of an AST](Start)
    pub fn parse(&mut self) -> PResult<Start> {
        self.consume_comments();
        Ok(Start(self.program()?))
    }
    /// Parses only one expression
    pub fn parse_expr(&mut self) -> PResult<Expr> {
        self.expr()
    }
    /// Peeks at the current token
    pub fn peek(&self) -> PResult<&Token> {
        self.tokens
            .get(self.cursor)
            .ok_or_else(|| Error::end_of_file().maybe_token(self.tokens.last().cloned()))
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
            self.check_eof().wrap(Error::expected(t))?.consume();
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
            Err(Error::expected(t).token(token.clone()))
        } else {
            Ok(token)
        }
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
            Data::Identifier(id) => Identifier { name: id.to_string(), index: None },
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
            Type::Keyword(Keyword::Let) => self.let_stmt().map(Stmt::Let).wrap(Error::not_let()),
            Type::Keyword(Keyword::Fn) => self.fn_decl().map(Stmt::Fn).wrap(Error::not_fn_decl()),
            _ => {
                let out = Stmt::Expr(self.expr()?);
                self.consume_type(Type::Semi)?;
                Ok(out)
            }
        }
        .wrap(Error::not_statement())
    }
    /// Parses a [Let] statement
    fn let_stmt(&mut self) -> PResult<Let> {
        self.keyword(Keyword::Let)?;
        let out =
            Let { name: self.name()?, init: self.consume_type(Type::Eq).and_then(Self::expr).ok() };
        self.consume_type(Type::Semi)?;
        Ok(out)
    }
    /// Parses a [function declaration](FnDecl) statement
    fn fn_decl(&mut self) -> PResult<FnDecl> {
        self.keyword(Keyword::Fn)?;
        let name = self.identifier()?;
        self.consume_type(Type::LParen)?;
        let args = self.params()?;
        self.consume_type(Type::RParen)?;
        // TODO: Parse type-expressions and store return types in the AST
        let ty = if self.consume_type(Type::Arrow).is_ok() {
            Some(self.type_expr()?)
        } else {
            None
        };
        Ok(FnDecl { name: Name { symbol: name, mutable: false, ty }, args, body: self.block()? })
    }
    /// Parses a [parameter](Name) list for [FnDecl]
    fn params(&mut self) -> PResult<Vec<Name>> {
        let mut args = vec![];
        while let Ok(name) = self.name() {
            args.push(name);
            if self.consume_type(Type::Comma).is_err() {
                break;
            }
        }
        Ok(args)
    }
    /// Parses a [Name]; the object of a let statement, or a single function parameter.
    fn name(&mut self) -> PResult<Name> {
        Ok(Name {
            mutable: self.keyword(Keyword::Mut).is_ok(),
            symbol: self.identifier()?,
            ty: self
                .consume_type(Type::Colon)
                .and_then(|this| this.type_expr())
                .ok(),
        })
    }
}
/// Path Expressions
impl Parser {
    fn path(&mut self) -> PResult<path::Path> {
        let absolute = self.consume_type(Type::ColonColon).is_ok();
        let mut parts = vec![];
        while let Ok(id) = self.path_part() {
            parts.push(id);
            if self.consume_type(Type::ColonColon).is_err() {
                break;
            }
        }
        Ok(Path { absolute, parts })
    }

    fn path_part(&mut self) -> PResult<PathPart> {
        match self.peek()?.ty() {
            Type::Identifier => self.identifier().map(PathPart::PathIdent),
            Type::Keyword(Keyword::Super) => {
                self.keyword(Keyword::Super).map(|_| PathPart::PathSuper)
            }
            Type::Keyword(Keyword::SelfKw) => {
                self.keyword(Keyword::SelfKw).map(|_| PathPart::PathSelf)
            }
            e => Err(Error::not_path_segment(e)),
        }
    }
}
/// Type Expressions
impl Parser {
    /// Parses a [Type Expression](TypeExpr)
    fn type_expr(&mut self) -> PResult<TypeExpr> {
        match self.peek()?.ty() {
            Type::LParen => self.type_tuple().map(TypeExpr::TupleType),
            Type::Bang => self.type_never().map(TypeExpr::Never),
            _ => self.path().map(TypeExpr::TypePath),
        }
    }
    fn type_tuple(&mut self) -> PResult<TupleType> {
        self.consume_type(Type::LParen)?;
        let mut types = vec![];
        while let Ok(ty) = self.type_expr() {
            types.push(ty);
            if self.consume_type(Type::Comma).is_err() {
                break;
            }
        }
        self.consume_type(Type::RParen)?;
        Ok(TupleType { types })
    }
    fn type_never(&mut self) -> PResult<Never> {
        self.consume_type(Type::Bang).map(|_| Never)
    }
}

/// Expressions
impl Parser {
    /// Parses an [expression](Expr)
    fn expr(&mut self) -> PResult<Expr> {
        Ok(Expr(self.assign()?))
    }
    /// Parses a [block expression](Block)
    fn block(&mut self) -> PResult<Block> {
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
        Ok(Block { statements, expr, let_count: None })
    }
    /// Parses a [primary expression](Primary)
    fn primary(&mut self) -> PResult<Primary> {
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
/// [Call] expressions
impl Parser {
    /// Parses a [call expression](Call)
    fn call(&mut self) -> PResult<Call> {
        let callee = self.primary()?;
        if self.matches(Type::LParen).is_err() {
            return Ok(Call::Primary(callee));
        };
        let mut args = vec![];
        while self.consume_type(Type::LParen).is_ok() {
            match self.consume_type(Type::RParen) {
                Ok(_) => args.push(Tuple { elements: vec![] }),
                Err(_) => {
                    args.push(self.tuple()?);
                    self.consume_type(Type::RParen)?;
                }
            }
        }
        Ok(Call::FnCall(FnCall { callee: callee.into(), args }))
    }
}

/// Groups and Tuples
impl Parser {
    /// Parses a [group expression](Group)
    fn group(&mut self) -> PResult<Group> {
        let t = self.consume_type(Type::LParen)?.peek()?;
        match t.ty() {
            Type::RParen => {
                self.consume();
                Ok(Group::Empty)
            }
            _ => {
                let mut out = self.tuple()?;
                let out = if out.elements.len() == 1 {
                    Group::Single(out.elements.remove(0).into())
                } else {
                    Group::Tuple(out)
                };
                self.consume_type(Type::RParen)?;
                Ok(out)
            }
        }
    }
    /// Parses a [tuple expression](Tuple)
    fn tuple(&mut self) -> PResult<Tuple> {
        let mut elements = vec![self.expr()?];
        while self.consume_type(Type::Comma).is_ok() {
            elements.push(self.expr()?);
        }
        Ok(Tuple { elements })
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
/// fn function_name(&mut self) -> PResult<ret::Value> {  ... }
/// ```
macro binary ($($f:ident = $a:ident, $b:ident);*$(;)?) {$(
    #[doc = concat!("Parses a(n) [", stringify!($f), " operation](Operation::Binary) expression")]
    fn $f (&mut self) -> PResult<Operation> {
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
    fn assign(&mut self) -> PResult<Operation> {
        let next = self.compare()?;
        let Ok(operator) = self.assign_op() else {
            return Ok(next);
        };
        let Operation::Call(Call::Primary(Primary::Identifier(target))) = next else {
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
    /// Parses a [unary operation](Operation::Unary) expression
    fn unary(&mut self) -> PResult<Operation> {
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
    /// Parses a [primary operation](Operation::Primary) expression
    fn primary_operation(&mut self) -> PResult<Operation> {
        Ok(Operation::Call(self.call()?))
    }
}
macro operator_impl ($($(#[$m:meta])* $f:ident : {$($type:pat => $op:ident),*$(,)?})*) {
    $($(#[$m])* fn $f(&mut self) -> PResult<operator::Binary> {
        use operator::Binary;
        let token = self.peek().wrap(Error::not_operator())?;
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
    /// Parses a [control flow](Flow) expression
    fn flow(&mut self) -> PResult<Flow> {
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
        .wrap(Error::not_branch())
    }
    /// Parses an [if](If) expression
    fn parse_if(&mut self) -> PResult<If> {
        self.keyword(Keyword::If)?;
        Ok(If { cond: self.expr()?.into(), body: self.block()?, else_: self.parse_else()? })
    }
    /// Parses a [while](While) expression
    fn parse_while(&mut self) -> PResult<While> {
        self.keyword(Keyword::While)?;
        Ok(While { cond: self.expr()?.into(), body: self.block()?, else_: self.parse_else()? })
    }
    /// Parses a [for](For) expression
    fn parse_for(&mut self) -> PResult<For> {
        self.keyword(Keyword::For)?;
        Ok(For {
            var: self.identifier()?,
            iter: { self.keyword(Keyword::In)?.expr()?.into() },
            body: self.block()?,
            else_: self.parse_else()?,
        })
    }
    /// Parses an [else](Else) sub-expression
    fn parse_else(&mut self) -> PResult<Option<Else>> {
        // it's fine for `else` to be missing entirely
        self.keyword(Keyword::Else)
            .ok()
            .map(|p| Ok(Else { expr: p.expr()?.into() }))
            .transpose()
    }
    /// Parses a [break](Break) expression
    fn parse_break(&mut self) -> PResult<Break> {
        Ok(Break { expr: self.keyword(Keyword::Break)?.expr()?.into() })
    }
    /// Parses a [return](Return) expression
    fn parse_return(&mut self) -> PResult<Return> {
        Ok(Return { expr: self.keyword(Keyword::Return)?.expr()?.into() })
    }
    /// Parses a [continue](Continue) expression
    fn parse_continue(&mut self) -> PResult<Continue> {
        self.keyword(Keyword::Continue)?;
        Ok(Continue)
    }
}
