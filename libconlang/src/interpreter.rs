//! Interprets an AST as a program

use crate::ast::preamble::*;
use env::Environment;
use error::{Error, IResult};
use temp_type_impl::ConValue;

/// Callable types can be called from within a Conlang program
pub trait Callable: std::fmt::Debug {
    /// Calls this [Callable] in the provided [Environment], with [ConValue] args  \
    /// The Callable is responsible for checking the argument count and validating types
    fn call(&self, interpreter: &mut Environment, args: &[ConValue]) -> IResult<ConValue>;
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
        BuiltIn, Callable, Environment,
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
        fn call(&self, interpreter: &mut Environment, args: &[ConValue]) -> IResult<ConValue> {
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
pub trait Interpret {
    /// Interprets this thing in the given [`Environment`].
    ///
    /// Everything returns a value!™
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue>;
}

impl Interpret for Start {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        self.0.interpret(env)
    }
}
impl Interpret for Program {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let mut out = ConValue::Empty;
        for stmt in &self.0 {
            out = stmt.interpret(env)?;
        }
        Ok(out)
    }
}
impl Interpret for Stmt {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        match self {
            Stmt::Let(l) => l.interpret(env),
            Stmt::Fn(f) => f.interpret(env),
            Stmt::Expr(e) => e.interpret(env),
        }
    }
}
impl Interpret for Let {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Let { name: Name { symbol: Identifier { name, .. }, .. }, init, .. } = self;
        if let Some(init) = init {
            let init = init.interpret(env)?;
            env.insert(name, Some(init));
        } else {
            env.insert(name, None);
        }
        Ok(ConValue::Empty)
    }
}
impl Interpret for FnDecl {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        // register the function in the current environment
        env.insert_fn(self);
        Ok(ConValue::Empty)
    }
}
impl Interpret for Expr {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        self.0.interpret(env)
    }
}
impl Interpret for Operation {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        match self {
            Operation::Assign(op) => op.interpret(env),
            Operation::Binary(op) => op.interpret(env),
            Operation::Unary(op) => op.interpret(env),
            Operation::Call(op) => op.interpret(env),
        }
    }
}
impl Interpret for Assign {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        use operator::Assign;
        let math::Assign { target: Identifier { name, .. }, operator, init } = self;
        let init = init.interpret(env)?;
        let target = env.get_mut(name)?;

        if let Assign::Assign = operator {
            use std::mem::discriminant as variant;
            // runtime typecheck
            match target {
                Some(value) if variant(value) == variant(&init) => {
                    *value = init;
                }
                value @ None => *value = Some(init),
                _ => Err(Error::TypeError)?,
            }
            return Ok(ConValue::Empty);
        }
        let Some(target) = target else {
            return Err(Error::NotInitialized(name.into()));
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
        Ok(ConValue::Empty)
    }
}
impl Interpret for Binary {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Binary { first, other } = self;
        let mut first = first.interpret(env)?;
        for (op, other) in other {
            first = match op {
                operator::Binary::LogAnd => {
                    if first.truthy()? {
                        other.interpret(env)
                    } else {
                        return Ok(first); // Short circuiting
                    }
                }
                operator::Binary::LogOr => {
                    if !first.truthy()? {
                        other.interpret(env)
                    } else {
                        return Ok(first); // Short circuiting
                    }
                }
                operator::Binary::LogXor => {
                    // TODO: It should be possible to assemble better error information from this
                    let (lhs, rhs) = (first.truthy()?, other.interpret(env)?.truthy()?);
                    Ok(ConValue::Bool(lhs ^ rhs))
                }
                // TODO: For all overloadable operators, transmute into function call
                operator::Binary::Mul => first * other.interpret(env)?,
                operator::Binary::Div => first / other.interpret(env)?,
                operator::Binary::Rem => first % other.interpret(env)?,
                operator::Binary::Add => first + other.interpret(env)?,
                operator::Binary::Sub => first - other.interpret(env)?,
                operator::Binary::Lsh => first << other.interpret(env)?,
                operator::Binary::Rsh => first >> other.interpret(env)?,
                operator::Binary::BitAnd => first & other.interpret(env)?,
                operator::Binary::BitOr => first | other.interpret(env)?,
                operator::Binary::BitXor => first ^ other.interpret(env)?,
                operator::Binary::RangeExc => first.range_exc(other.interpret(env)?),
                operator::Binary::RangeInc => first.range_inc(other.interpret(env)?),
                operator::Binary::Less => first.lt(&other.interpret(env)?),
                operator::Binary::LessEq => first.lt_eq(&other.interpret(env)?),
                operator::Binary::Equal => first.eq(&other.interpret(env)?),
                operator::Binary::NotEq => first.neq(&other.interpret(env)?),
                operator::Binary::GreaterEq => first.gt_eq(&other.interpret(env)?),
                operator::Binary::Greater => first.gt(&other.interpret(env)?),
            }?;
        }
        Ok(first)
    }
}
impl Interpret for Unary {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Unary { operand, operators } = self;
        let mut operand = operand.interpret(env)?;

