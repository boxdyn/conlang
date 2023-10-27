//! Interprets an AST as a program

use crate::ast::preamble::*;
use error::{Error, IResult, Reason};
use temp_type_impl::ConValue;

pub mod temp_type_impl {
    //! Temporary implementations of Conlang values
    use super::error::{Error, IResult, Reason};
    use std::ops::*;
    /// A Conlang value
    ///
    /// This is a hack to work around the fact that Conlang doesn't have a functioning type system
    /// yet :(
    #[derive(Clone, Debug)]
    pub enum ConValue {
        /// The empty/unit `()` type
        Empty,
        /// An integer
        Int(i128),
        /// A boolean
        Bool(bool),
        /// A unicode character
        Char(char),
        /// A string
        String(String),
        /// An exclusive range
        RangeExc(i128, i128),
        /// An inclusive range
        RangeInc(i128, i128),
    }
    impl ConValue {
        /// Gets whether the current value is true or false
        pub fn truthy(&self) -> IResult<bool> {
            match self {
                ConValue::Bool(v) => Ok(*v),
                _ => Err(Error::with_reason(Reason::TypeError))?,
            }
        }
        pub fn range_exc(self, other: Self) -> IResult<Self> {
            let (Self::Int(a), Self::Int(b)) = (self, other) else {
                Err(Error::with_reason(Reason::TypeError))?
            };
            Ok(Self::RangeExc(a, b.saturating_sub(1)))
        }
        pub fn range_inc(self, other: Self) -> IResult<Self> {
            let (Self::Int(a), Self::Int(b)) = (self, other) else {
                Err(Error::with_reason(Reason::TypeError))?
            };
            Ok(Self::RangeInc(a, b))
        }
        cmp! {
            lt: false, <;
            lt_eq: true, <=;
            eq: true, ==;
            neq: false, !=;
            gt_eq: true, >=;
            gt: false, >;
        }
    }
    /// Templates comparison functions for [ConValue]
    macro cmp ($($fn:ident: $empty:literal, $op:tt);*$(;)?) {$(
        /// TODO: Remove when functions are implemented:
        ///       Desugar into function calls
        pub fn $fn(&self, other: &Self) -> IResult<Self> {
            match (self, other) {
                (Self::Empty, Self::Empty) => Ok(Self::Bool($empty)),
                (Self::Int(a), Self::Int(b)) => Ok(Self::Bool(a $op b)),
                (Self::Bool(a), Self::Bool(b)) => Ok(Self::Bool(a $op b)),
                (Self::Char(a), Self::Char(b)) => Ok(Self::Bool(a $op b)),
                (Self::String(a), Self::String(b)) => Ok(Self::Bool(a $op b)),
                _ => Err(Error::with_reason(Reason::TypeError))
            }
        }
    )*}
    /// Implements [From] for an enum with 1-tuple variants
    macro from ($($T:ty => $v:expr),*$(,)?) {
        $(impl From<$T> for ConValue {
            fn from(value: $T) -> Self { $v(value.into()) }
        })*
    }
    from! {
        i128 => ConValue::Int,
        bool => ConValue::Bool,
        char => ConValue::Char,
        &str => ConValue::String,
        String => ConValue::String,
    }
    impl From<()> for ConValue {
        fn from(_: ()) -> Self {
            Self::Empty
        }
    }

