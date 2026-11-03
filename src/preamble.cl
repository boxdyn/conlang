#!/bin/false
//! Conlang functions which are loaded into the default REPL
//!
//! Contains generally-useful functions which don't require
//! special functionality that the Conlang interpreter
//! doesn't yet have.

// Note: module inlining is NOT performed for this file.

/// Enables the unstable module system
let modules_enabled = true;

// TODO: multiple passes for modules
mod std {
    mod try {
        /// # The `Try` Operator (`?`)
        ///
        /// Within the body of a function which returns a fallible value,
        /// you may use the postifx `?` (`Try`) operator to return errors
        /// to your caller.
        ///
        /// Internally, `Try` is transformed into a call to `.try()` as follows:
        /// ```conlang
        /// let x = could_fail()?;
        ///
        /// // de-sugars to
        /// let x = match could_fail().try() { // inserted call to try
        ///     Tried::Take(t) => t;           // Take value completes the expression
        ///     Tried::Return(r) => return r;  // Return value bubbles up to caller
        /// };
        /// ```
        enum Tried<T, R> { Take(T), Return(R) };
        impl Tried<T, R> {
            /// Implements the `try` (`?`) operator.
            fn try(self) -> Self = self;
        }

        trait Try {
            /// # The `Try` Operator (`?`)
            ///
            /// Within the body of a function which returns a fallible value,
            /// you may use the postifx `?` (`Try`) operator to return errors
            /// to your caller.
            ///
            /// Internally, `Try` is transformed into a call to `.try()` as follows:
            /// ```conlang
            /// let x = could_fail()?;
            ///
            /// // de-sugars to
            /// let x = match could_fail().try() { // inserted call to try
            ///     Tried::Take(t) => t;           // Take value completes the expression
            ///     Tried::Return(r) => return r;  // Return value bubbles up to caller
            /// };
            /// ```
            fn try(self: Self) -> Tried = ...;
        }
    }
}

mod std {
    mod result {
        use super::try::Tried;
        enum Result<T, E> { Ok(T), Err(E) };
        use Result::{Ok, Err};
        impl Result<T, E> {
            fn is_ok(self: &Self) -> bool {
                match self {
                    &Ok(_) => true;
                    &Err(_) => false;
                }
            }
            fn is_err(self: &Self) -> bool {
                match self {
                    &Ok(_) => false;
                    &Err(_) => true;
                }
            }
            /// Maps the value inside the Result::Ok, leaving errors alone.
            fn map<U>(&self: Self, f: fn(T) -> U) -> Result<U, E> {
                match self {
                    Ok(t) => Ok(f(t));
                    Err(e) => Err(e);
                }
            }
            /// Maps the value inside the Result::Err, leaving values alone.
            fn map_err<F>(&self: Self, f: fn(E) -> F) -> Result<T, F> {
                match self {
                    Ok(t) => Ok(t);
                    Err(e) => Err(f(e));
                }
            }
            /// Implements the `try` (`?`) operator on Results
            fn try(self: Result<T, E>) -> Tried<T, Result<!, E>> = match self {
                Ok(t) => Tried::Take(t);
                Err(e) => Tried::Return(Err(e));
            }
        }
    }
}

mod std {
    mod option {
        use super::{try::Tried, result::Result};
        enum Option<T> { Some(T), None };
        use Option::{Some, None};
        impl Option<T> {
            fn is_some(&self) -> bool = let Option::Some(_) = self;
            fn is_none(&self: &Self) -> bool = let Option::None = self;
            fn map<U>(&self: Self, f: fn(T) -> U) -> Option<U> {
                match self {
                    Option::Some(value) => Some(f(value));
                    Option::None => None;
                }
            }
            fn and_then<U>(&self: Self, f: fn(T) -> Option<U>) -> Option<U> {
                f(self?)
            }
            /// Implements the `try` (`?`) operator on Options.
            fn try(self) -> Tried<T, Option<!>> = match self {
                Option::Some(taken) => Tried::Take(taken);
                Option::None => Tried::Return(None);
            }
            /// Returns `Result::Err(error)` if `self` is not `Some(value)`
            fn ok_or<E>(self, error: E) -> Result<T, E> = match self {
                Option::Some(value) => Result::Ok(value);
                Option::None => Result::Err(error);
            }
            /// Returns `Result::Err(error())` if `self` is not `Some(value)`
            fn ok_or_else<E>(self, error: fn() -> E) = match self {
                Option::Some(value) => Result::Ok(value);
                Option::None => Result::Err(error());
            }
        }

