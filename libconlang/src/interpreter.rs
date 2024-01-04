//! Interprets an AST as a program
#![allow(deprecated)] // TODO: REMOVE

use crate::ast::preamble::*;
use error::{Error, IResult};
use scope::Environment;
use temp_type_impl::ConValue;

/// Callable types can be called from within a Conlang program
pub trait Callable: std::fmt::Debug {
    /// Calls this [Callable] in the provided [Interpreter], with [ConValue] args  \
    /// The Callable is responsible for checking the argument count and validating types
    fn call(&self, interpreter: &mut Interpreter, args: &[ConValue]) -> IResult<ConValue>;
    /// Returns the common name of this identifier.
    fn name(&self) -> &str;
}

/// [BuiltIn]s are [Callable]s with bespoke definitions
pub trait BuiltIn: std::fmt::Debug + Callable {}

pub mod temp_type_impl {
    //! Temporary implementations of Conlang values
    //!
    //! The most permanent fix is a temporary one.
    use super::{
        error::{Error, IResult},
        function::Function,
        BuiltIn, Callable, Interpreter,
    };
    use std::ops::*;
    /// A Conlang value
    ///
    /// This is a hack to work around the fact that Conlang doesn't
    /// have a functioning type system yet :(
    #[derive(Clone, Debug, Default)]
    pub enum ConValue {
        /// The empty/unit `()` type
        #[default]
        Empty,
        /// An integer
        Int(i128),
        /// A boolean
        Bool(bool),
        /// A unicode character
        Char(char),
        /// A string
        String(String),
        /// A tuple
        Tuple(Vec<ConValue>),
        /// An exclusive range
        RangeExc(i128, i128),
        /// An inclusive range
        RangeInc(i128, i128),
        /// A callable thing
        Function(Function),
        /// A built-in function
        BuiltIn(&'static dyn BuiltIn),
    }
    impl ConValue {
        /// Gets whether the current value is true or false
        pub fn truthy(&self) -> IResult<bool> {
            match self {
                ConValue::Bool(v) => Ok(*v),
                _ => Err(Error::TypeError)?,
            }
        }
        pub fn range_exc(self, other: Self) -> IResult<Self> {
            let (Self::Int(a), Self::Int(b)) = (self, other) else {
                Err(Error::TypeError)?
            };
            Ok(Self::RangeExc(a, b.saturating_sub(1)))
        }
        pub fn range_inc(self, other: Self) -> IResult<Self> {
            let (Self::Int(a), Self::Int(b)) = (self, other) else {
                Err(Error::TypeError)?
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
        assign! {
            add_assign: +;
            bitand_assign: &;
            bitor_assign: |;
            bitxor_assign: ^;
            div_assign: /;
            mul_assign: *;
            rem_assign: %;
            shl_assign: <<;
            shr_assign: >>;
            sub_assign: -;
        }
    }

    impl Callable for ConValue {
        fn name(&self) -> &str {
            match self {
                ConValue::Function(func) => func.name(),
                ConValue::BuiltIn(func) => func.name(),
                _ => "",
            }
        }
        fn call(&self, interpreter: &mut Interpreter, args: &[ConValue]) -> IResult<ConValue> {
            match self {
                Self::Function(func) => func.call(interpreter, args),
                Self::BuiltIn(func) => func.call(interpreter, args),
                _ => Err(Error::NotCallable(self.clone())),
            }
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
                _ => Err(Error::TypeError)
            }
        }
    )*}
    macro assign($( $fn: ident: $op: tt );*$(;)?) {$(
        pub fn $fn(&mut self, other: Self) -> IResult<()> {
            *self = (std::mem::take(self) $op other)?;
            Ok(())
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
        Function => ConValue::Function,
        Vec<ConValue> => ConValue::Tuple,
    }
    impl From<()> for ConValue {
        fn from(_: ()) -> Self {
            Self::Empty
        }
    }
    impl From<&[ConValue]> for ConValue {
        fn from(value: &[ConValue]) -> Self {
            match value.len() {
                0 => Self::Empty,
                1 => value[0].clone(),
                _ => Self::Tuple(value.into()),
            }
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
            _ => Err(Error::TypeError)?
            ]
        BitAnd: bitand = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a & b),
            (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a & b),
            _ => Err(Error::TypeError)?
        ]
        BitOr: bitor = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a | b),
            (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a | b),
            _ => Err(Error::TypeError)?
        ]
        BitXor: bitxor = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a ^ b),
            (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a ^ b),
            _ => Err(Error::TypeError)?
        ]
        Div: div = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a / b),
            _ => Err(Error::TypeError)?
        ]
        Mul: mul = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a * b),
            _ => Err(Error::TypeError)?
        ]
        Rem: rem = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a % b),
            _ => Err(Error::TypeError)?
        ]
        Shl: shl = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a << b),
            _ => Err(Error::TypeError)?
        ]
        Shr: shr = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a >> b),
            _ => Err(Error::TypeError)?
        ]
        Sub: sub = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a - b),
            _ => Err(Error::TypeError)?
        ]
    }
    impl Neg for ConValue {
        type Output = IResult<Self>;
        fn neg(self) -> Self::Output {
            Ok(match self {
                ConValue::Empty => ConValue::Empty,
                ConValue::Int(v) => ConValue::Int(-v),
                _ => Err(Error::TypeError)?,
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
                _ => Err(Error::TypeError)?,
            })
        }
    }
    impl std::fmt::Display for ConValue {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ConValue::Empty => "Empty".fmt(f),
                ConValue::Int(v) => v.fmt(f),
                ConValue::Bool(v) => v.fmt(f),
                ConValue::Char(v) => v.fmt(f),
                ConValue::String(v) => v.fmt(f),
                ConValue::RangeExc(a, b) => write!(f, "{a}..{}", b + 1),
                ConValue::RangeInc(a, b) => write!(f, "{a}..={b}"),
                ConValue::Tuple(tuple) => {
                    '('.fmt(f)?;
                    for (idx, element) in tuple.iter().enumerate() {
                        if idx > 0 {
                            ", ".fmt(f)?
                        }
                        element.fmt(f)?
                    }
                    ')'.fmt(f)
                }
                ConValue::Function(func) => {
                    write!(f, "fn {}", func.name())
                }
                ConValue::BuiltIn(func) => {
                    write!(f, "internal fn {}", func.name())
                }
            }
        }
    }
}

