#![allow(non_upper_case_globals)]

use crate::{
    convalue::ConValue,
    env::Environment,
    error::{Error, IResult},
};
use std::io::{Write, stdout};

/// A function built into the interpreter.
#[derive(Clone, Copy)]
pub struct Builtin {
    /// An identifier to be used during registration
    pub name: &'static str,
    /// The signature, displayed when the builtin is printed
    pub desc: &'static str,
    /// The function to be run when called
    pub func: &'static dyn Fn(&mut Environment, &[ConValue]) -> IResult<ConValue>,
}

impl Builtin {
    /// Constructs a new Builtin
    pub const fn new(
        name: &'static str,
        desc: &'static str,
        func: &'static impl Fn(&mut Environment, &[ConValue]) -> IResult<ConValue>,
    ) -> Builtin {
        Builtin { name, desc, func }
    }

    pub const fn description(&self) -> &'static str {
        self.desc
    }
}

impl std::fmt::Debug for Builtin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Builtin")
            .field("description", &self.desc)
            .finish_non_exhaustive()
    }
}

impl super::Callable for Builtin {
    fn call(&self, interpreter: &mut Environment, args: &[ConValue]) -> IResult<ConValue> {
        (self.func)(interpreter, args)
    }

    fn name(&self) -> cl_ast::Sym {
        self.name.into()
    }
}