        // Option and Result are mutually interdependent
        impl Result<T, E> {
            fn ok(self) -> Option<T> = match self {
                Result::Ok(t) => Some(t);
                Result::Err(_) => None;
            }
            fn err(self) -> Option<E> = match self {
                Result::Ok(_) => None;
                Result::Err(e) => Some(e);
            }
        }
    }
}

mod std {
    mod ops {
        trait Neg {
            /// Negation `-a`
            ///
            /// The prefix `-` operator is defined in terms of this function:
            /// ```conlang
            /// !a => a.neg()
            /// ```
            fn neg(self: Self) -> Self = -self;
        }
        trait Not {
            /// Inversion `!a`
            ///
            /// The prefix `!` operator is defined in terms of this function:
            /// ```conlang
            /// !a => a.not()
            /// ```
            fn not(self: Self) -> Self = !self;
        }
        trait Mul<R> {
            /// Multiplication `a * b`
            ///
            /// The infix `*` operator is defined in terms of this function:
            /// ```conlang
            /// a * b => a.mul(b)
            /// ```
            fn mul(self: Self, other: Self) -> R = self * other;
        }
        trait Div<R> {
            /// Division `a / b`
            ///
            /// The infix `/` operator is defined in terms of this function:
            /// ```conlang
            /// a / b => a.div(b)
            /// ```
            fn div(self: Self, other: Self) -> R = self / other;
        }
        trait Rem<R> {
            /// Remainder `a % b`
            ///
            /// The infix `%` operator is defined in terms of this function:
            /// ```conlang
            /// a % b => a.rem(b)
            /// ```
            fn rem(self: Self, other: Self) -> R = self % other;
        }
        trait Add<R> {
            /// Addition `a + b`
            ///
            /// The infix `+` operator is defined in terms of this function:
            /// ```conlang
            /// a + b => a.add(b)
            /// ```
            fn add(self: Self, other: Self) -> R = self + other;
        }
        trait Sub<R> {
            /// Subtraction `a - b`
            ///
            /// The infix `-` operator is defined in terms of this function:
            /// ```conlang
            /// a - b => a.sub(b)
            /// ```
            fn sub(self: Self, other: Self) -> R = self - other;
        }
        trait Shl<R> {
            /// Shift Left `a << b`
            ///
            /// The infix `<<` operator is defined in terms of this function:
            /// ```conlang
            /// a << b => a.shl(b)
            /// ```
            fn shl(self: Self, other: i32) -> R = self << other;
        }
        trait Shr<R> {
            /// Shift Right `a >> b`
            ///
            /// The infix `>>` operator is defined in terms of this function:
            /// ```conlang
            /// a >> b => a.shr(b)
            /// ```
            fn shr(self: Self, other: i32) -> R = self >> other;
        }
        trait And<R> {
            /// Bitwise And `a & b`
            ///
            /// The infix `&` operator is defined in terms of this function:
            /// ```conlang
            /// a & b => a.and(b)
            /// ```
            fn and(self: Self, other: Self) -> R = self & other;
        }
        trait Xor<R> {
            /// Bitwise Exclusive Or `a ^ b`
            ///
            /// The infix `^` operator is defined in terms of this function:
            /// ```conlang
            /// a ^ b => a.xor(b)
            /// ```
            fn xor(self: Self, other: Self) -> R = self ^ other;
        }
        trait Or<R> {
            /// Bitwise Or `a | b`
            ///
            /// The infix `|` operator is defined in terms of this function:
            /// ```conlang
            /// a | b => a.or(b)
            /// ```
            fn or(self: Self, other: Self) -> R = self ^ other;
        }
        trait Cmp {
            /// Value-to-value Comparison
            ///
            /// If `lhs < rhs`, the result is less than 0
            /// If `lhs == rhs`, the result is 0
            /// If `lhs > rhs`, the result is greater than 0
            ///
            /// Six comparison operators are defined in terms of this function:
            /// ```conlang
            /// a <  b => a.cmp(b) <  0
            /// a <= b => a.cmp(b) <= 0
            /// a == b => a.cmp(b) == 0
            /// a != b => a.cmp(b) != 0
            /// a >= b => a.cmp(b) >= 0
            /// a >  b => a.cmp(b) >  0
            /// ```
            fn cmp(self: &Self, other: &Self) -> bool = self.cmp(other);

