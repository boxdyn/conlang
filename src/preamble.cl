#!/bin/false
//! Conlang functions which are loaded into the default REPL
//!
//! Contains generally-useful functions which don't require
//! special functionality that the Conlang interpreter
//! doesn't yet have.

// Note: module inlining is NOT performed for this file.

enum Option<T> { Some(T), None };
use Option::{Some, None};
impl Option {
    fn is_some(&self) -> bool {
        match self {
            Some(_) => true;
            None() => false;
        }
    }
    fn is_none(&self: &Self) -> bool {
        let Some(_) = self
    }
    fn map<U>(&self: Self, f: fn(T) -> U) -> Option<U> {
        match self {
            Some(value) => Some(f(value));
            None() => None();
        }
    }
    fn and_then<U>(&self: Self, f: fn(T) -> Option<U>) -> Option<U> {
        f(self?)
    }
}

enum Result<T, E> { Ok(T), Err(E) };
use Result::{Ok, Err};
impl Result {
    fn is_ok(self: &Self) -> bool {
        match *self {
            Ok(_) => true;
            Err(_) => false;
        }
    }
    fn is_err(self: &Self) -> bool {
        match *self {
            Ok(_) => false;
            Err(_) => true;
        }
    }
    /// Maps the value inside the Result::Ok, leaving errors alone.
    fn map<U>(self: &Self, f: fn(T) -> U) -> Result<U, E> {
        match *self {
            Ok(t) => Ok(f(t));
            Err(e) => Err(e);
        }
    }
    /// Maps the value inside the Result::Err, leaving values alone.
    fn map_err<F>(self: &Self, f: fn(E) -> F) -> Result<T, F> {
        match *self {
            Ok(t) => Ok(t);
            Err(e) => Err(f(e));
        }
    }
}

// TODO: accurate floating point number parsing

impl f64 {
    /// The half-circle constant
    let PI = f64::from_bits(0x400921fb54442d18);
    /// The golden ratio
    let PHI = f64::from_bits(0x3ff9e3779b97f4a8);
    /// A really small number
    let SMOL = f64::from_bits(0x0010000000000000);
    /// A really big number
    let LORG = f64::from_bits(0x7fefffffffffffff);
    /// The most big number
    let INF = f64::from_bits(0x7ff0000000000000);
    /// The most insidious number
    let NaN = f64::from_bits(0x7ff8000000000000);
    /// Alias for the most insidious number
    let NAN = NaN;

    /// Gets the sign of `n` as an integer
    pub fn sign(n: f64) = f64::to_bits(n) >> 63;
    /// Gets the exponent of `n` as an integer
    pub fn exponent(n: f64) = (f64::to_bits(n) >> 52 & 0x7ff) - 1023;
    /// Gets the significand (mantissa) of `n` as an integer
    pub fn significand(n: f64) = f64::to_bits(n) & (1 << 52) - 1;
    /// Constructs a value from (sign, exponent, significand) form
    pub fn from_parts(sign: i64, exp: i64, significand: i64) = f64::from_bits(
        (sign & 1) << 63 | (exp + 1023 & 0x7ff) << 52 | (significand & (1 << 52) - 1)
    );
}

/// TODO: real interfaces
struct Iterator<T>;
impl Iterator<T> {
    /// Implements the Iterator interface for a type
    const fn _derive(T: Type, next: fn(&T) -> Option<_>) impl T use {
        next,
        Iterator::{ into_iter, foreach, inspect, map, filter, filter_map }
    };

    // Required methods:
    /// Produces the next item, or `None`
    fn next(self: &Self) -> Option<T>;
    /// Produces an Iterator instance from `self`
    fn into_iter(&self) -> Iterator<T> = self;

    /// Consumes the Iterator, calling `f` on each item
    fn foreach(self, f: fn(T) -> ())
        while let Some(value) = (*self).next() f(value);

    struct Inspect<T>(&Iterator<T>, fn(&T));
    fn inspect(self, f: fn(&T)) -> Inspect<T> =
        Iterator::Inspect(self, f);

    /// An Iterator which maps values in T to values in U
    struct Map<T, U>(&Iterator<T>, fn(T) -> U);
    fn map<U>(self, f: fn(T) -> U) -> Map<T, U> =
        Iterator::Map(self, f);

    /// An Iterator which skips values deemed false by the given predicate
    struct Filter<T>(&Iterator<T>, fn(T) -> bool);
    fn filter(self, f: fn(T) -> bool) -> Filter<T> =
        Iterator::Filter(self, f);

    /// An Iterator which both filters and maps values according to a predicate
    struct FilterMap<T, U>(&Iterator<T>, fn(T) -> Option<U>);
    fn filter_map(self, f: fn(T) -> Option<U>) -> FilterMap<T, U> =
        Iterator::FilterMap(self, f);
}

Iterator::_derive(Iterator::Inspect, {
    fn (self: &Iterator::Inspect) = match (*self.0).next() {
        Some(value) => Some((self.1)(&value); value);
        _ => None;
    }
});
Iterator::_derive(Iterator::Map, {
    fn (self: &Iterator::Map) = Some((self.1)((*self.0).next()?))
});
Iterator::_derive(Iterator::Filter, {
    fn (self: &Iterator::Filter) = loop {
        let Some(value) = (*self.0).next() else break None;
        if (self.1)(value) break Some(value);
    }
});
Iterator::_derive(Iterator::FilterMap, {
    fn (self: &Iterator::FilterMap) = while let Some(value) = (*self.0).next() {
        if let Some(value) = (self.1)(value) break Some(value);
    } else None
});