/// Turns a function definition into a [Builtin].
///
/// ```rust
/// # use cl_interpret::{builtin::builtin, convalue::ConValue};
/// let my_builtin = builtin! {
///     /// Use the `@env` suffix to bind the environment!
///     /// (needed for recursive calls)
///     fn my_builtin(ConValue::Bool(b), rest @ ..) @env {
///         // This is all Rust code!
///         eprintln!("my_builtin({b}, ..)");
///         match rest {
///             [] => Ok(ConValue::Empty),
///             _ => my_builtin(env, rest), // Can be called as a normal function!
///         }
///     }
/// };
/// ```
pub macro builtin(
    $(#[$($meta:tt)*])*
    fn $name:ident ($($arg:pat),*$(,)?) $(@$env:tt)? $body:block
) {{
    $(#[$($meta)*])*
    fn $name(_env: &mut Environment, _args: &[ConValue]) -> IResult<ConValue> {
        // Set up the builtin! environment
        $(#[allow(unused)]let $env = _env;)?
        // Allow for single argument `fn foo(args @ ..)` pattern
        #[allow(clippy::redundant_at_rest_pattern, irrefutable_let_patterns)]
        let [$($arg),*] = _args else {
            Err($crate::error::Error::TypeError())?
        };
        $body.map(Into::into)
    }
    Builtin {
        name: stringify!($name),
        desc: stringify![builtin fn $name($($arg),*)],
        func: &$name,
    }
}}

/// Constructs an array of [Builtin]s from pseudo-function definitions
pub macro builtins($(
    $(#[$($meta:tt)*])*
    fn $name:ident ($($args:tt)*) $(@$env:tt)? $body:block
)*) {
    [$(builtin!($(#[$($meta)*])* fn $name ($($args)*) $(@$env)? $body)),*]
}

/// Creates an [Error::BuiltinError] using interpolation of runtime expressions.
/// See [std::format].
pub macro error_format ($($t:tt)*) {
    $crate::error::Error::BuiltinError(format!($($t)*))
}

pub const Builtins: &[Builtin] = &builtins![
    /// Unstable variadic format function
    fn fmt(args @ ..) {
        use std::fmt::Write;
        let mut out = String::new();
        if let Err(e) = args.iter().try_for_each(|arg| write!(out, "{arg}")) {
            eprintln!("{e}");
        }
        Ok(out)
    }

    /// Prints the arguments in-order, with no separators
    fn print(args @ ..) {
        let mut out = stdout().lock();
        args.iter().try_for_each(|arg| write!(out, "{arg}") ).ok();
        Ok(())
    }

    /// Prints the arguments in-order, followed by a newline
    fn println(args @ ..) {
        let mut out = stdout().lock();
        args.iter().try_for_each(|arg| write!(out, "{arg}") ).ok();
        writeln!(out).ok();
        Ok(())
    }

    /// Debug-prints the argument, returning a copy
    fn dbg(arg) {
        println!("{arg:?}");
        Ok(arg.clone())
    }

    /// Debug-prints the argument
    fn dbgp(args @ ..) {
        let mut out = stdout().lock();
        args.iter().try_for_each(|arg| writeln!(out, "{arg:#?}") ).ok();
        Ok(())
    }

    fn panic(message) {
        Err(error_format!("Panic: {message}"))?;
        Ok(())
    }

    /// Dumps the environment
    fn dump() @env {
        println!("{env}");
        Ok(())
    }

    /// Gets all global variables in the environment
    fn globals() @env {
        let globals = env.globals();
        Ok(ConValue::Slice(globals.base, globals.binds.len()))
    }

    fn builtins() @env {
        let len = env.globals().binds.len();
        for builtin in 0..len {
            if let Some(value @ ConValue::Builtin(_)) = env.get_id(builtin) {
                println!("{builtin}: {value}")
            }
        }
        Ok(())
    }

    fn alloca(ConValue::Int(len)) @env {
        Ok(env.alloca(ConValue::Empty, *len as usize))
    }

    /// Returns the length of the input list as a [ConValue::Int]
    fn len(list) @env {
        Ok(match list {
            ConValue::Empty => 0,
            ConValue::Str(s) => s.chars().count() as _,
            ConValue::String(s) => s.chars().count() as _,
            ConValue::Ref(r) => {
                return len(env, &[env.get_id(*r).ok_or(Error::StackOverflow(*r))?.clone()])
            }
            ConValue::Slice(_, len) => *len as _,
            ConValue::Array(arr) => arr.len() as _,
            ConValue::Tuple(t) => t.len() as _,
            _ => Err(Error::TypeError())?,
        })
    }

    fn push(ConValue::Ref(index), item) @env{
        let Some(ConValue::Array(v)) = env.get_id_mut(*index) else {
            Err(Error::TypeError())?
        };

        let mut items = std::mem::take(v).into_vec();
        items.push(item.clone());
        *v = items.into_boxed_slice();

        Ok(ConValue::Empty)
    }

    fn chars(string) @env {
        Ok(match string {
            ConValue::Str(s) => ConValue::Array(s.chars().map(Into::into).collect()),
            ConValue::String(s) => ConValue::Array(s.chars().map(Into::into).collect()),
            ConValue::Ref(r) => {
                return chars(env, &[env.get_id(*r).ok_or(Error::StackOverflow(*r))?.clone()])
            }
            _ => Err(Error::TypeError())?,
        })
    }

    fn dump_symbols() {
        println!("{}", cl_structures::intern::string_interner::StringInterner::global());
        Ok(ConValue::Empty)
    }

    fn slice_of(ConValue::Ref(arr), ConValue::Int(start)) {
        Ok(ConValue::Slice(*arr, *start as usize))
    }

    /// Returns a shark
    fn shark() {
        Ok('\u{1f988}')
    }
];

pub const Math: &[Builtin] = &builtins![
    /// Multiplication `a * b`
    fn mul(lhs, rhs) {
        Ok(match (lhs, rhs) {
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a * b),
            _ => Err(Error::TypeError())?
        })
    }

    /// Division `a / b`
    fn div(lhs, rhs) {
        Ok(match (lhs, rhs){
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a / b),
            _ => Err(Error::TypeError())?
        })
    }

    /// Remainder `a % b`
    fn rem(lhs, rhs) {
        Ok(match (lhs, rhs) {
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a % b),
            _ => Err(Error::TypeError())?,
        })
    }

    /// Addition `a + b`
    fn add(lhs, rhs) {
        Ok(match (lhs, rhs) {
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a + b),
            (ConValue::Str(a), ConValue::Str(b)) => (a.to_string() + b).into(),
            (ConValue::Str(a), ConValue::String(b)) => (a.to_string() + b).into(),
            (ConValue::String(a), ConValue::Str(b)) => (a.to_string() + b).into(),
            (ConValue::String(a), ConValue::String(b)) => (a.to_string() + b).into(),
            (ConValue::Str(s), ConValue::Char(c)) => { let mut s = s.to_string(); s.push(*c); s.into() }
            (ConValue::String(s), ConValue::Char(c)) => { let mut s = s.to_string(); s.push(*c); s.into() }
            (ConValue::Char(a), ConValue::Char(b)) => {
                ConValue::String([a, b].into_iter().collect::<String>())
            }
            _ => Err(Error::TypeError())?
        })
    }

    /// Subtraction `a - b`
    fn sub(lhs, rhs) {
        Ok(match (lhs, rhs) {
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a - b),
            _ => Err(Error::TypeError())?,
        })
    }

    /// Shift Left `a << b`
    fn shl(lhs, rhs) {
        Ok(match (lhs, rhs) {
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a << b),
            _ => Err(Error::TypeError())?,
        })
    }

    /// Shift Right `a >> b`
    fn shr(lhs, rhs) {
        Ok(match (lhs, rhs) {
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a >> b),
            _ => Err(Error::TypeError())?,
        })
    }

    /// Bitwise And `a & b`
    fn and(lhs, rhs) {
        Ok(match (lhs, rhs) {
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a & b),
            (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a & b),
            _ => Err(Error::TypeError())?,
        })
    }

    /// Bitwise Or `a | b`
    fn or(lhs, rhs) {
        Ok(match (lhs, rhs) {
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a | b),
            (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a | b),
            _ => Err(Error::TypeError())?,
        })
    }

    /// Bitwise Exclusive Or `a ^ b`
    fn xor(lhs, rhs) {
        Ok(match (lhs, rhs) {
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a ^ b),
            (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a ^ b),
            _ => Err(Error::TypeError())?,
        })
    }

    #[allow(non_snake_case)]
    fn RangeExc(start, end) @env {
        Ok(ConValue::TupleStruct(Box::new((
            "RangeExc", Box::new([start.clone(), end.clone()])
        ))))
    }

    #[allow(non_snake_case)]
    fn RangeInc(start, end) @env {
        Ok(ConValue::TupleStruct(Box::new((
            "RangeInc", Box::new([start.clone(), end.clone()])
        ))))
    }

    #[allow(non_snake_case)]
    fn RangeTo(end) @env {
        Ok(ConValue::TupleStruct(Box::new((
            "RangeTo", Box::new([end.clone()])
        ))))
    }

    #[allow(non_snake_case)]
    fn RangeToInc(end) @env {
        Ok(ConValue::TupleStruct(Box::new((
            "RangeToInc", Box::new([end.clone()])
        ))))
    }

    /// Negates the ConValue
    fn neg(tail) {
        Ok(match tail {
            ConValue::Empty => ConValue::Empty,
            ConValue::Int(v) => ConValue::Int(v.wrapping_neg()),
            ConValue::Float(v) => ConValue::Float(-v),
            _ => Err(Error::TypeError())?,
        })
    }

    /// Inverts the ConValue
    fn not(tail) {
        Ok(match tail {
            ConValue::Empty => ConValue::Empty,
            ConValue::Int(v) => ConValue::Int(!v),
            ConValue::Bool(v) => ConValue::Bool(!v),
            _ => Err(Error::TypeError())?,
        })
    }

    /// Compares two values
    fn cmp(head, tail) {
        Ok(ConValue::Int(match (head, tail) {
            (ConValue::Int(a), ConValue::Int(b)) => a.cmp(b) as _,
            (ConValue::Bool(a), ConValue::Bool(b)) => a.cmp(b) as _,
            (ConValue::Char(a), ConValue::Char(b)) => a.cmp(b) as _,
            (ConValue::Str(a), ConValue::Str(b)) => a.cmp(b) as _,
            (ConValue::Str(a), ConValue::String(b)) => a.to_ref().cmp(b.as_str()) as _,
            (ConValue::String(a), ConValue::Str(b)) => a.as_str().cmp(b.to_ref()) as _,
            (ConValue::String(a), ConValue::String(b)) => a.cmp(b) as _,
            _ => Err(error_format!("Incomparable values: {head}, {tail}"))?
        }))
    }

    /// Does the opposite of `&`
    fn deref(tail) @env {
        Ok(match tail {
            ConValue::Ref(v) => env.get_id(*v).cloned().ok_or(Error::StackOverflow(*v))?,
            _ => tail.clone(),
        })
    }
];
