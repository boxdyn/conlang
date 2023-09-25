//! Lerox is a stupid, simple combinator library, part of the Conlang project

/// The core trait of Lerox, [Combinator], generically implements a handful of simple
/// combinators for your type based on two required functions:
/// - [`is_alright(&self) -> bool`](Combinator::is_alright) performs some unspecified *evaluation*
///   on `self`, and returns true if `self` is valid for your problem domain.
/// - [`into_alright(self) -> Self`](Combinator::into_alright) performs some unspecified
///   *transformation* of `self` into an alright version of `self` (that is, one which satisfies
///   [`self.is_alright()`](Combinator::is_alright) == `true`)
pub trait Combinator: Sized {
    /// Evaluates the [Combinator], and returns whether it is alright.
    ///
    /// This is the basis for [Combinator]'s [and*](Combinator::and())/[or*](Combinator::or())
    /// behaviors, and is like a generic form of [Option::is_some()] or [Result::is_ok()]
    fn is_alright(&self) -> bool;

    /// Does something to make this [Combinator] [alright](Combinator::is_alright())
    fn into_alright(self) -> Self;

    /// If self is alright, runs f(self)
    fn and(self, f: impl Fn(Self) -> Self) -> Self {
        match self.is_alright() {
            true => f(self),
            false => self,
        }
    }

    /// If self is not alright, runs f(self) as if it were alright
    fn or(self, f: impl FnOnce(Self) -> Self) -> Self {
        match self.is_alright() {
            true => self,
            false => f(self.into_alright()),
        }
    }

    /// Repeats the function f on self until it would fail
    fn and_any(self, f: impl Fn(Self) -> Self) -> Self {
        self.and(|mut this| {
            while this.is_alright() {
                this = this.and(&f)
            }
            this.into_alright()
        })
    }

    /// Repeats the function f on self at least once, then until it fails.
    fn and_many(self, f: impl Fn(Self) -> Self) -> Self {
        self.and(&f).and_any(f)
    }

    /// Returns the result of running f on self, or if it fails, the original self
    fn and_maybe(self, f: impl Fn(Self) -> Self) -> Self
    where Self: Clone {
        self.and_either(f, |g| g)
    }

    /// Returns the result of running f on self, or if it fails, runs g on self
    fn and_either(self, f: impl Fn(Self) -> Self, g: impl Fn(Self) -> Self) -> Self
    where Self: Clone {
        self.clone().and(f).or(|_| g(self))
    }
}
