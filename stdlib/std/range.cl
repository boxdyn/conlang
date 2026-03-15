//! Iterable ranges

use super::option::Option;

/// An Exclusive Range `a .. b` iterates from a to b, excluding b
#[lang = "range_exc"]
pub struct RangeExc<T>(T, T);

impl<T> RangeExc<T> {
    fn next(self: &Self) -> Option<T> {
        if (*self).0 >= (*self).1 {
            return Option::None;
        }
        let out = (*self).0;
        (*self).0 += 1;
        Option::Some(out)
    }
}

/// An Inclusive Range `a ..= b` iterates from a to b, including b
#[lang = "range_inc"]
pub struct RangeInc<T>(T, T);

impl<T> RangeInc<T> {
    fn next(self: &Self) -> Option<T> {
        if (*self).0 > (*self).1 {
            return Option::None;
        }
        let out = (*self).0;
        (*self).0 += 1;
        Option::Some(out)
    }
}