    /// Implements binary [std::ops] traits for [ConValue]
    ///
    /// TODO: Desugar operators into function calls
    macro ops($($trait:ty: $fn:ident = [$($match:tt)*])*) {
        $(impl $trait for ConValue {
            type Output = IResult<Self>;
            /// TODO: Desugar operators into function calls
            fn $fn(self, rhs: Self) -> Self::Output {Ok(match (self, rhs) {$($match)*})}
        })*
    }
    ops! {
        Add: add = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a + b),
            (ConValue::String(a), ConValue::String(b)) => ConValue::String(a + &b),
            _ => Err(Error::with_reason(Reason::TypeError))?
            ]
        BitAnd: bitand = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a & b),
            (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a & b),
            _ => Err(Error::with_reason(Reason::TypeError))?
        ]
        BitOr: bitor = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a | b),
            (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a | b),
            _ => Err(Error::with_reason(Reason::TypeError))?
        ]
        BitXor: bitxor = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a ^ b),
            (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a ^ b),
            _ => Err(Error::with_reason(Reason::TypeError))?
        ]
        Div: div = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a / b),
            _ => Err(Error::with_reason(Reason::TypeError))?
        ]
        Mul: mul = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a * b),
            _ => Err(Error::with_reason(Reason::TypeError))?
        ]
        Rem: rem = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a % b),
            _ => Err(Error::with_reason(Reason::TypeError))?
        ]
        Shl: shl = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a << b),
            _ => Err(Error::with_reason(Reason::TypeError))?
        ]
        Shr: shr = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a >> b),
            _ => Err(Error::with_reason(Reason::TypeError))?
        ]
        Sub: sub = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a - b),
            _ => Err(Error::with_reason(Reason::TypeError))?
        ]
    }
    impl Neg for ConValue {
        type Output = IResult<Self>;
        fn neg(self) -> Self::Output {
            Ok(match self {
                ConValue::Empty => ConValue::Empty,
                ConValue::Int(v) => ConValue::Int(-v),
                _ => Err(Error::with_reason(Reason::TypeError))?,
            })
        }
    }
    impl Not for ConValue {
        type Output = IResult<Self>;
        fn not(self) -> Self::Output {
            Ok(match self {
                ConValue::Empty => ConValue::Empty,
                ConValue::Int(v) => ConValue::Int(!v),
                ConValue::Bool(v) => ConValue::Bool(!v),
                _ => Err(Error::with_reason(Reason::TypeError))?,
            })
        }
    }
    impl std::fmt::Display for ConValue {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ConValue::Empty => "Empty".fmt(f),
                ConValue::Int(v) => v.fmt(f),
                ConValue::Bool(v) => v.fmt(f),
                ConValue::Char(v) => write!(f, "'{v}'"),
                ConValue::String(v) => write!(f, "\"{v}\""),
                ConValue::RangeExc(a, b) => write!(f, "{a}..{}", b + 1),
                ConValue::RangeInc(a, b) => write!(f, "{a}..={b}"),
            }
        }
    }
}

/// A work-in-progress tree walk interpreter for Conlang
#[derive(Clone, Debug, Default)]
pub struct Interpreter {
    stack: Vec<ConValue>,
}

impl Interpreter {
    /// Creates a new [Interpreter]
    pub fn new() -> Self {
        Default::default()
    }
    /// Interprets the [Start] of a syntax tree
    pub fn interpret(&mut self, start: &Start) -> IResult<()> {
        self.visit(start)
    }
    /// Evaluates a single [Expression](expression::Expr) and returns the value stack.
    pub fn eval(&mut self, expr: &expression::Expr) -> IResult<Vec<ConValue>> {
        self.visit_expr(expr)?;
        Ok(std::mem::take(&mut self.stack))
    }
    fn push(&mut self, value: impl Into<ConValue>) {
        self.stack.push(value.into())
    }
    fn peek(&mut self) -> IResult<&ConValue> {
        self.stack
            .last()
            .ok_or(Error::with_reason(Reason::StackUnderflow))
    }
    fn pop(&mut self) -> IResult<ConValue> {
        self.stack
            .pop()
            .ok_or(Error::with_reason(Reason::StackUnderflow))
    }
    fn pop_two(&mut self) -> IResult<(ConValue, ConValue)> {
        Ok((self.pop()?, self.pop()?))
    }
}

impl Visitor<IResult<()>> for Interpreter {
    fn visit_program(&mut self, prog: &Program) -> IResult<()> {
        for stmt in &prog.0 {
            self.visit_statement(stmt)?;
        }
        Ok(())
    }

    fn visit_statement(&mut self, stmt: &Stmt) -> IResult<()> {
        match stmt {
            Stmt::Let { name, mutable, ty, init } => todo!(
                "let{} {name:?}: {ty:?} = {init:?}",
                if *mutable { " mut" } else { "" }
            ),
            Stmt::Expr(e) => {
                self.visit_expr(e)?;
                self.pop().map(drop)
            }
        }
    }