mod _ {
    struct unstable_metaprogramming;
    impl unstable_metaprogramming {
        // TODO: give enums a parental relation
        const fn enum_impl(Enum) {
            for Variant in Enum::VARIANTS
                for key, value in Enum.module().mod_into_binds()
                    impl Variant { let Self = Enum; bind(key, value) }
        }
        const fn spread(Type, types: [_]) {
            for ty in types
                for key, value in Type.module().mod_into_binds()
                    impl ty { bind(key, value) }
        }
    }
}

impl RangeInc {
    struct InclusiveIter(usize, usize);
    Iterator::_derive(InclusiveIter, {
        fn next(self: &RangeInc::InclusiveIter) = if self.0 > self.1 None else {
            let out = self.0;
            self.0 += 1;
            Some(out)
        }
    });
    
    fn into_iter(&RangeInc(start, end)) = RangeInc::InclusiveIter(start, end);
}
impl RangeExc {
    struct ExclusiveIter(usize, usize);
    Iterator::_derive(ExclusiveIter, {
        fn next(self: &RangeExc::ExclusiveIter) = if self.0 >= self.1 None else {
            let out = self.0;
            self.0 += 1;
            Some(out)
        }
    });
    fn into_iter(&RangeExc(start, end)) = RangeExc::ExclusiveIter(start, end);
}

// TODO: type Array = [_];
let Array = [].type_of();
impl Array {
    /// An iterator over an array's contents
    struct ArrayIter(Array, RangeExc);
    Iterator::_derive(ArrayIter, {
        fn next(self: &Array::ArrayIter::ExclusiveIter) match self.1.next() {
            Some(index) => Some(self.0[index]);
            _ => None;
        }
    });

    /// Constructs an iterator over this array's contents
    fn into_iter(self: &Array) = Array::ArrayIter(self, (0..self.len()).into_iter());

    /// Computes the cartesian product of `selves` and `others`
    fn cartesian<T>(selves: &[T], others: &[T]) -> [[T; 2]] {
        let out = [];
        for &t in selves
            for &u in others
                out.push([t, u]);
        out
    }

    /// Computes the cartesian product of `values` with itself.
    fn grid<T>(&values: [T]) -> [[T; 2]] = values.cartesian(values);

    /// Gets the index of the first value which matches the predicate
    fn find<T>(values: &[T], f: (&T) -> bool) =
        for idx in 0..values.len() {if f(values[idx]) break Some(idx) } else None;

    fn collect<T>(iter: &Iterator<T>) -> Self {
        let out = [];
        while let Some(value) = (*iter).next() out.push(value);
        out
    }
}

// TODO: Remove this when name resolution isn't fake
unstable_metaprogramming::enum_impl(Option);
unstable_metaprogramming::enum_impl(Result);
unstable_metaprogramming::spread(f64, [f32]);
unstable_metaprogramming::spread(i128, [
    i8, i16, i32, i64,       isize,
    u8, u16, u32, u64, u128, usize,
]);


/// Returns the larger of `a` and `b`
pub fn max<T: Cmp>(a: T, b: T) -> T = if a < b b else a;

/// Returns the smaller of `a` and `b`
pub fn min<T: Cmp>(a: T, b: T) -> T = if a > b b else a;

/// Computes the square root of `n`
pub fn sqrt(mut n: f64) -> f64 {
    // NOTE: within a function body, we don't (currently) have access to `impl` members.
    if n < 0.0 return f64::NaN; // TODO: re-uppercase
    if n == 0.0 return 0.0;
    let z, err = n, f64::INF;
    while let adj = (z * z - n) / (2.0 * z) && f64::exponent(adj) < f64::exponent(err) {
        z -= adj;
        err = adj;
    }
    z
}

/// Returns the [char] representation of the `n`+1-ary digit `n` (up to base-36)
pub fn as_digit(n: u32) -> char = match n {
    ..10 => n + '0' as u32;
    _ => n - 10 + 'a' as u32;
} as char;

/// Formats `n` as a string in base `radix`
pub fn radix(n: i64, radix: i64) {
    fn recurse(n: i64, radix: i64) = if n == 0 "" else {
        recurse(n / radix, radix) + as_digit(n % radix)
    }

    match n {
        0 => "0";
        // TODO: breaks at i64::MIN
        ..0 => "-" + recurse(-n, radix);
        _ => recurse(n, radix);
    }
}

/// Counts the leading zeroes in `n`
pub fn count_leading_zeroes(n: u128) -> u128 {
    let mut xd = u128::BITS;
    if n < 0 return 0;
    while n != 0 {
        xd -= 1;
        n >>= 1;
    }
    xd
}

/// Formats `n` as a hexadecimal literal (0xf...)
pub fn hex(n: u128) {
    let out = "0x";
    for xd in min(count_leading_zeroes(n) / 4, 31)..32 {
        out += as_digit(n >> (31 - xd) * 4 & 0xf)
    }
    out
}

/// Formats `n` as an octal literal (0o7...)
pub fn oct(n: u128) {
    let out = "0o";
    for xd in min((count_leading_zeroes(n) + 1) / 3, 42)..43 {
        out += as_digit(n >> (42 - xd) * 3 & 7)
    }
    out
}

/// Formats `n` as a binary literal (0b1...)
pub fn bin(n: u128) {
    let out = "0b";
    for xd in min(count_leading_zeroes(n), 127)..128 {
        out += as_digit(n >> 127 - xd & 1)
    }
    out
}

/// Returns a shark
pub fn shark() = '\u{1f988}';
