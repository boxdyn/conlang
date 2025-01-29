//! Implementations of built-in functions

use super::{
    convalue::ConValue,
    env::Environment,
    error::{Error, IResult},
    BuiltIn, Callable,
};
use cl_ast::Sym;
use std::{
    io::{stdout, Write},
    rc::Rc,
    slice,
};

builtins! {
    const MISC;
    /// Unstable variadic format function
    pub fn format<_, args> () -> IResult<ConValue> {
        use std::fmt::Write;
        let mut out = String::new();
        for arg in args {
            write!(out, "{arg}").ok();
        }
        Ok(ConValue::String(out.into()))
    }

    /// Unstable variadic print function
    pub fn print<_, args> () -> IResult<ConValue> {
        let mut out = stdout().lock();
        for arg in args {
            write!(out, "{arg}").ok();
        }
        Ok(ConValue::Empty)
    }

    /// Unstable variadic println function
    pub fn println<_, args> () -> IResult<ConValue> {
        let mut out = stdout().lock();
        for arg in args {
            write!(out, "{arg}").ok();
        }
        writeln!(out).ok();
        Ok(ConValue::Empty)
    }

    /// Prints the [Debug](std::fmt::Debug) version of the input values
    pub fn dbg<_, args> () -> IResult<ConValue> {
        let mut out = stdout().lock();
        for arg in args {
            writeln!(out, "{arg:?}").ok();
        }
        Ok(args.into())
    }

    pub fn dbgp<_, args> () -> IResult<ConValue> {
        let mut out = stdout().lock();
        for arg in args {
            writeln!(out, "{arg:#?}").ok();
        }
        Ok(ConValue::Empty)
    }

    /// Dumps info from the environment
    pub fn dump<env, _>() -> IResult<ConValue> {
        println!("{}", *env);
        Ok(ConValue::Empty)
    }

    /// Gets the length of a container or range
    pub fn len<env, _>(list) -> IResult<ConValue> {
        Ok(ConValue::Int(match list {
            ConValue::Empty => 0,
            ConValue::String(s) => s.chars().count() as _,
            ConValue::Ref(r) => return len.call(env, slice::from_ref(r.as_ref())),
            ConValue::Array(t) => t.len() as _,
            ConValue::Tuple(t) => t.len() as _,
            ConValue::RangeExc(start, end) => (end - start) as _,
            ConValue::RangeInc(start, end) => (end - start + 1) as _,
            _ => Err(Error::TypeError)?,
        }))
    }

    /// Gets a line of text from stdin
    pub fn get_line() -> IResult<ConValue> {
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
        Ok(ConValue::String(line.into()))
    }

    /// Lists the potential "upvars" (lifted environment) of a function
    pub fn collect_upvars<env, _>(ConValue::Function(f)) -> IResult<ConValue> {
        use crate::function::collect_upvars::collect_upvars;
        for (name, bind) in collect_upvars(f.decl(), env) {
            match bind {
                Some(bind) =>println!("{name}: {bind}"),
                None => println!("{name}: _"),
            }
        }
        Ok(ConValue::Empty)
    }

    /// Lists the collected environment of a function.
    pub fn upvars(ConValue::Function(f)) -> IResult<ConValue> {
        let uv = f.upvars();
        for (name, bind) in uv.iter() {
            match bind {
                Some(bind) =>println!("{name}: {bind}"),
                None => println!("{name}: _"),
            }
        }
        Ok(ConValue::Empty)
    }
}