    fn visit_operation(&mut self, expr: &math::Operation) -> IResult<()> {
        use math::Operation;
        // TODO: the indentation depth here is driving me insane.
        //       maybe refactor the ast to break binary and unary
        //       operations into their own nodes, and use
        //       Operation to unify them?
        match expr {
            Operation::Binary { first, other } => {
                self.visit_operation(first)?;
                for (op, other) in other {
                    match op {
                        operator::Binary::LogAnd => {
                            if self.peek()?.truthy()? {
                                self.pop()?;
                                self.visit_operation(other)?;
                            }
                        }
                        operator::Binary::LogOr => {
                            if !self.peek()?.truthy()? {
                                self.pop()?;
                                self.visit_operation(other)?;
                            }
                        }
                        operator::Binary::LogXor => {
                            let first = self.pop()?.truthy()?;
                            self.visit_operation(other)?;
                            let second = self.pop()?.truthy()?;
                            self.push(first ^ second);
                        }
                        _ => {
                            self.visit_operation(other)?;
                            self.visit_binary_op(op)?;
                        }
                    }
                }
                Ok(())
            }
            Operation::Unary { operators, operand } => {
                self.visit_primary(operand)?;
                for op in operators.iter().rev() {
                    self.visit_unary_op(op)?;
                }
                Ok(())
            }
        }
    }

    fn visit_binary_op(&mut self, op: &operator::Binary) -> IResult<()> {
        use operator::Binary;
        let (second, first) = self.pop_two()?;
        self.push(match op {
            Binary::Mul => first * second,
            Binary::Div => first / second,
            Binary::Rem => first % second,
            Binary::Add => first + second,
            Binary::Sub => first - second,
            Binary::Lsh => first << second,
            Binary::Rsh => first >> second,
            Binary::BitAnd => first & second,
            Binary::BitOr => first | second,
            Binary::BitXor => first ^ second,
            Binary::LogAnd | Binary::LogOr | Binary::LogXor => {
                unimplemented!("Implemented in visit_operation")
            }
            Binary::RangeExc => first.range_exc(second),
            Binary::RangeInc => first.range_inc(second),
            Binary::Less => first.lt(&second),
            Binary::LessEq => first.lt_eq(&second),
            Binary::Equal => first.eq(&second),
            Binary::NotEq => first.neq(&second),
            Binary::GreaterEq => first.gt_eq(&second),
            Binary::Greater => first.gt(&second),
            Binary::Assign => todo!("Assignment"),
            Binary::AddAssign => todo!("Assignment"),
            Binary::SubAssign => todo!("Assignment"),
            Binary::MulAssign => todo!("Assignment"),
            Binary::DivAssign => todo!("Assignment"),
            Binary::RemAssign => todo!("Assignment"),
            Binary::BitAndAssign => todo!("Assignment"),
            Binary::BitOrAssign => todo!("Assignment"),
            Binary::BitXorAssign => todo!("Assignment"),
            Binary::ShlAssign => todo!("Assignment"),
            Binary::ShrAssign => todo!("Assignment"),
        }?);
        Ok(())
    }

    fn visit_unary_op(&mut self, op: &operator::Unary) -> IResult<()> {
        let operand = self.pop()?;
        self.push(match op {
            operator::Unary::RefRef => todo!(),
            operator::Unary::Ref => todo!(),
            operator::Unary::Deref => todo!(),
            operator::Unary::Neg => (-operand)?,
            operator::Unary::Not => (!operand)?,
            operator::Unary::At => todo!(),
            operator::Unary::Hash => {
                println!("{operand}");
                operand
            }
            operator::Unary::Tilde => todo!(),
        });
        Ok(())
    }

    fn visit_if(&mut self, expr: &control::If) -> IResult<()> {
        self.visit_expr(&expr.cond)?;
        if self.pop()?.truthy()? {
            self.visit_block(&expr.body)?;
        } else if let Some(block) = &expr.else_ {
            self.visit_else(block)?;
        }
        Ok(())
    }

