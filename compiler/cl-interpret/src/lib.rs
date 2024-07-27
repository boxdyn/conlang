//! Walks a Conlang AST, interpreting it as a program.
#![warn(clippy::all)]
#![feature(decl_macro)]

use cl_ast::Sym;
use convalue::ConValue;
use env::Environment;
use error::{Error, IResult};
use interpret::Interpret;

/// Callable types can be called from within a Conlang program
pub trait Callable: std::fmt::Debug {
    /// Calls this [Callable] in the provided [Environment], with [ConValue] args  \
    /// The Callable is responsible for checking the argument count and validating types
    fn call(&self, interpreter: &mut Environment, args: &[ConValue]) -> IResult<ConValue>;
    /// Returns the common name of this identifier.
    fn name(&self) -> Sym;
}

/// [BuiltIn]s are [Callable]s with bespoke definitions
pub trait BuiltIn: std::fmt::Debug + Callable {
    fn description(&self) -> &str;
}

pub mod convalue {
    //! Values in the dynamically typed AST interpreter.
    //!
    //! The most permanent fix is a temporary one.
    use cl_ast::Sym;

    use super::{
        error::{Error, IResult},
        function::Function,
        BuiltIn, Callable, Environment,
    };
    use std::{ops::*, rc::Rc};

    type Integer = isize;

    /// A Conlang value stores data in the interpreter
    #[derive(Clone, Debug, Default)]
    pub enum ConValue {
        /// The empty/unit `()` type
        #[default]
        Empty,
        /// An integer
        Int(Integer),
        /// A boolean
        Bool(bool),
        /// A unicode character
        Char(char),
        /// A string
        String(Sym),
        /// A reference
        Ref(Rc<ConValue>),
        /// An Array
        Array(Rc<[ConValue]>),
        /// A tuple
        Tuple(Rc<[ConValue]>),
        /// An exclusive range
        RangeExc(Integer, Integer),
        /// An inclusive range
        RangeInc(Integer, Integer),
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
        pub fn index(&self, index: &Self) -> IResult<ConValue> {
            let Self::Int(index) = index else {
                Err(Error::TypeError)?
            };
            match self {
                ConValue::String(string) => string
                    .chars()
                    .nth(*index as _)
                    .map(ConValue::Char)
                    .ok_or(Error::OobIndex(*index as usize, string.chars().count())),
                ConValue::Array(arr) => arr
                    .get(*index as usize)
                    .cloned()
                    .ok_or(Error::OobIndex(*index as usize, arr.len())),
                _ => Err(Error::TypeError),
            }
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
        fn name(&self) -> Sym {
            match self {
                ConValue::Function(func) => func.name(),
                ConValue::BuiltIn(func) => func.name(),
                _ => "".into(),
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
                (Self::String(a), Self::String(b)) => Ok(Self::Bool(&**a $op &**b)),
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
    impl From<&Sym> for ConValue {
        fn from(value: &Sym) -> Self {
            ConValue::String(*value)
        }
    }
    from! {
        Integer => ConValue::Int,
        bool => ConValue::Bool,
        char => ConValue::Char,
        Sym => ConValue::String,
        &str => ConValue::String,
        String => ConValue::String,
        Rc<str> => ConValue::String,
        Function => ConValue::Function,
        Vec<ConValue> => ConValue::Tuple,
        &'static dyn BuiltIn => ConValue::BuiltIn,
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
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_add(b)),
            (ConValue::String(a), ConValue::String(b)) => (a.to_string() + &b.to_string()).into(),
            (ConValue::String(s), ConValue::Char(c)) => { let mut s = s.to_string(); s.push(c); s.into() }
            (ConValue::Char(a), ConValue::Char(b)) => {
                ConValue::String([a, b].into_iter().collect::<String>().into())
            }
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
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.checked_div(b).unwrap_or_else(|| {
                eprintln!("Warning: Divide by zero in {a} / {b}"); a
            })),
            _ => Err(Error::TypeError)?
        ]
        Mul: mul = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_mul(b)),
            _ => Err(Error::TypeError)?
        ]
        Rem: rem = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.checked_rem(b).unwrap_or_else(|| {
                eprintln!("Warning: Divide by zero in {a} % {b}"); a
            })),
            _ => Err(Error::TypeError)?
        ]
        Shl: shl = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_shl(b as _)),
            _ => Err(Error::TypeError)?
        ]
        Shr: shr = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_shr(b as _)),
            _ => Err(Error::TypeError)?
        ]
        Sub: sub = [
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a.wrapping_sub(b)),
            _ => Err(Error::TypeError)?
        ]
    }
    impl std::fmt::Display for ConValue {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ConValue::Empty => "Empty".fmt(f),
                ConValue::Int(v) => v.fmt(f),
                ConValue::Bool(v) => v.fmt(f),
                ConValue::Char(v) => v.fmt(f),
                ConValue::String(v) => v.fmt(f),
                ConValue::Ref(v) => write!(f, "&{v}"),
                ConValue::Array(array) => {
                    '['.fmt(f)?;
                    for (idx, element) in array.iter().enumerate() {
                        if idx > 0 {
                            ", ".fmt(f)?
                        }
                        element.fmt(f)?
                    }
                    ']'.fmt(f)
                }
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
                    write!(f, "{}", func.decl())
                }
                ConValue::BuiltIn(func) => {
                    write!(f, "{}", func.description())
                }
            }
        }
    }
}