        for op in operators.iter().rev() {
            operand = match op {
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
            };
        }
        Ok(operand)
    }
}
impl Interpret for Call {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        match self {
            Call::FnCall(fncall) => fncall.interpret(env),
            Call::Primary(primary) => primary.interpret(env),
        }
    }
}
impl Interpret for FnCall {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        // evaluate the callee
        let mut callee = self.callee.interpret(env)?;
        for args in &self.args {
            let ConValue::Tuple(args) = args.interpret(env)? else {
                Err(Error::TypeError)?
            };
            callee = callee.call(env, &args)?;
        }
        Ok(callee)
    }
}
impl Interpret for Primary {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        match self {
            Primary::Identifier(prim) => prim.interpret(env),
            Primary::Literal(prim) => prim.interpret(env),
            Primary::Block(prim) => prim.interpret(env),
            Primary::Group(prim) => prim.interpret(env),
            Primary::Branch(prim) => prim.interpret(env),
        }
    }
}
impl Interpret for Identifier {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        env.get(&self.name).cloned()
    }
}
impl Interpret for Literal {
    fn interpret(&self, _env: &mut Environment) -> IResult<ConValue> {
        Ok(match self {
            Literal::String(value) => ConValue::from(value.as_str()),
            Literal::Char(value) => ConValue::Char(*value),
            Literal::Bool(value) => ConValue::Bool(*value),
            Literal::Float(value) => todo!("Float values in interpreter: {value:?}"),
            Literal::Int(value) => ConValue::Int(*value as _),
        })
    }
}
impl Interpret for Block {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let mut env = env.frame("block");
        // TODO: this could TOTALLY be done with a binary operator.
        for stmt in &self.statements {
            stmt.interpret(&mut env)?;
        }
        if let Some(expr) = self.expr.as_ref() {
            expr.interpret(&mut env)
        } else {
            Ok(ConValue::Empty)
        }
    }
}
impl Interpret for Group {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        match self {
            Group::Tuple(tuple) => tuple.interpret(env),
            Group::Single(value) => value.interpret(env),
            Group::Empty => Ok(ConValue::Empty),
        }
    }
}
impl Interpret for Tuple {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        Ok(ConValue::Tuple(self.elements.iter().try_fold(
            vec![],
            |mut out, element| {
                out.push(element.interpret(env)?);
                Ok(out)
            },
        )?))
    }
}
impl Interpret for Flow {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        match self {
            Flow::While(flow) => flow.interpret(env),
            Flow::If(flow) => flow.interpret(env),
            Flow::For(flow) => flow.interpret(env),
            Flow::Continue(flow) => flow.interpret(env),
            Flow::Return(flow) => flow.interpret(env),
            Flow::Break(flow) => flow.interpret(env),
        }
    }
}
impl Interpret for While {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        while self.cond.interpret(env)?.truthy()? {
            match self.body.interpret(env) {
                Err(Error::Break(value)) => return Ok(value),
                Err(Error::Continue) => continue,
                e => e?,
            };
        }
        if let Some(other) = &self.else_ {
            other.interpret(env)
        } else {
            Ok(ConValue::Empty)
        }
    }
}
impl Interpret for Else {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        self.expr.interpret(env)
    }
}
impl Interpret for If {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        if self.cond.interpret(env)?.truthy()? {
            self.body.interpret(env)
        } else if let Some(other) = &self.else_ {
            other.interpret(env)
        } else {
            Ok(ConValue::Empty)
        }
    }
}
impl Interpret for For {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let bounds = match self.iter.interpret(env)? {
            ConValue::RangeExc(a, b) => a..=b,
            ConValue::RangeInc(a, b) => a..=b,
            _ => Err(Error::TypeError)?,
        };

