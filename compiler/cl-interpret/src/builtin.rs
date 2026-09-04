//! Interpreter [Builtin] functions, and [builtin] macro to define them
#![allow(non_upper_case_globals)]

use cl_ast::types::Symbol;

use crate::{
    Callable,
    convalue::ConValue,
    env::Environment,
    error::{Error, ErrorKind, IResult},
    place::Place,
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

impl std::fmt::Display for Builtin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.desc)
    }
}

impl super::Callable for Builtin {
    fn call(&self, interpreter: &mut Environment, args: &[ConValue]) -> IResult<ConValue> {
        (self.func)(interpreter, args)
    }

    fn name(&self) -> Option<Symbol> {
        Some(self.name.into())
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
///             [] => Ok(ConValue::Unit),
///             _ => my_builtin(env, rest), // Can be called as a normal function!
///         }
///     }
/// };
/// ```
pub macro builtin(
    $(#[doc = $($docs:tt)*])*
    fn $name:ident ($($arg:pat),*$(,)?) $(@$env:tt)? $body:block
) {{
    builtin_body!($(#[$($meta)*])* fn $name ($($args)*) $(@$env)? $body);
    builtin_define!($(#[$($meta)*])* fn $name ($($args)*) $(@$env)? $body)
}}

/// Constructs an array of [Builtin]s from pseudo-function definitions.
///
/// Unlike [builtin], functions defined in this way can be mutually recursive.
pub macro builtins($(
    $(#[$($meta:tt)*])*
    fn $name:ident ($($args:tt)*) $(@$env:tt)? $body:block
)*) {{
    $(builtin_body!($(#[$($meta)*])* fn $name ($($args)*) $(@$env)? $body);)*
    [$(builtin_define!($(#[$($meta)*])* fn $name ($($args)*) $(@$env)? $body)),*]
}}

/// Constructs the Rust-native function portion of a [builtin] (or [builtins])
macro builtin_body(
    $(#[doc = $($docs:tt)*])*
    fn $name:ident ($($arg:pat),*$(,)?) $(@$env:tt)? $body:block
) {
    $(#[doc = $($docs)*])*
    fn $name(_env: &mut Environment, _args: &[ConValue]) -> IResult<ConValue> {
        // Set up the builtin! environment
        $(#[allow(unused)]let $env = _env;)?
        // Allow for single argument `fn foo(args @ ..)` pattern
        #[allow(clippy::redundant_at_rest_pattern, irrefutable_let_patterns)]
        let [$($arg),*] = _args else {
            Err($crate::error::Error::TypeError(
                concat!("(", $(stringify!($arg,),)* ")"),
                $crate::typeinfo::Model::Any.intern()
            ))?
        };
        $body.map(Into::into)
    }
}

/// Constructs the [Builtin] object portion of a [builtin] (or [builtins])
macro builtin_define(
    $(#[doc = $($docs:tt)*])*
    fn $name:ident ($($arg:pat),*$(,)?) $(@$env:tt)? $body:block
) {
    Builtin {
        name: stringify!($name),
        desc: concat![
            $("///", $($docs,)* "\n",)*
            stringify!(builtin fn $name($($arg),*))
        ],
        func: &$name,
    }
}

/// Creates an [Error::BuiltinError] using interpolation of runtime expressions.
/// See [std::format].
pub macro error_format ($($t:tt)*) {
    $crate::error::Error::BuiltinError(format!($($t)*))
}

pub const Builtins: &[Builtin] = &builtins![
    /// Unstable variadic format function
    fn fmt(args @ ..) @env {
        use std::fmt::Write;
        let mut out = String::new();

        for mut arg in args.iter() {
            while let ConValue::Ref(r) = arg {
                arg = r.get(env)?;
            }
            if let Err(e) = write!(out, "{arg}") {
                eprintln!("{e}");
            }
        }
        Ok(out)
    }

    /// Prints the arguments in-order, with no separators
    fn print(args @ ..) @env {
        let mut out = stdout().lock();
        for mut arg in args.iter() {
            while let ConValue::Ref(r) = arg {
                arg = r.get(env)?;
            }
            write!(out, "{arg}").ok();
        }
        Ok(())
    }

    /// Prints the arguments in-order, followed by a newline
    fn println(args @ ..) @env {
        let mut out = stdout().lock();
        for mut arg in args.iter() {
            while let ConValue::Ref(r) = arg {
                arg = r.get(env)?;
            }
            write!(out, "{arg}").ok();
        }
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

    /// Binds an arbitrary string `name` to the given `value`,
    /// bypassing identifiers entirely.
    fn bind(ConValue::Str(name), value) @env {
        env.bind(*name, value.clone());
        Ok(())
    }

    /// Constructs a reference from a raw integer
    fn raw_ref(ConValue::Int(index)) {
        Ok(ConValue::Ref(Place::from_index(*index as _)))
    }

    /// Panics, with a message created by formatting `args` (as if by `fmt`)
    fn panic(args @ ..) @env {
        use std::fmt::Write;
        let mut stdout = stdout().lock();
        let mut out = String::from("Explicit panic: ");
        if let Err(e) = args.iter().try_for_each(|arg| write!(out, "{arg}")) {
            writeln!(stdout, "{e}").ok();
        }
        writeln!(stdout, "{out}");
        Err(Error::Panic(out))?;
        Ok(())
    }

    fn todo(args @ ..) @env {
        use std::fmt::Write;
        let mut stdout = stdout().lock();
        let mut out = String::from("Not yet implemented: ");
        if let Err(e) = args.iter().try_for_each(|arg| write!(out, "{arg}")) {
            writeln!(stdout, "{e}").ok();
        }
        writeln!(stdout, "{out}");
        Err(Error::Panic(out))?;
        Ok(())
    }

    /// Dumps the environment
    fn dump() @env {
        println!("{env}");
        Ok(())
    }

    fn backtrace() @env {
        println!("Backtrace:\n{}", env.backtrace());
        Ok(())
    }

    fn host_backtrace() {
        println!("Host backtrace:\n{}", std::backtrace::Backtrace::force_capture());
        Ok(())
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

    /// Returns the length of the input list as a [ConValue::Int]
    fn len(list) @env {
        Ok(match list.dereference_in(env)? {
            ConValue::Unit => 0,
            ConValue::Str(s) => s.chars().count() as _,
            ConValue::String(s) => s.chars().count() as _,
            &ConValue::Slice(_, start, end) => end as i128 - start as i128,
            ConValue::Array(arr) => arr.len() as _,
            ConValue::Tuple(t) => t.len() as _,
            other => Err(Error::TypeError("A type with a length", other.type_of()))?,
        })
    }

    fn slice(ConValue::Ref(index), ConValue::Int(start), ConValue::Int(end)) {
        match (start, end) {
            (0.., 0..) if start <= end => Ok(ConValue::Slice(index.clone(), *start as _, (end - start) as _)),
            _ => Err(Error::BuiltinError(format_args!("Bad index: {index}[{start}, {end}]")))
        }
    }

    /// Pushes an `item` onto the top of an array (by reference)
    fn push(ConValue::Ref(array), item) @env{
        let mut index = array.get_mut(env)?;
        while let ConValue::Ref(r) = index {
            index = r.clone().get_mut(env)?;
        }
        let ConValue::Array(v) = index else {
            Err(Error::TypeError("An array", index.type_of()))?
        };

        let mut items = std::mem::take(v).into_vec();
        items.push(item.clone());
        *v = items.into_boxed_slice();

        Ok(ConValue::Unit)
    }

    /// Pops an item off the top of an array (by reference)
    fn pop(ConValue::Ref(array)) @env {
        let v = match array.get_mut(env)? {
            ConValue::Array(v) => v,
            other => Err(Error::TypeError("An array", other.type_of()))?,
        };

        let mut items = std::mem::take(v).into_vec();
        let out = items.pop().unwrap_or(ConValue::Unit);
        *v = items.into_boxed_slice();

        Ok(out)
    }

    /// Converts `string` into an array of chars
    fn chars(string) @env {
        Ok(match string.dereference_in(env)? {
            ConValue::Str(s) => ConValue::Array(s.chars().map(Into::into).collect()),
            ConValue::String(s) => ConValue::Array(s.chars().map(Into::into).collect()),
            _ => Err(Error::TypeError("string", string.type_of()))?,
        })
    }

    /// Invokes a function with the given arguments
    fn invoke(function, args) @env {
        match args {
            ConValue::Unit => function.call(env, &[]),
            ConValue::Array(args) | ConValue::Tuple(args) => function.call(env, args),
            _ => function.call(env, std::slice::from_ref(args)),
        }
    }

    fn dump_symbols() {
        println!("{}", cl_structures::intern::string_interner::StringInterner::global());
        Ok(ConValue::Empty)
    }

    fn catch_panic(lambda, args @ ..) @env {
        match lambda.call(env, args) {
            Err(Error { kind: ErrorKind::Panic(e, ..), ..}) => {
                println!("Caught panic!");
                Ok(ConValue::String(e))
            },
            other => other,
        }
    }

    /// Returns a shark
    fn shark() {
        Ok('\u{1f988}')
    }
];

pub const Math: &[Builtin] = &builtins![
    /// Multiplication `a * b`
    fn mul(lhs, rhs) { lhs.clone() * rhs.clone() }

    /// Division `a / b`
    fn div(lhs, rhs) { lhs.clone() / rhs.clone() }

    /// Remainder `a % b`
    fn rem(lhs, rhs) { lhs.clone() % rhs.clone() }

    /// Addition `a + b`
    fn add(lhs, rhs) { lhs.clone() + rhs.clone() }

    /// Subtraction `a - b`
    fn sub(lhs, rhs) { lhs.clone() - rhs.clone() }

    /// Shift Left `a << b`
    fn shl(lhs, rhs) { lhs.clone() << rhs.clone() }

    /// Shift Right `a >> b`
    fn shr(lhs, rhs) { lhs.clone() >> rhs.clone() }

    /// Bitwise And `a & b`
    fn and(lhs, rhs) { lhs.clone() & rhs.clone() }

    /// Bitwise Or `a | b`
    fn or(lhs, rhs) { lhs.clone() | rhs.clone() }

    /// Bitwise Exclusive Or `a ^ b`
    fn xor(lhs, rhs) { lhs.clone() ^ rhs.clone() }

    /// Negates the ConValue
    fn neg(tail) {
        Ok(match tail {
            ConValue::Unit => ConValue::Unit,
            ConValue::Int(v) => ConValue::Int(-v),
            ConValue::Float(v) => ConValue::Float(-v),
            _ => Err(Error::TypeError("type implements Neg", tail.type_of()))?,
        })
    }

    /// Inverts the ConValue
    fn not(tail) {
        Ok(match tail {
            ConValue::Unit => ConValue::Unit,
            ConValue::Int(v) => ConValue::Int(!v),
            ConValue::Bool(v) => ConValue::Bool(!v),
            _ => Err(Error::TypeError("type implements Not", tail.type_of()))?,
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
        Ok(tail.dereference_in(env)?.clone())
    }

    fn f64_to_bits(float) @env {
        match float.dereference_in(env)? {
            &ConValue::Float(f) => Ok(ConValue::Int(f.to_bits() as _)),
            other => Err(error_format!("Cannot convert {other} from float to bits")),
        }
    }

    fn f64_from_bits(bits) @env {
        match bits.dereference_in(env)? {
            &ConValue::Int(i) => Ok(ConValue::Float(f64::from_bits(i as u64))),
            other => Err(error_format!("Cannot convert {other} from float to bits")),
        }
    }

    /// Twiddles bits into floats, or vice versa
    fn float(bits) @env {
        match bits.dereference_in(env)? {
            &ConValue::Float(f) => Ok(ConValue::Int(f.to_bits() as _)),
            &ConValue::Int(i) => Ok(ConValue::Float(f64::from_bits(i as u64))),
            other => Err(error_format!("Cannot convert {other} to/from float/bits")),
        }
    }
];