pub mod interpret;

pub mod function {
    //! Represents a block of code which lives inside the Interpreter

    use super::{Callable, ConValue, Environment, Error, IResult, Interpret};
    use cl_ast::{Function as FnDecl, Param, Sym};
    use std::rc::Rc;
    /// Represents a block of code which persists inside the Interpreter
    #[derive(Clone, Debug)]
    pub struct Function {
        /// Stores the contents of the function declaration
        decl: Rc<FnDecl>,
        // /// Stores the enclosing scope of the function
        // env: Box<Environment>,
    }

    impl Function {
        pub fn new(decl: &FnDecl) -> Self {
            Self { decl: decl.clone().into() }
        }
        pub fn decl(&self) -> &FnDecl {
            &self.decl
        }
    }

    impl Callable for Function {
        fn name(&self) -> Sym {
            let FnDecl { name, .. } = *self.decl;
            name
        }
        fn call(&self, env: &mut Environment, args: &[ConValue]) -> IResult<ConValue> {
            let FnDecl { name, bind, body, sign: _ } = &*self.decl;
            // Check arg mapping
            if args.len() != bind.len() {
                return Err(Error::ArgNumber { want: bind.len(), got: args.len() });
            }
            let Some(body) = body else {
                return Err(Error::NotDefined(*name));
            };
            // TODO: completely refactor data storage
            let mut frame = env.frame("fn args");
            for (Param { mutability: _, name }, value) in bind.iter().zip(args) {
                frame.insert(*name, Some(value.clone()));
            }
            match body.interpret(&mut frame) {
                Err(Error::Return(value)) => Ok(value),
                Err(Error::Break(value)) => Err(Error::BadBreak(value)),
                result => result,
            }
        }
    }
}

pub mod builtin;

pub mod env {
    //! Lexical and non-lexical scoping for variables
    use super::{
        builtin::{BINARY, MISC, RANGE, UNARY},
        convalue::ConValue,
        error::{Error, IResult},
        function::Function,
        BuiltIn, Callable, Interpret,
    };
    use cl_ast::{Function as FnDecl, Sym};
    use std::{
        collections::HashMap,
        fmt::Display,
        ops::{Deref, DerefMut},
    };

    type StackFrame = HashMap<Sym, Option<ConValue>>;

    /// Implements a nested lexical scope
    #[derive(Clone, Debug)]
    pub struct Environment {
        frames: Vec<(StackFrame, &'static str)>,
    }

