//! Parses [tokens](super::token) into an [AST](super::ast)
use super::{
    ast::preamble::*,
    lexer::Lexer,
    token::{Keyword, Token, Type},
};
use error::{Error, *};

mod error {
    use super::{Token, Type};

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub enum Reason {
        Expected(Type),
        NotIdentifier,
        NotLiteral,
        NotString,
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
    /// [Parser] [Result]
    pub type PResult<T> = Result<T, Error>;
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct Error {
        reason: Reason,
        start: Option<Token>,
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
    /// Consumes any consecutive comments
    fn consume_comments(&mut self) -> &mut Self {
        while let Some(Type::Comment) = self.peek().map(|t| t.ty()) {
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
    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.curr)
    }
    /// Look ahead `n` tokens
    pub fn ahead(&self, n: usize) -> Option<&Token> {
        self.tokens.get(self.curr.wrapping_add(n))
    }
    /// Look behind `n` tokens
    pub fn behind(&self, n: usize) -> Option<&Token> {
        self.tokens.get(self.curr.wrapping_sub(n))
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
    /// Parse the [start of an AST](Start)
    pub fn parse(&mut self) -> PResult<Start> {
        self.consume_comments();
        Ok(Start(self.expr()?))
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
        let out = mid(self);
        self.consume_type(rhs)?;
        out
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
fn check_eof(t: Option<&Token>) -> PResult<&Token> {
    t.ok_or(Error::end_of_file())
}

/// # Terminals and Pseudo-Terminals
impl<'t> Parser<'t> {
    pub fn identifier(&mut self) -> PResult<Identifier> {
        let range = self.matches(Type::Identifier)?.range();
        Ok(Identifier(self.consume().text[range].into()))
    }
    pub fn literal(&mut self) -> PResult<literal::Literal> {
        use literal::Literal::*;
        use Keyword::{False, True};
        let tok = check_eof(self.peek())?;
        match tok.ty() {
            Type::Float => self.float().map(Float),
            Type::Integer => self.int().map(Int),
            Type::String => self.string().map(String),
            Type::Character => self.char().map(Char),
            Type::Keyword(True | False) => self.bool().map(Bool),
            _ => Err(Error::not_literal().token(*tok)),
        }
    }
    pub fn float(&mut self) -> PResult<literal::Float> {
        ptodo!(self)
    }
    pub fn int(&mut self) -> PResult<u128> {
        #[cfg(debug_assertions)]
        eprintln!("/* TODO: parse integer literals from other bases */");
        let token = *self.matches(Type::Integer)?;
        self.consume().text[token.range()]
            .parse()
            .map_err(|_| Error::not_int().token(token))
    }
    pub fn string(&mut self) -> PResult<String> {
        let range = self.matches(Type::String)?.range();
        Ok(self.consume().text[range].into())
    }
    pub fn char(&mut self) -> PResult<char> {
        ptodo!(self)
    }
    pub fn bool(&mut self) -> PResult<bool> {
        use Keyword::{False, True};
        let token = check_eof(self.peek())?;
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
        self.flow()
            .map(Expr::Flow)
            .or_else(|_| self.ignore().map(Expr::Ignore))
    }
    pub fn block(&mut self) -> PResult<expression::Block> {
        self.delimited(Type::LCurly, Parser::expr, Type::RCurly)
            .map(|e| expression::Block { expr: Box::new(e) })
    }
    pub fn group(&mut self) -> PResult<expression::Group> {
        self.delimited(Type::LParen, Parser::expr, Type::RParen)
            .map(|e| expression::Group { expr: Box::new(e) })
    }
    pub fn r#final(&mut self) -> PResult<expression::Final> {
        use expression::Final;
        self.identifier()
            .map(Final::Identifier)
            .or_else(|_| self.literal().map(Final::Literal))
            .or_else(|_| self.block().map(Final::Block))
            .or_else(|_| self.group().map(Final::Group))
            .or_else(|_| self.branch().map(Final::Branch))
    }
}

/// Helper macro for math parsing subexpressions with production
/// ```ebnf
/// Ret = a (b a)*
/// ```
/// # Examples
/// ```rust,ignore
/// math_impl!{
///     function_name: ret::Value = parse_operands, parse_operators;
/// }
/// ```
/// becomes
/// ```rust,ignore
/// pub fn function_name(&mut self) -> PResult<ret::Value> { ... }
/// ```
macro math_impl ($($f: ident: $Ret:path = $a:ident, $b:ident);*$(;)?) {$(
    pub fn $f (&mut self) -> PResult<$Ret> {
        let (first, mut others) = (self.$a()?, vec![]);
        while let Some(op) = self.$b() {
            others.push((op, self.$a()?));
        }
        Ok($Ret(first, others))
    }
)*}
/// # [Arithmetic and Logical Subexpressions](math)
impl<'t> Parser<'t> {
    math_impl! {
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
        Ok(math::Unary(ops, self.r#final()?))
    }
}
macro operator_impl($($(#[$m:meta])*$f:ident: $Ret:ty),*$(,)*) {$(
    $(#[$m])* pub fn $f(&mut self) -> Option<$Ret> {
        let out: Option<$Ret> = self.peek()?.ty().into();
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
    pub fn branch(&mut self) -> PResult<control::Branch> {
        use control::Branch;
        use Keyword::{For, If, While};
        let token = check_eof(self.peek())?;
        match token.ty() {
            Type::Keyword(While) => self.parse_while().map(Branch::While),
            Type::Keyword(For) => self.parse_for().map(Branch::For),
            Type::Keyword(If) => self.parse_if().map(Branch::If),
            _ => Err(Error::not_branch().token(*token)),
        }
    }
    pub fn parse_if(&mut self) -> PResult<control::If> {
        self.consume_type(Type::Keyword(Keyword::If))?;
        Ok(control::If {
            cond: self.expr()?.into(),
            body: self.block()?,
            else_: self.parse_else()?,
        })
    }
    pub fn parse_while(&mut self) -> PResult<control::While> {
        self.consume_type(Type::Keyword(Keyword::While))?;
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
        match self.keyword(Keyword::Else) {
            Ok(_) => Ok(Some(control::Else { block: self.block()? })),
            Err(_) => Ok(None),
        }
    }
    pub fn flow(&mut self) -> PResult<control::Flow> {
        use control::Flow;
        use Keyword::{Break, Continue, Return};
        let token = check_eof(self.peek())?;
        match token.ty() {
            Type::Keyword(Break) => self.parse_break().map(Flow::Break),
            Type::Keyword(Return) => self.parse_return().map(Flow::Return),
            Type::Keyword(Continue) => self.parse_continue().map(Flow::Continue),
            _ => Err(Error::not_control_flow().token(*token)),
        }
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