        let mut env = env.frame("loop variable");
        for loop_var in bounds {
            env.insert(&self.var.name, Some(loop_var.into()));
            match self.body.interpret(&mut env) {
                Err(Error::Break(value)) => return Ok(value),
                Err(Error::Continue) => continue,
                result => result?,
            };
        }
        if let Some(other) = &self.else_ {
            other.interpret(&mut env)
        } else {
            Ok(ConValue::Empty)
        }
    }
}
impl Interpret for Continue {
    fn interpret(&self, _env: &mut Environment) -> IResult<ConValue> {
        Err(Error::Continue)
    }
}
impl Interpret for Return {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        Err(Error::Return(self.expr.interpret(env)?))
    }
}
impl Interpret for Break {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        Err(Error::Break(self.expr.interpret(env)?))
    }
}

pub mod function {
    //! Represents a block of code which lives inside the Interpreter
    use super::{Callable, ConValue, Environment, Error, FnDecl, IResult, Identifier, Interpret};
    use crate::ast::preamble::Name;
    /// Represents a block of code which persists inside the Interpreter
    #[derive(Clone, Debug)]
    pub struct Function {
        /// Stores the contents of the function declaration
        decl: Box<FnDecl>,
        // /// Stores the enclosing scope of the function
        // env: Box<Environment>,
    }

    impl Function {
        pub fn new(decl: &FnDecl) -> Self {
            Self { decl: decl.clone().into() }
        }
    }

    impl Callable for Function {
        fn name(&self) -> &str {
            &self.decl.name.symbol.name
        }
        fn call(&self, env: &mut Environment, args: &[ConValue]) -> IResult<ConValue> {
            // Check arg mapping
            if args.len() != self.decl.args.len() {
                return Err(Error::ArgNumber { want: self.decl.args.len(), got: args.len() });
            }
            // TODO: Isolate cross-function scopes!
            // let mut env = self.env.clone();
            let mut frame = env.frame("fn args");
            for (Name { symbol: Identifier { name, .. }, .. }, value) in
                self.decl.args.iter().zip(args)
            {
                frame.insert(name, Some(value.clone()));
            }
            match self.decl.body.interpret(&mut frame) {
                Err(Error::Return(value)) => Ok(value),
                Err(Error::Break(value)) => Err(Error::BadBreak(value)),
                result => result,
            }
        }
    }
}

pub mod builtin {
    //! Implementations of built-in functions
    mod builtin_imports {
        pub use crate::interpreter::{
            env::Environment, error::IResult, temp_type_impl::ConValue, BuiltIn, Callable,
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
            fn call(&self, _inter: &mut Environment, args: &[ConValue]) -> IResult<ConValue> {
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
            fn call(&self, _inter: &mut Environment, args: &[ConValue]) -> IResult<ConValue> {
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
            fn call(&self, env: &mut Environment, _args: &[ConValue]) -> IResult<ConValue> {
                println!("{}", *env);
                Ok(ConValue::Empty)
            }

            fn name(&self) -> &str {
                "dump"
            }
        }
    }
}

pub mod env {
    //! Lexical and non-lexical scoping for variables

    use super::{
        builtin::DEFAULT_BUILTINS,
        error::{Error, IResult},
        function::Function,
        temp_type_impl::ConValue,
        Callable, FnDecl, Interpret,
    };
    use std::{
        collections::HashMap,
        fmt::Display,
        ops::{Deref, DerefMut},
    };

    /// Implements a nested lexical scope
    #[derive(Clone, Debug)]
    pub struct Environment {
        frames: Vec<(HashMap<String, Option<ConValue>>, &'static str)>,
    }

    impl Display for Environment {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            for (frame, name) in self.frames.iter().rev() {
                writeln!(f, "--- {name} ---")?;
                for (var, val) in frame {
                    write!(f, "- {var}: ")?;
                    match val {
                        Some(value) => writeln!(f, "{value}"),
                        None => writeln!(f, "<undefined>"),
                    }?
                }
            }
            Ok(())
        }
    }
    impl Default for Environment {
        fn default() -> Self {
            let mut builtins = HashMap::new();
            for &builtin in DEFAULT_BUILTINS {
                builtins.insert(builtin.name().into(), Some(ConValue::BuiltIn(builtin)));
            }
            // FIXME: Temporary until modules are implemented
            Self { frames: vec![(builtins, "builtins"), (HashMap::new(), "globals")] }
        }
    }