    impl Display for Environment {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            for (frame, name) in self.frames.iter().rev() {
                writeln!(f, "--- {name} ---")?;
                for (var, val) in frame {
                    write!(f, "{var}: ")?;
                    match val {
                        Some(value) => writeln!(f, "\t{value}"),
                        None => writeln!(f, "<undefined>"),
                    }?
                }
            }
            Ok(())
        }
    }
    impl Default for Environment {
        fn default() -> Self {
            Self {
                frames: vec![
                    (to_hashmap(RANGE), "range ops"),
                    (to_hashmap(UNARY), "unary ops"),
                    (to_hashmap(BINARY), "binary ops"),
                    (to_hashmap(MISC), "builtins"),
                    (HashMap::new(), "globals"),
                ],
            }
        }
    }
    fn to_hashmap(from: &[&'static dyn BuiltIn]) -> HashMap<Sym, Option<ConValue>> {
        from.iter().map(|&v| (v.name(), Some(v.into()))).collect()
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
        pub fn call(&mut self, name: Sym, args: &[ConValue]) -> IResult<ConValue> {
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
        pub fn get_mut(&mut self, id: Sym) -> IResult<&mut Option<ConValue>> {
            for (frame, _) in self.frames.iter_mut().rev() {
                if let Some(var) = frame.get_mut(&id) {
                    return Ok(var);
                }
            }
            Err(Error::NotDefined(id))
        }
        /// Resolves a variable immutably.
        ///
        /// Returns a reference to the variable's contents, if it is defined and initialized.
        pub fn get(&self, id: Sym) -> IResult<ConValue> {
            for (frame, _) in self.frames.iter().rev() {
                match frame.get(&id) {
                    Some(Some(var)) => return Ok(var.clone()),
                    Some(None) => return Err(Error::NotInitialized(id)),
                    _ => (),
                }
            }
            Err(Error::NotDefined(id))
        }
        /// Inserts a new [ConValue] into this [Environment]
        pub fn insert(&mut self, id: Sym, value: Option<ConValue>) {
            if let Some((frame, _)) = self.frames.last_mut() {
                frame.insert(id, value);
            }
        }
        /// A convenience function for registering a [FnDecl] as a [Function]
        pub fn insert_fn(&mut self, decl: &FnDecl) {
            let FnDecl { name, .. } = decl;
            let (name, function) = (name, Some(Function::new(decl).into()));
            if let Some((frame, _)) = self.frames.last_mut() {
                frame.insert(*name, function);
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

    use cl_ast::Sym;

    use super::convalue::ConValue;

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
        /// A value could not be indexed
        NotIndexable,
        /// An array index went out of bounds
        OobIndex(usize, usize),
        /// An expression is not assignable
        NotAssignable,
        /// A name was not defined in scope before being used
        NotDefined(Sym),
        /// A name was defined but not initialized
        NotInitialized(Sym),
        /// A value was called, but is not callable
        NotCallable(ConValue),
        /// A function was called with the wrong number of arguments
        ArgNumber {
            want: usize,
            got: usize,
        },
        Outlined(Sym),
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
                Error::NotIndexable => {
                    write!(f, "expression cannot be indexed")
                }
                Error::OobIndex(idx, len) => {
                    write!(f, "Index out of bounds: index was {idx}. but len is {len}")
                }
                Error::NotAssignable => {
                    write!(f, "expression is not assignable")
                }
                Error::NotDefined(value) => {
                    write!(f, "{value} not bound. Did you mean `let {value};`?")
                }
                Error::NotInitialized(value) => {
                    write!(f, "{value} bound, but not initialized")
                }
                Error::NotCallable(value) => {
                    write!(f, "{value} is not callable.")
                }
                Error::ArgNumber { want, got } => {
                    write!(
                        f,
                        "Expected {want} argument{}, got {got}",
                        if *want == 1 { "" } else { "s" }
                    )
                }
                Error::Outlined(name) => {
                    write!(f, "Module {name} specified, but not imported.")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
