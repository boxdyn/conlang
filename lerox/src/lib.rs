//! Lerox is a stupid, simple combinator library, part of the Conlang project

pub trait Combinable {
    /// Evaluates the [Combinable], and returns whether it is alright
    fn is_alright(&self) -> bool;
    /// Makes this [Combinable] alright, somehow
    fn alright(self) -> Self;
}

pub trait Combinator: Combinable + Sized {
    /// If self is alright, runs f(self)
    fn and(self, f: impl Fn(Self) -> Self) -> Self {
        match self.is_alright() {
            true => f(self),
            false => self,
        }
    }

    /// If self is not alright, runs f(self)
    fn or(self, f: impl FnOnce(Self) -> Self) -> Self {
        match self.is_alright() {
            true => self,
            false => f(self),
        }
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

    /// Repeats the function f on self until it fails
    fn and_any(self, f: impl Fn(Self) -> Self) -> Self {
        self.and(|mut this| {
            while this.is_alright() {
                this = this.and(&f)
            }
            this.alright()
        })
    }

    /// Repeats the function f on self at least once, then until it fails.
    fn and_many(self, f: impl Fn(Self) -> Self) -> Self {
        self.and(&f).and_any(f)
    }
}

impl<C: Combinable> Combinator for C {}