            /// Returns the larger of `a` and `b`
            pub fn max<T: ops::Cmp>(a: Self, b: Self) -> Self = if a < b b else a;
    
            /// Returns the smaller of `a` and `b`
            pub fn min<T: ops::Cmp>(a: Self, b: Self) -> Self = if a > b b else a;
        }
        struct RangeExc<T>(T, T);
        struct RangeToExc<T>(T);
        struct RangeInc<T>(T, T);
        struct RangeToInc<T>(T);

    }
}

mod std {
    mod num {
        use {
            u8, u16, u32, u64, u128, usize,
            i8, i16, i32, i64, i128, isize,
            f32, f64, bool, char,
        }
        impl i128 {
            impl std::ops::Cmp for i128; // cmp implementation is compiler builtin

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

            /// Formats `n` as a hexadecimal literal (0xf...)
            pub fn hex(n: u128) {
                let out = "0x";
                for xd in (n.leading_zeroes() / 4).min(31)..32 {
                    out += as_digit(n >> (31 - xd) * 4 & 0xf)
                }
                out
            }

            /// Formats `n` as an octal literal (0o7...)
            pub fn oct(n: u128) {
                let out = "0o";
                for xd in ((n.leading_zeroes() + 1) / 3).min(42)..43 {
                    out += as_digit(n >> (42 - xd) * 3 & 7)
                }
                out
            }

            /// Formats `n` as a binary literal (0b1...)
            pub fn bin(n: u128) {
                let out = "0b";
                for xd in n.leading_zeroes().min(127)..128 {
                    out += as_digit(n >> 127 - xd & 1)
                }
                out
            }
        }

        // type f32;
        // type f64;
        impl f64 {
            impl std::ops::Cmp for f64; // cmp implementation is compiler builtin

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
    }
}

mod std {
    mod iter {
        use std::option::Option;

        /// TODO: real interfaces
        trait IntoIter<T> {
            /// Produces an Iterator instance from `self`
            fn into_iter(self: &Self) -> Iterator<T>;
        }

        trait Iterator<T> {
            /// All iterators<T> implement IntoIter<T>
            impl IntoIter<T> for Iterator<T>
                /// Produces an Iterator instance from `self`
                fn into_iter(&self) -> Self = self;

            // Required methods:
            /// Produces the next item, or `None`
            fn next(self: &Self) -> Option<T>;

            // Provided methods:
            /// Consumes the Iterator, calling `f` on each item
            fn foreach(self, f: fn(T) -> ())
                while let Some(value) = self.next() f(value);

            /// Returns `true` if there is at least one value for which `f` returns `true`
            fn any_of(self, f: fn(T) -> bool)
                while let Some(value) = self.next() {
                    if f(value) break true
                } else false;

            /// Returns `true` if `f` returns `true` for all values
            fn all_of(self, f: fn(T) -> bool)
                while let Some(value) = self.next() {
                    if !f(value) break false
                } else true;

            /// Returns the first value where `f` returns true
            fn first_where(self, f: fn(&T) -> bool) -> Option<T>
                while let Some(value) = self.next() {
                    if f(&value) break Some(value)
                } else None;

            /// Folds items into the accumulator by repeatedly applying `f`.
            fn fold<A>(self, acc: A, f: fn(A, T) -> A) -> A
                while let Some(value) = self.next() {
                    acc = f(acc, value)
                } else acc;

            /// Counts the items in this iterator
            fn count(self) -> usize {
                let count = 0;
                while let Some(value) = self.next() count += 1;
                count
            }

            /// Consumes the remainder of this iterator, returning whether
            /// they were ascending order
            fn is_sorted(self) -> bool {
                let Some(mut prev) = self.next() else return true;
                while let Some(next) = self.next() {
                    if prev > next break false;
                    prev = next;
                } else true
            }