    fn visit_while(&mut self, expr: &control::While) -> IResult<()> {
        let mut broke = false;
        while {
            self.visit_expr(&expr.cond)?;
            self.pop()?.truthy()?
        } {
            let Err(out) = self.visit_block(&expr.body) else {
                // Every expression returns a value. If allowed to pile up, they'll overflow the
                // stack.
                self.pop()?;
                continue;
            };
            match out.reason() {
                Reason::Continue => continue,
                Reason::Break(value) => {
                    self.push(value);
                    broke = true;
                    break;
                }
                r => Err(Error::with_reason(r))?,
            }
        }
        if let (Some(r#else), false) = (&expr.else_, broke) {
            self.visit_else(r#else)?;
        }
        Ok(())
    }

    fn visit_for(&mut self, expr: &control::For) -> IResult<()> {
        self.visit_expr(&expr.iter)?;
        let mut broke = false;
        let bounds = match self.pop()? {
            ConValue::RangeExc(a, b) | ConValue::RangeInc(a, b) => (a, b),
            _ => Err(Error::with_reason(Reason::NotIterable))?,
        };
        for _ in bounds.0..=bounds.1 {
            let Err(out) = self.visit_block(&expr.body) else {
                self.pop()?;
                continue;
            };
            match out.reason() {
                Reason::Continue => continue,
                Reason::Break(value) => {
                    self.push(value);
                    broke = true;
                    break;
                }
                r => Err(Error::with_reason(r))?,
            }
        }
        if let (Some(r#else), false) = (&expr.else_, broke) {
            self.visit_else(r#else)?;
        }
        Ok(())
    }

    fn visit_else(&mut self, else_: &control::Else) -> IResult<()> {
        self.visit_block(&else_.block)
    }

    fn visit_continue(&mut self, _: &control::Continue) -> IResult<()> {
        Err(Error::cnt())
    }

    fn visit_break(&mut self, brk: &control::Break) -> IResult<()> {
        Err(Error::brk({
            self.visit_expr(&brk.expr)?;
            self.pop()?
        }))
    }

    fn visit_return(&mut self, ret: &control::Return) -> IResult<()> {
        Err(Error::ret({
            self.visit_expr(&ret.expr)?;
            self.pop()?
        }))
    }

    fn visit_identifier(&mut self, ident: &Identifier) -> IResult<()> {
        todo!("Identifier lookup and scoping rules: {ident:?}")
    }

    fn visit_string_literal(&mut self, string: &str) -> IResult<()> {
        self.push(string);
        Ok(())
    }

    fn visit_char_literal(&mut self, char: &char) -> IResult<()> {
        self.push(*char);
        Ok(())
    }

    fn visit_bool_literal(&mut self, bool: &bool) -> IResult<()> {
        self.push(*bool);
        Ok(())
    }

    fn visit_float_literal(&mut self, float: &literal::Float) -> IResult<()> {
        todo!("visit floats in interpreter: {float:?}")
    }

    fn visit_int_literal(&mut self, int: &u128) -> IResult<()> {
        self.push((*int) as i128);
        Ok(())
    }

    fn visit_empty(&mut self) -> IResult<()> {
        self.push(());
        Ok(())
    }
}

pub mod error {
    //! The [Error] type represents any error thrown by the [Interpreter](super::Interpreter)
    use super::temp_type_impl::ConValue;

    pub type IResult<T> = Result<T, Error>;
    /// Represents any error thrown by the [Interpreter](super::Interpreter)
    #[derive(Clone, Debug)]
    pub struct Error {
        reason: Reason,
    }
    impl Error {
        /// Returns the [Reason] for this error
        pub fn reason(self) -> Reason {
            self.reason
        }
        /// Creates an error with a given [Reason]
        pub(crate) fn with_reason(reason: Reason) -> Self {
            Self { reason }
        }
        /// Creates a [Return](Reason::Return) error, with the given [value](ConValue)
        pub fn ret(value: ConValue) -> Self {
            Self { reason: Reason::Return(value) }
        }
        /// Creates a [Break](Reason::Break) error, with the given [value](ConValue)
        pub fn brk(value: ConValue) -> Self {
            Self { reason: Reason::Break(value) }
        }
        /// Creates a [Continue](Reason::Continue) error
        pub fn cnt() -> Self {
            Self { reason: Reason::Continue }
        }
    }

    /// The reason for the [Error]
    #[derive(Clone, Debug)]
    pub enum Reason {
        /// Propagate a Return value
        Return(ConValue),
        /// Propagate a Break value
        Break(ConValue),
        /// Continue to the next iteration of a loop
        Continue,
        /// Underflowed the stack
        StackUnderflow,
        /// Type incompatibility
        // TODO: store the type information in this error
        TypeError,
        /// In clause of For loop didn't yield a Range
        NotIterable,
    }

    impl std::error::Error for Error {}
    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.reason.fmt(f)
        }
    }
    impl std::fmt::Display for Reason {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Reason::Return(value) => write!(f, "return {value:?}"),
                Reason::Break(value) => write!(f, "break {value:?}"),
                Reason::Continue => "continue".fmt(f),
                Reason::StackUnderflow => "Stack underflow".fmt(f),
                Reason::TypeError => "Type error".fmt(f),
                Reason::NotIterable => "`in` clause of `for` loop did not yield an iterable".fmt(f),
            }
        }
    }
}
