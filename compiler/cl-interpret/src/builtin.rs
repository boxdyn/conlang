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
use std::{
    io::{Write, stdout},
    vec,
};

/// A function built into the interpreter.
#[derive(Clone, Copy)]
pub struct Builtin {
    /// An identifier to be used during registration
    pub name: &'static str,
    /// The signature, displayed when the builtin is printed
    pub desc: &'static str,
    /// The function to be run when called
    pub func: &'static dyn Fn(&mut Environment, Vec<ConValue>) -> IResult<ConValue>,
}

impl Builtin {
    /// Constructs a new Builtin
    pub const fn new(
        name: &'static str,
        desc: &'static str,
        func: &'static impl Fn(&mut Environment, Vec<ConValue>) -> IResult<ConValue>,
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
    fn call(&self, interpreter: &mut Environment, args: Vec<ConValue>) -> IResult<ConValue> {
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
/// Functions defined in this way can be mutually recursive.
///
/// ```rust
/// # use cl_interpret::{builtin::builtins, convalue::ConValue};
/// let my_builtins = builtins! {
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
pub macro builtins($(
    $(#[$($meta:tt)*])*
    fn $name:ident ($($args:tt)*) $(@$env:tt)? $(-> $rety:ty)? $body:block
)*) {{
    $(builtin_body!($(#[$($meta)*])* fn $name ($($args)*) $(@$env)? $(-> $rety)? $body);)*
    [$(builtin_define!($(#[$($meta)*])* fn $name ($($args)*) $(@$env)? $(-> $rety)? $body)),*]
}}

/// Constructs the Rust-native function portion of a [builtin] (or [builtins])
macro builtin_body(
    $(#[doc = $($docs:tt)*])*
    fn $name:ident ($($arg:pat),*$(,)?) $(@$env:tt)? $(-> $rety:ty)? $body:block
) {
    $(#[doc = $($docs)*])*
    /// ```conlang
    #[doc = stringify!(builtin fn $name($($arg),*) $(-> $rety)?)]
    /// ```
    fn $name(_env: &mut Environment, mut _args: Vec<ConValue>) -> IResult<ConValue> {
        // Set up the builtin! environment
        $(#[allow(unused)]let $env = _env;)?
        // Allow for single argument `fn foo(args @ ..)` pattern
        #[allow(clippy::redundant_at_rest_pattern, irrefutable_let_patterns)]
        let [$($arg),*] = _args.as_mut_slice() else {
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
    fn $name:ident ($($arg:pat),*$(,)?) $(@$env:tt)? $(-> $rety:ty)? $body:block
) {
    Builtin {
        name: stringify!($name),
        desc: concat![
            $("///", $($docs,)* "\n",)*
            stringify!(builtin fn $name($($arg),*) $(-> $rety)?)
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
    fn fmt(args @ ..) @env -> str {
        use std::fmt::Write;
        let mut out = String::new();

        for arg in args.iter() {
            if let Err(e) = write!(out, "{}", arg.dereference_in(env)?) {
                eprintln!("{e}");
            }
        }
        Ok(out)
    }

    /// Prints the arguments in-order, with no separators
    fn print(args @ ..) @env {
        let mut out = stdout().lock();
        for arg in args.iter() {
            write!(out, "{}", arg.dereference_in(env)?).ok();
        }
        Ok(())
    }

    /// Prints the arguments in-order, followed by a newline
    fn println(args @ ..) @env {
        let mut out = stdout().lock();
        for arg in args.iter() {
            write!(out, "{}", arg.dereference_in(env)?).ok();
        }
        writeln!(out).ok();
        Ok(())
    }

    /// Debug-prints the argument, returning a copy
    fn dbg(arg) @env -> arg {
        println!("{:?}", arg.dereference_in(env)?);
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
    fn bind(name, value) @env {
        let name = Symbol::from(get_str(env, name)?);
        env.bind(name, value.clone());
        Ok(())
    }

    /// Constructs a reference from a raw integer
    fn raw_ref(ConValue::Int(index)) -> &_ {
        Ok(ConValue::Ref(Place::from_index(*index as _)))
    }

    /// Panics, with a message created by formatting `args` (as if by `fmt`)
    fn panic(args @ ..) @env -> ! {
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

    fn todo(args @ ..) @env -> ! {
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
    fn len(list) @env -> i128 {
        Ok(match list.dereference_in(env)? {
            ConValue::Unit => 0,
            ConValue::Str(s) => s.chars().count() as _,
            ConValue::String(s) => s.chars().count() as _,
            &ConValue::Slice(_, start, end) => end as i128 - start as i128,
            ConValue::Array(arr) => arr.len() as _,
            ConValue::Tuple(t) => t.len() as _,
            other => Err(Error::TypeError("A type with a length", other.type_of(env)))?,
        })
    }

    fn slice(ConValue::Ref(index), ConValue::Int(start), ConValue::Int(end)) -> [_] {
        match (*start, *end) {
            (0.., 0..) if start <= end => Ok(ConValue::Slice(index.clone(), *start as _, (*end - *start) as _)),
            _ => Err(error_format!("Bad index: {index}[{start}, {end}]"))
        }
    }

    /// Converts `string` into an array of chars
    fn chars(string) @env -> [char] {
        Ok(match string.dereference_in(env)? {
            ConValue::Str(s) => ConValue::Array(s.chars().map(Into::into).collect()),
            ConValue::String(s) => ConValue::Array(s.chars().map(Into::into).collect()),
            _ => Err(Error::TypeError("str", string.type_of(env)))?,
        })
    }

    /// Invokes a function with the given arguments
    fn invoke(function, args) @env -> R {
        match args.take() {
            ConValue::Unit => function.call(env, vec![]),
            ConValue::Array(args) | ConValue::Tuple(args) => function.call(env, args.into_vec()),
            value => function.call(env, vec![value]),
        }
    }

    /// Dumps all interned Symbols
    fn symbols() {
        println!("{}", cl_structures::intern::string_interner::StringInterner::global());
        Ok(ConValue::Unit)
    }

    /// Gets the underlying `mod` for type `ty`
    fn module(ty) @env -> Module {
        let ty = ty.dereference_in(env)?;
        let ty = match ty {
            ConValue::TypeInfo(ty) => *ty,
            other => other.type_of(env),
        };
        let Some(impls) = env.impls.get(&ty) else {
            return Ok(ConValue::Module(Default::default()));
        };
        Ok(ConValue::Module(Box::new(
            impls.iter().map(|(&k, v)| (k.into(), v.clone())).collect()
        )))
    }

    fn mod_into_binds(module) @env -> [(str, _)] {
        let ConValue::Module(m) = module.dereference_in(env)? else {
            return Err(Error::TypeError("mod", module.type_of(env)))
        };

        Ok(ConValue::Array(
            m.iter()
                .map(|(&k, v)| ConValue::Tuple([ConValue::Str(k), v.clone()].into()))
                .collect()
        ))
    }

    fn type_of(value) @env -> Type {
        Ok(value.dereference_in(env)?.type_of(env))
    }

    /// Gets the underlying `mod` for type `ty`
    fn captures(func) @env -> [str] {
        let ConValue::Function(func) = func.dereference_in(env)? else {
            return Err(Error::TypeError("fn", func.type_of(env)))
        };
        Ok(ConValue::Array(
            func.captures()
                .iter()
                .copied()
                .map(ConValue::Str)
                .collect()
        ))
    }

    /// Gets the underlying `mod` for type `ty`
    fn upvars(func) @env -> Module {
        let ConValue::Function(func) = func.dereference_in(env)? else {
            return Err(Error::TypeError("fn", func.type_of(env)))
        };
        Ok(ConValue::Module(Box::new(
            func.upvars().borrow().0.iter().map(|(&name, &idx)| {
                (name, env.get_id(idx).cloned().unwrap_or_default())
            }).collect()
        )))
    }

    /// Executes the provided `lambda` with `args`, and halts stack unwinding
    fn catch_panic(lambda, args @ ..) @env -> Result<_, _> {
        let out = match lambda.call(env, args.to_vec()) {
            Err(Error { kind: ErrorKind::Panic(e, ..), ..}) => {
                println!("Caught panic!");
                Err(ConValue::String(e))
            },
            other => Ok(other?),
        };

        env.to_convalue_result(out)
    }

    /// Returns a shark
    fn shark() -> char {
        Ok('\u{1f988}')
    }
];

pub const Math: &[Builtin] = &builtins![
    /// Multiplication `a * b`
    fn mul(lhs, rhs) @env -> Self { lhs.clone().mul(rhs.clone(), env) }

    /// Division `a / b`
    fn div(lhs, rhs) @env -> Self { lhs.clone().div(rhs.clone(), env) }

    /// Remainder `a % b`
    fn rem(lhs, rhs) @env -> Self { lhs.clone().rem(rhs.clone(), env) }

    /// Addition `a + b`
    fn add(lhs, rhs) @env -> Self { lhs.clone().add(rhs.clone(), env) }

    /// Subtraction `a - b`
    fn sub(lhs, rhs) @env -> Self { lhs.clone().sub(rhs.clone(), env) }

    /// Shift Left `a << b`
    fn shl(lhs, rhs) @env -> Self { lhs.clone().shl(rhs.clone(), env) }

    /// Shift Right `a >> b`
    fn shr(lhs, rhs) @env -> Self { lhs.clone().shr(rhs.clone(), env) }

    /// Bitwise And `a & b`
    fn and(lhs, rhs) @env -> Self { lhs.clone().and(rhs.clone(), env) }

    /// Bitwise Or `a | b`
    fn or(lhs, rhs) @env -> Self { lhs.clone().or(rhs.clone(), env) }

    /// Bitwise Exclusive Or `a ^ b`
    fn xor(lhs, rhs) @env -> Self { lhs.clone().xor(rhs.clone(), env) }

    /// Negates the ConValue
    fn neg(tail) @env -> Self { tail.clone().neg(env) }

    /// Inverts the ConValue
    fn not(tail) @env -> Self { tail.clone().not(env) }

    /// Compares two values
    fn cmp(head, tail) @env {
        let head = head.dereference_in(env)?;
        let tail = tail.dereference_in(env)?;
        Ok(head.compare(tail, env)? as i128)
    }

    /// Does the opposite of `&`
    fn deref(tail) @env -> _ {
        Ok(tail.dereference_in(env)?.clone())
    }

    fn get_time_micros() @env -> i128 {
        std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|dur| ConValue::Int(dur.as_micros() as _))
            .map_err(|e| error_format!("{e}"))
    }
];

pub const IntIntrinsics: &[Builtin] = &builtins! {
    /// Computes `int` to the `power`th power
    fn pow(int, power) @env -> i128 {
        Ok(get_int(env, int, "i128")?.wrapping_pow(get_int(env, power, "i128")? as _))
    }
    /// Computes the square root of `int`
    fn isqrt(int) @env -> i128 {
        Ok(get_int(env, int, "i128")?.isqrt())
    }
    /// Counts the number of one-bits in `int`
    fn count_ones(int) @env -> i128 {
        Ok(get_int(env, int, "i128")?.count_ones() as i128)
    }
    /// Counts the number of zero-bits in `int`
    fn count_zeros(int) @env -> i128 {
        Ok(get_int(env, int, "i128")?.count_zeros() as i128)
    }
    /// Counts the number of leading zeroes in `int`
    fn leading_zeros(int) @env -> i128 {
        Ok(get_int(env, int, "i128")?.leading_zeros() as i128)
    }
    /// Counts the number of trailing zeroes in `int`
    fn trailing_zeros(int) @env -> i128 {
        Ok(get_int(env, int, "i128")?.trailing_zeros() as i128)
    }
    /// Swaps the bytes of `int` (as i128)
    fn swap_bytes(int) @env -> i128 {
        Ok(get_int(env, int, "i128")?.swap_bytes() as i128)
    }
    /// Reverses the bits of `int` (as i128)
    fn reverse_bits(int) @env -> i128 {
        Ok(get_int(env, int, "i128")?.reverse_bits() as i128)
    }
    /// Reverses the bits of `int` (as i128)
    fn abs(int) @env -> i128 {
        Ok(get_int(env, int, "i128")?.wrapping_abs() as i128)
    }
    /// Returns the absolute difference between `int` and `other`
    fn abs_diff(int, other) @env -> i128 {
        let other = get_int(env, other, "i128")?;
        Ok(get_int(env, int, "i128")?.abs_diff(other) as i128)
    }
    /// Returns the logarithm of `int` with respect to `base`, rounded down
    fn ilog(int, base) @env -> i128 {
        let base = get_int(env, base, "i128")?;
        Ok(
            get_int(env, int, "i128")?
                .checked_ilog(base)
                .ok_or_else(|| error_format!("Invalid base: {base}"))? as i128
        )
    }
    /// Transmutes `bits` into [f64]
    fn f64_bits(bits) @env -> f64 {
        Ok(f64::from_bits(get_int(env, bits, "u64")? as u64))
    }
    fn parse(str, radix) @env -> i128 {
        let radix = get_int(env, radix, "u32")?;
        let 2..=36 = radix else {
            Err(Error::OobIndex(radix as _, 32))?
        };
        i128::from_str_radix(get_str(env, str)?, radix as _)
            .map_err(|e| error_format!("{e}"))
    }
};

pub const FloatIntrinsics: &[Builtin] = &builtins![
    /// Transmutes `float` into [u64]
    fn to_bits(float) @env -> u64 {
        Ok(ConValue::Int(get_float(env, float)?.to_bits() as _))
    }
    /// Transmutes `bits` into [f64]
    fn from_bits(bits) @env -> f64 {
        Ok(f64::from_bits(get_int(env, bits, "u64")? as u64))
    }
    // float intrinsics
    /// Computes the sine of a number
    fn sin(float) @env -> f64 { Ok(get_float(env, float)?.sin()) }
    /// Computes the cosine of a number
    fn cos(float) @env -> f64 { Ok(get_float(env, float)?.cos()) }
    /// Computes the tangent of a number
    fn tan(float) @env -> f64 { Ok(get_float(env, float)?.tan()) }
    /// Computes the square root of a number
    fn sqrt(float) @env -> f64 { Ok(get_float(env, float)?.sqrt()) }
    /// Raises a number to an integer power
    fn powi(float, int) @env -> f64 {
        Ok(get_float(env, float)?.powi(get_int(env, int, "i32")? as _))
    }
    /// Raises a number to a floating-point power
    fn powf(float, power) @env -> f64 {
        Ok(get_float(env, float)?.powf(get_float(env, power)? as _))
    }
    /// Returns the logarithm of `float` with respect to `base`
    fn log(float, base) @env -> f64 {
        Ok(match get_float(env, base)? {
            2.0 => get_float(env, float)?.log2(),
            10.0 => get_float(env, float)?.log10(),
            base => get_float(env, float)?.log(base),
        })
    }
    /// Parses a string as f64
    fn parse(str) @env -> f64 {
        get_str(env, str)?
            .parse::<f64>()
            .map_err(|e| error_format!("{e}"))
    }
];

pub const CharIntrinsics: &[Builtin] = &builtins![];

// TODO: BoolIntrinsics, StringIntrinsics, ArrayIntrinsics, TupleIntrinsics

pub const ArrayIntrinsics: &[Builtin] = &builtins! {
    /// Returns the length of the input list as a [ConValue::Int]
    fn len(array) @env -> i128 {
        Ok(match array.dereference_in(env)? {
            ConValue::Array(arr) => arr.len() as i128,
            other => Err(Error::TypeError("[_]", other.type_of(env)))?,
        })
    }

    /// Pushes an `item` onto the top of an array (by reference)
    fn push(array_by_ref, item) @env {
        let mut array = get_array_by_ref(env, array_by_ref)?;
        let mut items = std::mem::take(array).into_vec();
        items.push(item.clone());
        *array = items.into_boxed_slice();

        Ok(ConValue::Unit)
    }

    /// Pops an item off the top of an array (by reference)
    fn pop(array_by_ref) @env -> Option<_> {
        let mut array = get_array_by_ref(env, array_by_ref)?;
        let mut items = std::mem::take(array).into_vec();
        let out = items.pop();
        *array = items.into_boxed_slice();

        env.to_convalue_option(out)
    }

    fn slice(ConValue::Ref(index), ConValue::Int(start), ConValue::Int(end)) -> [_] {
        match (*start, *end) {
            (0.., 0..) if start <= end => Ok(ConValue::Slice(index.clone(), *start as _, (*end - *start) as _)),
            _ => Err(error_format!("Bad index: {index}[{start}, {end}]"))
        }
    }
};

fn get_float(env: &mut Environment, value: &ConValue) -> IResult<f64> {
    let &ConValue::Float(f) = value.dereference_in(env)? else {
        return Err(Error::TypeError("f64", value.type_of(env)));
    };
    Ok(f)
}

fn get_int(env: &mut Environment, value: &ConValue, ty: &'static str) -> IResult<i128> {
    let &ConValue::Int(v) = value.dereference_in(env)? else {
        return Err(Error::TypeError(ty, value.type_of(env)));
    };
    Ok(v)
}

fn get_str<'e>(env: &'e mut Environment, value: &'e ConValue) -> IResult<&'e str> {
    Ok(match value.dereference_in(env)? {
        ConValue::Str(s) => s.to_ref(),
        ConValue::String(s) => s.as_ref(),
        _ => Err(Error::TypeError("str", value.type_of(env)))?,
    })
}

fn get_array_by_ref<'e>(
    env: &'e mut Environment,
    value: &ConValue,
) -> IResult<&'e mut Box<[ConValue]>> {
    let ConValue::Ref(place) = value else {
        Err(Error::TypeError("&[_]", value.type_of(env)))?
    };
    let mut array = place.get_mut(env)?;
    while let ConValue::Ref(r) = array {
        array = r.clone().get_mut(env)?;
    }
    match array {
        ConValue::Array(array) => Ok(array),
        _ => {
            let array = place.get(env)?;
            Err(Error::TypeError("[_]", array.type_of(env)))
        }
    }
}