/// A work-in-progress tree walk interpreter for Conlang
#[derive(Clone, Debug)]
pub struct Interpreter {
    scope: Box<Environment>,
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
    /// Calls a function inside the interpreter's scope,
    /// and returns the result
    pub fn call(&mut self, name: &str, args: &[ConValue]) -> IResult<ConValue> {
        let function = self.resolve(name)?;
        function.call(self, args)?;
        self.pop()
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
        self.stack.last().ok_or(Error::StackUnderflow)
    }
    fn pop(&mut self) -> IResult<ConValue> {
        self.stack.pop().ok_or(Error::StackUnderflow)
    }
    fn pop_two(&mut self) -> IResult<(ConValue, ConValue)> {
        Ok((self.pop()?, self.pop()?))
    }
    fn resolve(&mut self, value: &str) -> IResult<ConValue> {
        self.scope.get(value).cloned()
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
            Stmt::Let(l) => self.visit_let(l),
            Stmt::Fn(f) => self.visit_fn_decl(f),
            Stmt::Expr(e) => {
                self.visit_expr(e)?;
                self.pop().map(drop)
            }
        }
    }

    fn visit_let(&mut self, stmt: &Let) -> IResult<()> {
        let Let { name: Name { name: Identifier { name, .. }, .. }, init, .. } = stmt;
        if let Some(init) = init {
            self.visit_expr(init)?;
            let init = self.pop()?;
            self.scope.insert(name, Some(init));
        } else {
            self.scope.insert(name, None);
        }
        Ok(())
    }

    fn visit_fn_decl(&mut self, function: &FnDecl) -> IResult<()> {
        // register the function in the current environment
        self.scope.insert_fn(function);
        Ok(())
    }

    fn visit_block(&mut self, block: &expression::Block) -> IResult<()> {
        for stmt in &block.statements {
            self.visit_statement(stmt)?;
        }
        if let Some(expr) = block.expr.as_ref() {
            self.visit_expr(expr)
        } else {
            self.push(ConValue::Empty);
            Ok(())
        }
    }

    fn visit_tuple(&mut self, tuple: &Tuple) -> IResult<()> {
        let mut out = vec![];
        for expr in &tuple.elements {
            self.visit_expr(expr)?;
            out.push(self.pop()?);
        }
        self.push(out);
        Ok(())
    }

    fn visit_fn_call(&mut self, call: &FnCall) -> IResult<()> {
        // evaluate the callee
        self.visit_primary(&call.callee)?;
        for args in &call.args {
            self.visit_tuple(args)?;
            let (ConValue::Tuple(args), callee) = self.pop_two()? else {
                Err(Error::TypeError)?
            };
            let return_value = callee.call(self, &args)?;
            self.push(return_value);
        }
        Ok(())
    }

    fn visit_assign(&mut self, assign: &math::Assign) -> IResult<()> {
        use operator::Assign;
        let math::Assign { target, operator, init } = assign;
        self.visit_operation(init)?;
        let init = self.pop()?;
        let resolved = self.scope.get_mut(&target.name)?;

        if let Assign::Assign = operator {
            use std::mem::discriminant as variant;
            // runtime typecheck
            match resolved.as_mut() {
                Some(value) if variant(value) == variant(&init) => {
                    *value = init;
                }
                None => *resolved = Some(init),
                _ => Err(Error::TypeError)?,
            }
            self.push(ConValue::Empty);
            return Ok(());
        }

        let Some(target) = resolved.as_mut() else {
            Err(Error::NotInitialized(target.name.to_owned()))?
        };

        match operator {
            Assign::AddAssign => target.add_assign(init)?,
            Assign::SubAssign => target.sub_assign(init)?,
            Assign::MulAssign => target.mul_assign(init)?,
            Assign::DivAssign => target.div_assign(init)?,
            Assign::RemAssign => target.rem_assign(init)?,
            Assign::BitAndAssign => target.bitand_assign(init)?,
            Assign::BitOrAssign => target.bitor_assign(init)?,
            Assign::BitXorAssign => target.bitxor_assign(init)?,
            Assign::ShlAssign => target.shl_assign(init)?,
            Assign::ShrAssign => target.shr_assign(init)?,
            _ => (),
        }

        self.push(ConValue::Empty);

        Ok(())
    }

    fn visit_binary(&mut self, bin: &math::Binary) -> IResult<()> {
        let Binary { first, other } = bin;

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

    fn visit_unary(&mut self, unary: &math::Unary) -> IResult<()> {
        let Unary { operand, operators } = unary;

        self.visit_operation(operand)?;

        for op in operators.iter().rev() {
            self.visit_unary_op(op)?;
        }

        Ok(())
    }

    fn visit_assign_op(&mut self, _: &operator::Assign) -> IResult<()> {
        unimplemented!("visit_assign_op is implemented in visit_operation")
    }

    fn visit_binary_op(&mut self, op: &operator::Binary) -> IResult<()> {
        use operator::Binary;
        let (second, first) = self.pop_two()?;

        let out = match op {
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
        }?;

        self.push(out);
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

    fn visit_if(&mut self, expr: &If) -> IResult<()> {
        self.visit_expr(&expr.cond)?;
        if self.pop()?.truthy()? {
            self.visit_block(&expr.body)?;
        } else if let Some(block) = &expr.else_ {
            self.visit_else(block)?;
        } else {
            self.push(ConValue::Empty)
        }
        Ok(())
    }

    fn visit_while(&mut self, expr: &While) -> IResult<()> {
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
            match out {
                Error::Continue => continue,
                Error::Break(value) => {
                    self.push(value);
                    return Ok(());
                }
                r => Err(r)?,
            }
        }
        if let Some(r#else) = &expr.else_ {
            self.visit_else(r#else)?;
        } else {
            self.push(ConValue::Empty);
        }
        Ok(())
    }

    fn visit_for(&mut self, expr: &control::For) -> IResult<()> {
        self.scope.enter();
        self.visit_expr(&expr.iter)?;
        let bounds = match self.pop()? {
            ConValue::RangeExc(a, b) | ConValue::RangeInc(a, b) => (a, b),
            _ => Err(Error::NotIterable)?,
        };
        for loop_var in bounds.0..=bounds.1 {
            self.scope.insert(&expr.var.name, Some(loop_var.into()));
            let Err(out) = self.visit_block(&expr.body) else {
                self.pop()?;
                continue;
            };
            match out {
                Error::Continue => continue,
                Error::Break(value) => {
                    self.push(value);
                    return Ok(());
                }
                r => Err(r)?,
            }
        }
        if let Some(r#else) = &expr.else_ {
            self.visit_else(r#else)?;
        } else {
            self.push(ConValue::Empty)
        }
        self.scope.exit()?;
        Ok(())
    }

    fn visit_else(&mut self, else_: &control::Else) -> IResult<()> {
        self.visit_expr(&else_.expr)
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
        let value = self.resolve(&ident.name)?;
        self.push(value);
        Ok(())
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

impl Default for Interpreter {
    fn default() -> Self {
        Self { scope: Environment::new().into(), stack: Default::default() }
    }
}

pub mod function {
    //! Represents a block of code which lives inside the Interpreter
    use super::{
        // scope::Environment,
        Callable,
        ConValue,
        Error,
        FnDecl,
        IResult,
        Identifier,
        Interpreter,
    };
    use crate::ast::{preamble::Name, visitor::Visitor};
    /// Represents a block of code which persists inside the Interpreter
    #[derive(Clone, Debug)]
    pub struct Function {
        /// Stores the contents of the function declaration
        declaration: Box<FnDecl>,
        // /// Stores the enclosing scope of the function
        // TODO: Capture variables
        //environment: Box<Environment>,
    }

    impl Function {
        pub fn new(declaration: &FnDecl) -> Self {
            Self {
                declaration: declaration.clone().into(),
                //environment: Box::new(Environment::new()),
            }
        }
    }

    impl Callable for Function {
        fn name(&self) -> &str {
            &self.declaration.name.name.name
        }
        fn call(&self, interpreter: &mut Interpreter, args: &[ConValue]) -> IResult<ConValue> {
            // Check arg mapping
            if args.len() != self.declaration.args.len() {
                return Err(Error::ArgNumber {
                    want: self.declaration.args.len(),
                    got: args.len(),
                });
            }
            // TODO: Isolate cross-function scopes!
            interpreter.scope.enter();
            for (Name { name: Identifier { name, .. }, .. }, value) in
                self.declaration.args.iter().zip(args)
            {
                interpreter.scope.insert(name, Some(value.clone()));
            }
            match interpreter.visit_block(&self.declaration.body) {
                Err(Error::Return(value)) => interpreter.push(value),
                Err(Error::Break(value)) => Err(Error::BadBreak(value))?,
                Err(e) => Err(e)?,
                Ok(()) => (),
            }
            interpreter.scope.exit()?;
            interpreter.pop()
        }
    }
}

pub mod builtin {
    mod builtin_imports {
        pub use crate::interpreter::{
            error::IResult, temp_type_impl::ConValue, BuiltIn, Callable, Interpreter,
        };
    }
    use super::BuiltIn;
    /// Builtins to load when a new interpreter is created
    pub const DEFAULT_BUILTINS: &[&dyn BuiltIn] = &[&print::Print, &dbg::Dbg, &dump::Dump];

    mod print {
        //! Implements the unstable `print(...)` builtin
        use super::builtin_imports::*;
        /// Implements the `print(...)` builtin
        #[derive(Clone, Debug)]
        pub struct Print;
        impl BuiltIn for Print {}
        #[rustfmt::skip]
        impl Callable for Print {
            fn name(&self) -> &'static str { "print" }
            fn call(&self, _inter: &mut Interpreter, args: &[ConValue]) -> IResult<ConValue> {
                for arg in args {
                    print!("{arg}")
                }
                println!();
                Ok(ConValue::Empty)
            }
        }
    }
    mod dbg {
        //! Implements the unstable `dbg(...)` builtin
        use super::builtin_imports::*;
        #[derive(Clone, Debug)]
        pub struct Dbg;
        impl BuiltIn for Dbg {}
        #[rustfmt::skip]
        impl Callable for Dbg {
            fn name(&self) -> &str { "dbg" }
            fn call(&self, _inter: &mut Interpreter, args: &[ConValue]) -> IResult<ConValue> {
                println!("{args:?}");
                Ok(args.into())
            }
        }
    }

    mod dump {
        use super::builtin_imports::*;
        #[derive(Clone, Debug)]
        pub struct Dump;
        impl BuiltIn for Dump {}
        impl Callable for Dump {
            fn call(&self, interpreter: &mut Interpreter, _args: &[ConValue]) -> IResult<ConValue> {
                println!("Scope:\n{}", interpreter.scope);
                println!("Stack:{:#?}", interpreter.stack);
                Ok(ConValue::Empty)
            }

            fn name(&self) -> &str {
                "dump"
            }
        }
    }
}

pub mod scope {
    //! Lexical and non-lexical scoping for variables

    use super::{
        builtin::DEFAULT_BUILTINS,
        error::{Error, IResult},
        function::Function,
        temp_type_impl::ConValue,
        FnDecl,
    };
    use std::{collections::HashMap, fmt::Display};

    /// Implements a nested lexical scope
    #[derive(Clone, Debug, Default)]
    pub struct Environment {
        outer: Option<Box<Self>>,
        vars: HashMap<String, Option<ConValue>>,
    }

    impl Display for Environment {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            for (var, val) in &self.vars {
                writeln!(f, "{var}: {}", if val.is_some() { "..." } else { "None" })?;
            }
            "--- Frame ---\n".fmt(f)?;
            if let Some(outer) = &self.outer {
                outer.fmt(f)?;
            }
            Ok(())
        }
    }

    impl Environment {
        pub fn new() -> Self {
            let mut out = Self::default();
            for &builtin in DEFAULT_BUILTINS {
                out.insert(builtin.name(), Some(ConValue::BuiltIn(builtin)))
            }
            out
        }
        /// Enter a nested scope
        pub fn enter(&mut self) {
            let outer = std::mem::take(self);
            self.outer = Some(outer.into());
        }
        /// Exits the scope, destroying all local variables and
        /// returning the outer scope, if there is one
        pub fn exit(&mut self) -> IResult<()> {
            if let Some(outer) = std::mem::take(&mut self.outer) {
                *self = *outer;
                Ok(())
            } else {
                Err(Error::ScopeExit)
            }
        }
        /// Resolves a variable mutably
        ///
        /// Returns a mutable reference to the variable's record, if it exists
        pub fn get_mut(&mut self, id: &str) -> IResult<&mut Option<ConValue>> {
            match self.vars.get_mut(id) {
                Some(var) => Ok(var),
                None => self
                    .outer
                    .as_mut()
                    .ok_or_else(|| Error::NotDefined(id.into()))?
                    .get_mut(id),
            }
        }
        /// Resolves a variable immutably
        pub fn get(&self, id: &str) -> IResult<&ConValue> {
            match self.vars.get(id) {
                Some(var) => var.as_ref().ok_or_else(|| Error::NotInitialized(id.into())),
                None => self
                    .outer
                    .as_ref()
                    .ok_or_else(|| Error::NotDefined(id.into()))?
                    .get(id),
            }
        }
        pub fn insert(&mut self, id: &str, value: Option<ConValue>) {
            self.vars.insert(id.to_string(), value);
        }
        /// A convenience function for registering a [FnDecl] as a [Function]
        pub fn insert_fn(&mut self, decl: &FnDecl) {
            self.vars.insert(
                decl.name.name.name.clone(),
                Some(Function::new(decl).into()),
            );
        }
    }
}

pub mod error {
    //! The [Error] type represents any error thrown by the [Interpreter](super::Interpreter)

    use super::temp_type_impl::ConValue;

    pub type IResult<T> = Result<T, Error>;
    /// Represents any error thrown by the [Interpreter](super::Interpreter)
    impl Error {
        /// Creates a [Return](Error::Return) error, with the given [value](ConValue)
        pub fn ret(value: ConValue) -> Self {
            Error::Return(value)
        }
        /// Creates a [Break](Error::Break) error, with the given [value](ConValue)
        pub fn brk(value: ConValue) -> Self {
            Error::Break(value)
        }
        /// Creates a [Continue](Error::Continue) error
        pub fn cnt() -> Self {
            Error::Continue
        }
    }

    /// The reason for the [Error]
    #[derive(Clone, Debug)]
    pub enum Error {
        /// Propagate a Return value
        Return(ConValue),
        /// Propagate a Break value
        Break(ConValue),
        /// Break propagated across function bounds
        BadBreak(ConValue),
        /// Continue to the next iteration of a loop
        Continue,
        /// Underflowed the stack
        StackUnderflow,
        /// Exited the last scope
        ScopeExit,
        /// Type incompatibility
        // TODO: store the type information in this error
        TypeError,
        /// In clause of For loop didn't yield a Range
        NotIterable,
        /// A name was not defined in scope before being used
        NotDefined(String),
        /// A name was defined but not initialized
        NotInitialized(String),
        /// A value was called, but is not callable
        NotCallable(ConValue),
        /// A function was called with the wrong number of arguments
        ArgNumber { want: usize, got: usize },
    }

    impl std::error::Error for Error {}
    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Return(value) => write!(f, "return {value}"),
                Error::Break(value) => write!(f, "break {value}"),
                Error::BadBreak(value) => write!(f, "rogue break: {value}"),
                Error::Continue => "continue".fmt(f),
                Error::StackUnderflow => "Stack underflow".fmt(f),
                Error::ScopeExit => "Exited the last scope. This is a logic bug.".fmt(f),
                Error::TypeError => "Incompatible types".fmt(f),
                Error::NotIterable => "`in` clause of `for` loop did not yield an iterable".fmt(f),
                Error::NotDefined(value) => {
                    write!(f, "{value} not bound. Did you mean `let {value};`?")
                }
                Error::NotInitialized(value) => {
                    write!(f, "{value} bound, but not initialized")
                }
                Error::NotCallable(value) => {
                    write!(f, "{value} is not a function, and cannot be called")
                }
                Error::ArgNumber { want, got } => {
                    write!(f, "Expected {want} arguments, got {got}")
                }
            }
        }
    }
}