            /// An Iterator which calls a function on each value
            struct Inspect<T>(&Self, fn(&T));
            /// Returns an iterator which calls `f` on every value
            fn inspect(self, f: fn(&T)) -> Inspect<T> = Inspect(self, f);

            /// An Iterator which maps values in T to values in U
            struct Map<T, U>(&Self, fn(T) -> U);
            /// Returns an iterator which maps values in T to values in U through `f`
            fn map<U>(self, f: fn(T) -> U) -> Map<T, U> = Map(self, f);

            /// An Iterator which skips values deemed false by the given predicate
            struct Filter<T>(&Self, fn(T) -> bool);
            /// Returns an iterator which skips values deemed false by the given predicate
            fn filter(self, f: fn(T) -> bool) -> Filter<T> =  Filter(self, f);

            /// An Iterator which both filters and maps values according to a predicate
            struct FilterMap<T, U>(&Self, fn(T) -> Option<U>);
            /// Returns an iterator which both filters and maps values according to `f`
            fn filter_map(self, f: fn(T) -> Option<U>) -> FilterMap<T, U> = FilterMap(self, f);

            /// An Iterator which yields elements from two iterators in parallel
            /// until either of them returns `None`
            struct Zip<T, U>(&Self, Iterator<U>);
            /// Returns an iterator which yields elements from two iterators in parallel
            /// until either of them returns `None`
            fn zip<U>(self, other: IntoIter<U>) -> Zip<T, U> = Zip(self, other.into_iter());

            /// An Iterator which yields optional elements from two iterators in parallel
            /// until both of them return `None`
            struct ZipLongest<T, U>(&Self, Iterator<U>);
            /// Returns an iterator which yields optional elements from two iterators in parallel
            /// until both of them return `None`
            fn zip_longest<T, U>(self, other: IntoIter<U>) -> ZipLongest<T, U> = ZipLongest(self, other.into_iter());

            /// An Iterator which yields elements from its first iterator until exhaustion,
            /// then yields elements from its second iterator
            struct Chain<T>(&Self, Iterator<T>);
            /// Returns an iterator which yields elements from `self` until exhaustion,
            /// then yields elements from `other`
            fn chain(self, other: IntoIter<T>) -> Chain<T> = Chain(self, other.into_iter());

            /// An Iterator which pairs each yielded value with its index in the sequence
            struct Enumerate<T>(&Self, usize);
            /// Returns an iterator which pairs each yielded value with its index in the sequence
            fn enumerate(self) -> Enumerate<T> = Enumerate(self, 0);

            /// An Iterator which yields at most `count` elements.
            struct Take(&Self, usize, usize);
            /// Returns an iterator which yields at most `count` elements.
            fn take(self, count: usize) -> Take<T> = Take(self, 0, count);

            /// An Iterator whose `peek` method looks one element ahead.
            struct Peekable<T>(&Self, Option<T>);
            impl Peekable<T> {
                fn peek(self) -> Option<T> {
                    if (self.1.is_none()) self.1 = self.0.next();
                    self.1
                }
                fn next_if(self, f: fn(T) -> bool) -> Option<T> {
                    if let Some(c) if f(c) = self.peek() {
                        self.next()
                    } else None
                }
            }
            /// Returns an iterator whose `peek` method looks one element ahead.
            fn peekable(self) -> Peekable<T> = Peekable(self, None);
        }

        impl Iterator<T> for Iterator::Inspect<T> {
            fn next(self: &Iterator::Inspect) = match self.0.next() {
                Some(value) => Some((self.1)(&value); value);
                _ => None;
            }
        }

        impl Iterator<U> for Iterator::Map<T, U> {
            fn next(self: &Iterator::Map) = Some((self.1)(self.0.next()?))
        }

        impl Iterator<T> for Iterator::Filter<T> {
            fn next(self: &Iterator::Filter) = loop {
                let Some(value) = self.0.next() else break None;
                if (self.1)(value) break Some(value);
            }
        }

        impl Iterator<U> for Iterator::FilterMap<T, U> {
            fn next(self: &Iterator::FilterMap) = while let Some(value) = self.0.next() {
                if let Some(value) = (self.1)(value) break Some(value);
            } else None;
        }