    impl Environment {
        pub fn new() -> Self {
            Self::default()
        }
        /// Creates an [Environment] with no [builtins](super::builtin)
        pub fn no_builtins(name: &'static str) -> Self {
            Self { frames: vec![(Default::default(), name)] }
        }

        pub fn eval(&mut self, node: &impl Interpret) -> IResult<ConValue> {
            node.interpret(self)
        }

        /// Calls a function inside the interpreter's scope,
        /// and returns the result
        pub fn call(&mut self, name: &str, args: &[ConValue]) -> IResult<ConValue> {
            // FIXME: Clone to satisfy the borrow checker
            let function = self.get(name)?.clone();
            function.call(self, args)
        }
        /// Enters a nested scope, returning a [`Frame`] stack-guard.
        ///
        /// [`Frame`] implements Deref/DerefMut for [`Environment`].
        pub fn frame(&mut self, name: &'static str) -> Frame {
            Frame::new(self, name)
        }
        /// Resolves a variable mutably.
        ///
        /// Returns a mutable reference to the variable's record, if it exists.
        pub fn get_mut(&mut self, id: &str) -> IResult<&mut Option<ConValue>> {
            for (frame, _) in self.frames.iter_mut().rev() {
                if let Some(var) = frame.get_mut(id) {
                    return Ok(var);
                }
            }
            Err(Error::NotDefined(id.into()))
        }
        /// Resolves a variable immutably.
        ///
        /// Returns a reference to the variable's contents, if it is defined and initialized.
        pub fn get(&self, id: &str) -> IResult<&ConValue> {
            for (frame, _) in self.frames.iter().rev() {
                match frame.get(id) {
                    Some(Some(var)) => return Ok(var),
                    Some(None) => return Err(Error::NotInitialized(id.into())),
                    _ => (),
                }
            }
            Err(Error::NotDefined(id.into()))
        }
        /// Inserts a new [ConValue] into this [Environment]
        pub fn insert(&mut self, id: &str, value: Option<ConValue>) {
            if let Some((frame, _)) = self.frames.last_mut() {
                frame.insert(id.into(), value);
            }
        }
        /// A convenience function for registering a [FnDecl] as a [Function]
        pub fn insert_fn(&mut self, decl: &FnDecl) {
            let (name, function) = (
                decl.name.symbol.name.clone(),
                Some(Function::new(decl).into()),
            );
            if let Some((frame, _)) = self.frames.last_mut() {
                frame.insert(name, function);
            }
        }
    }

    /// Functions which aid in the implementation of [`Frame`]
    impl Environment {
        /// Enters a scope, creating a new namespace for variables
        fn enter(&mut self, name: &'static str) -> &mut Self {
            self.frames.push((Default::default(), name));
            self
        }

        /// Exits the scope, destroying all local variables and
        /// returning the outer scope, if there is one
        fn exit(&mut self) -> &mut Self {
            if self.frames.len() > 2 {
                self.frames.pop();
            }
            self
        }
    }

    /// Represents a stack frame
    #[derive(Debug)]
    pub struct Frame<'scope> {
        scope: &'scope mut Environment,
    }
    impl<'scope> Frame<'scope> {
        fn new(scope: &'scope mut Environment, name: &'static str) -> Self {
            Self { scope: scope.enter(name) }
        }
    }
    impl<'scope> Deref for Frame<'scope> {
        type Target = Environment;
        fn deref(&self) -> &Self::Target {
            self.scope
        }
    }
    impl<'scope> DerefMut for Frame<'scope> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.scope
        }
    }
    impl<'scope> Drop for Frame<'scope> {
        fn drop(&mut self) {
            self.scope.exit();
        }
    }
}

pub mod error {
    //! The [Error] type represents any error thrown by the [Environment](super::Environment)

    use super::temp_type_impl::ConValue;

    pub type IResult<T> = Result<T, Error>;

    /// Represents any error thrown by the [Environment](super::Environment)
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

#[cfg(test)]
mod tests;
