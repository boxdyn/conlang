//! Implementations of built-in functions

use super::{
    env::Environment,
    error::{Error, IResult},
    temp_type_impl::ConValue,
    BuiltIn, Callable,
};
use cl_ast::Sym;
use std::{
    io::{stdout, Write},
    rc::Rc,
};

builtins! {
    const MISC;
    /// Unstable variadic print function
    pub fn print<_, args> () -> IResult<ConValue> {
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
    /// Dumps info from the environment
    pub fn dump<env, _>() -> IResult<ConValue> {
        println!("{}", *env);
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
            ConValue::Int(v) => ConValue::Int(-v),
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
        $(#[$meta:meta])*$vis:vis fn $name:ident$(<$env:tt, $args:tt>)? ( $($($arg:tt),+$(,)?)? ) $(-> $rety:ty)?
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
            #[allow(unused)]
            fn call(&self, env: &mut Environment, args: &[ConValue]) $(-> $rety)? {
                // println!("{}", stringify!($name), );
                $(let $env = env;
                let $args = args;)?
                $(let [$($arg),*] = to_args(args)?;)?
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