        impl Iterator<(T, U)> for Iterator::Zip<T, U> {
            fn next(self: &Iterator::Zip) = Some((self.0.next()?, self.1.next()?))
        }

        impl Iterator<(Option<T>, Option<U>)> for Iterator::ZipLongest<T, U> {
            fn next(self: &Iterator::ZipLongest<T, U>) -> Option<(Option<T>, Option<U>)> {
                let a = self.0.next();
                let b = self.1.next();
                if let Some(_) = a || let Some(_) = b {
                    Some((a, b))
                } else None
            }
        }

        impl Iterator<T> for Iterator::Chain<T> {
            fn next(self: &Iterator::Chain<T>) -> Option<T> {
                match self.0.next() {
                    Some(value) => Some(value);
                    None => self.1.next();
                }
            }
        }

        impl Iterator<(usize, T)> for Iterator::Enumerate<T> {
            fn next(self: &Iterator::Enumerate<T>) -> Option<T> {
                let out = self.1, self.0.next()?;
                self.1 += 1;
                Some(out)
            }
        }

        impl Iterator<T> for Iterator::Take<T> {
            fn next(self: &Iterator::Take<T>) = if self.1 < self.2 {
                let out = self.0.next()?;
                self.1 += 1;
                Some(out)
            } else None
        }

        impl Iterator<T> for Iterator::Peekable<T> {
            fn next(self: &Iterator::Peekable<T>) = match self.1 {
                Option::Some(value) => (self.1 = None; Some(value));
                Option::None => self.0.next();
            }
        }

        impl IntoIter<usize> for std::ops::RangeInc {
            struct InclusiveIter(usize, usize);
            impl Iterator<usize> for InclusiveIter {
                fn next(self: &RangeInc::InclusiveIter) = if self.0 > self.1 None else {
                    let out = self.0;
                    self.0 = out + 1;
                    Some(out)
                }
            }

            fn into_iter(&RangeInc(start, end)) = RangeInc::InclusiveIter(start, end);
        }

        impl IntoIter<usize> for std::ops::RangeExc {
            struct ExclusiveIter(usize, usize);
            impl Iterator<usize> for ExclusiveIter {
                fn next(self: &RangeExc::ExclusiveIter) = if self.0 >= self.1 None else {
                    let out = self.0;
                    self.0 = out + 1;
                    Some(out)
                }
            }

            fn into_iter(&RangeExc(start, end)) = RangeExc::ExclusiveIter(start, end);
        }
    }
}

mod std {
    mod array {
        use super::iter::{IntoIter, Iterator};
        use Array; // builtin type-alias for type_of([_])

        impl IntoIter<T> for Array<T> {
            /// An iterator over an array's contents
            struct ArrayIter(Self, usize);
            impl Iterator<_> for ArrayIter {
                fn next(self: &Self::ArrayIter) -> Option<u8> {
                    let index = self.1;
                    self.1 = index + 1;
                    self.0.get(index)
                }
            }

            /// Constructs an iterator over this array's contents
            fn into_iter(self: &Self) = Self::ArrayIter(self, 0);
            let iter = into_iter;

        }

        impl Array {
            /// Determines whether the elements of `self` are in ascending order
            fn is_sorted(self: &Self) = for i in 1..self.len() {
                if self[i-1] > self[i] break false
            } else true;

            /// Computes the cartesian product of `selves` and `others`
            fn cartesian<T>(selves: &[T], others: &[T]) -> [[T; 2]] {
                let out = [];
                for &t in selves
                    for &u in others
                        out.push([t, u]);
                out
            }

            /// The wrong way to sort a million 32-bit integers
            fn sort<T>(self: &[T]) -> &[T] {
                for i in 0..self.len()
                    for j in i + 1..self.len()
                        if self[i] > self[j]
                            self[i], self[j] = self[j], self[i];
                self
            }

            /// Reverses the elements in `self`, returning the reversed Array
            fn reverse<T>(self: &[T]) -> &[T] {
                for i in 0..self.len() / 2 {
                    let j = self.len() - 1 - i;
                    self[i], self[j] = self[j], self[i]
                }
                self
            }

            /// Computes the cartesian product of `values` with itself.
            fn grid<T>(&values: [T]) -> [[T; 2]] = values.cartesian(values);