builtins! {
    const BINARY;
    /// Multiplication `a * b`
    pub fn mul(lhs, rhs) -> IResult<ConValue> {
        Ok(match (lhs, rhs) {
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a * b),
            _ => Err(Error::TypeError)?
        })
    }

    /// Division `a / b`
    pub fn div(lhs, rhs) -> IResult<ConValue> {
        Ok(match (lhs, rhs){
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a / b),
            _ => Err(Error::TypeError)?
        })
    }

    /// Remainder `a % b`
    pub fn rem(lhs, rhs) -> IResult<ConValue> {
        Ok(match (lhs, rhs) {
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a % b),
            _ => Err(Error::TypeError)?,
        })
    }

    /// Addition `a + b`
    pub fn add(lhs, rhs) -> IResult<ConValue> {
        Ok(match (lhs, rhs) {
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a + b),
            (ConValue::String(a), ConValue::String(b)) => (a.to_string() + &b.to_string()).into(),
            _ => Err(Error::TypeError)?
        })
    }

    /// Subtraction `a - b`
    pub fn sub(lhs, rhs) -> IResult<ConValue> {
        Ok(match (lhs, rhs) {
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a - b),
            _ => Err(Error::TypeError)?,
        })
    }

    /// Shift Left `a << b`
    pub fn shl(lhs, rhs) -> IResult<ConValue> {
        Ok(match (lhs, rhs) {
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a << b),
            _ => Err(Error::TypeError)?,
        })
    }

    /// Shift Right `a >> b`
    pub fn shr(lhs, rhs) -> IResult<ConValue> {
        Ok(match (lhs, rhs) {
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a >> b),
            _ => Err(Error::TypeError)?,
        })
    }

    /// Bitwise And `a & b`
    pub fn and(lhs, rhs) -> IResult<ConValue> {
        Ok(match (lhs, rhs) {
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a & b),
            (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a & b),
            _ => Err(Error::TypeError)?,
        })
    }

    /// Bitwise Or `a | b`
    pub fn or(lhs, rhs) -> IResult<ConValue> {
        Ok(match (lhs, rhs) {
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a | b),
            (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a | b),
            _ => Err(Error::TypeError)?,
        })
    }

    /// Bitwise Exclusive Or `a ^ b`
    pub fn xor(lhs, rhs) -> IResult<ConValue> {
        Ok(match (lhs, rhs) {
            (ConValue::Empty, ConValue::Empty) => ConValue::Empty,
            (ConValue::Int(a), ConValue::Int(b)) => ConValue::Int(a ^ b),
            (ConValue::Bool(a), ConValue::Bool(b)) => ConValue::Bool(a ^ b),
            _ => Err(Error::TypeError)?,
        })
    }

    /// Tests whether `a < b`
    pub fn lt(lhs, rhs) -> IResult<ConValue> {
        cmp!(lhs, rhs, false, <)
    }

    /// Tests whether `a <= b`
    pub fn lt_eq(lhs, rhs) -> IResult<ConValue> {
        cmp!(lhs, rhs, true, <=)
    }

    /// Tests whether `a == b`
    pub fn eq(lhs, rhs) -> IResult<ConValue> {
        cmp!(lhs, rhs, true, ==)
    }

    /// Tests whether `a != b`
    pub fn neq(lhs, rhs) -> IResult<ConValue> {
        cmp!(lhs, rhs, false, !=)
    }

    /// Tests whether `a <= b`
    pub fn gt_eq(lhs, rhs) -> IResult<ConValue> {
        cmp!(lhs, rhs, true, >=)
    }

    /// Tests whether `a < b`
    pub fn gt(lhs, rhs) -> IResult<ConValue> {
        cmp!(lhs, rhs, false, >)
    }
}
builtins! {
    const RANGE;
    /// Exclusive Range `a..b`
    pub fn range_exc(lhs, rhs) -> IResult<ConValue> {
        let (&ConValue::Int(lhs), &ConValue::Int(rhs)) = (lhs, rhs) else {
            Err(Error::TypeError)?
        };
        Ok(ConValue::RangeExc(lhs, rhs.saturating_sub(1)))
    }

    /// Inclusive Range `a..=b`
    pub fn range_inc(lhs, rhs) -> IResult<ConValue> {
        let (&ConValue::Int(lhs), &ConValue::Int(rhs)) = (lhs, rhs) else {
            Err(Error::TypeError)?
        };
        Ok(ConValue::RangeInc(lhs, rhs))
    }
}
builtins! {
    const UNARY;
    /// Negates the ConValue
    pub fn neg(tail) -> IResult<ConValue> {
        Ok(match tail {
            ConValue::Empty => ConValue::Empty,
            ConValue::Int(v) => ConValue::Int(v.wrapping_neg()),
            ConValue::Float(v) => ConValue::Float(-v),
            _ => Err(Error::TypeError)?,
        })
    }

    /// Inverts the ConValue
    pub fn not(tail) -> IResult<ConValue> {
        Ok(match tail {
            ConValue::Empty => ConValue::Empty,
            ConValue::Int(v) => ConValue::Int(!v),
            ConValue::Bool(v) => ConValue::Bool(!v),
            _ => Err(Error::TypeError)?,
        })
    }

    pub fn deref(tail) -> IResult<ConValue> {
        Ok(match tail {
            ConValue::Ref(v) => Rc::as_ref(v).clone(),
            _ => tail.clone(),
        })
    }
}

/// Turns an argument slice into an array with the (inferred) correct number of elements
pub fn to_args<const N: usize>(args: &[ConValue]) -> IResult<&[ConValue; N]> {
    args.try_into()
        .map_err(|_| Error::ArgNumber { want: N, got: args.len() })
}

/// Turns function definitions into ZSTs which implement [Callable] and [BuiltIn]
macro builtins (
    $(prefix = $prefix:literal)?
    const $defaults:ident $( = [$($additional_builtins:expr),*$(,)?])?;
    $(
        $(#[$meta:meta])*$vis:vis fn $name:ident$(<$env:tt, $args:tt>)? ( $($($arg:pat),+$(,)?)? ) $(-> $rety:ty)?
            $body:block
    )*
) {
    /// Builtins to load when a new interpreter is created
    pub const $defaults: &[&dyn BuiltIn] = &[$(&$name,)* $($additional_builtins)*];
    $(
        $(#[$meta])* #[allow(non_camel_case_types)] #[derive(Clone, Debug)]
        /// ```rust,ignore
        #[doc = stringify!(builtin! fn $name($($($arg),*)?) $(-> $rety)? $body)]
        /// ```
        $vis struct $name;
        impl BuiltIn for $name {
            fn description(&self) -> &str { concat!("builtin ", stringify!($name), stringify!(($($($arg),*)?) )) }
        }
        impl Callable for $name {
            #[allow(unused, irrefutable_let_patterns)]
            fn call(&self, env: &mut Environment, args: &[ConValue]) $(-> $rety)? {
                $(let $env = env;
                let $args = args;)?
                $(let [$($arg),*] = to_args(args)? else {
                    Err(Error::TypeError)?
                };)?
                $body
            }
            fn name(&self) -> Sym { stringify!($name).into() }
        }
    )*
}

/// Templates comparison functions for [ConValue]
macro cmp ($a:expr, $b:expr, $empty:literal, $op:tt) {
    match ($a, $b) {
        (ConValue::Empty, ConValue::Empty) => Ok(ConValue::Bool($empty)),
        (ConValue::Int(a), ConValue::Int(b)) => Ok(ConValue::Bool(a $op b)),
        (ConValue::Bool(a), ConValue::Bool(b)) => Ok(ConValue::Bool(a $op b)),
        (ConValue::Char(a), ConValue::Char(b)) => Ok(ConValue::Bool(a $op b)),
        (ConValue::String(a), ConValue::String(b)) => Ok(ConValue::Bool(&**a $op &**b)),
        _ => Err(Error::TypeError)
    }
}