            /// Gets the index of the first value which matches the predicate
            fn find<T>(values: &[T], f: (&T) -> bool) =
                for idx in 0..values.len() {if f(values[idx]) break Some(idx) } else None;

            /// Collects values from `iter` into a buffer
            fn collect<T>(iter: &IntoIter<T>) -> [T] {
                let out = [];
                let iter = iter.into_iter();
                while let Some(value) = iter.next() out.push(value);
                out
            }

            /// Returns a list of unique elements in `values`.
            ///
            /// Has `O(n^2)` time complexity.
            fn unique<T>(iter: &IntoIter<T>) -> [T] {
                let iter = iter.into_iter();
                let out = [];
                while let Some(value) = iter.next() {
                    for item in out {
                        if value == item break;
                    } else out.push(value)
                }
                out
            }
        }

    }
}

mod std {
    mod string {
        // type str;
        impl str {
            /// An [Iterator] over the bytes in a `str`
            struct Bytes(&str, usize);

            impl Iterator<u8> for Bytes {
                fn next(self: &str::Bytes) -> Option<u8> {
                    let index = self.1;
                    self.1 += 1;
                    self.0.get(index)
                }
            }

            /// Returns an [Iterator] over the [Bytes] in `self`
            fn bytes(self: &str) = str::Bytes(self, 0);
        }

        impl IntoIter<char> for str {
            /// An [Iterator] over the chars in a `str`
            struct Chars(&str, usize);
            impl Iterator<char> for Chars {
                /// Decodes Unicode codepoints in a UTF-8-encoded `str`, ignoring errors
                fn next(self: &str::Chars) -> Option<char> {
                    let byte = self.0.get(self.1)?;
                    let out = match byte {
                        ..0x80 => (self.1 += 1; return Some(byte as char));
                        ..0xc0 => (self.1 += 1; return None); // continuation
                        ..0xe0 => (self.1 += 2; byte & 0x1f); // 2 bytes
                        ..0xf0 => (self.1 += 3; byte & 0x0f); // 3 bytes
                        ..0xf8 => (self.1 += 4; byte & 0x07); // 4 bytes
                        _ => return None;                     // invalid
                    }
                    if (byte >= 0xf0) out = out << 6 | (self.0.get(self.1 - 3)? & 0x3f);
                    if (byte >= 0xe0) out = out << 6 | (self.0.get(self.1 - 2)? & 0x3f);
                    if (byte >= 0xc0) out = out << 6 | (self.0.get(self.1 - 1)? & 0x3f);
                    Some(out as char)
                }
            }

            /// Returns an [Iterator] over the [Chars] in `self`
            fn chars(self: &str) = str::Chars(self, 0);
            let into_iter = chars;
        }
    }

    impl char {
        fn encode_utf8(self) -> [u8; 4] {
            let value = self as i32;
            let out = match value {
                ..0x80     => return [0, 0, 0, value];
                ..0x1000   => [0x00, 0x00, 0xc0, 0x80];
                ..0x10000  => [0x00, 0xe0, 0x80, 0x80];
                ..0x110000 => [0xf0, 0x80, 0x80, 0x80];
                .. => panic("Invalid codepoint: ", value);
            }
            for i in (0..4)
                out[i] |= value >> 18 - 6 * i & 0x3f;
            out
        }
    }

    impl [i128; 4] {
        fn decode_utf8(self) -> char = match self {
            [0, 0, 0, a] => (a & 0x7f);
            [0, 0, b, a] => (a & 0x3f) | (b & 0x1f) << 6;
            [0, c, b, a] => (a & 0x3f) | (b & 0x3f) << 6 | (c & 0x0f) << 12;
            [d, c, b, a] => (a & 0x3f) | (b & 0x3f) << 6 | (c & 0x3f) << 12 | (d & 0x07) << 18;
        } as char;
    }
}

mod std {
    mod preamble {
        use super::{
            option::{Option, Option::{Some, None}},
            result::{Result, Result::{Ok, Err}},
            ops::{RangeExc, RangeInc, RangeToExc, RangeToInc, Cmp::{min, max}},
            iter::{Iterator, IntoIter},
        };

        /// Returns a shark
        pub fn shark() = '\u{1f988}';
    }
}


use std::preamble::*;

modules_enabled = false;
