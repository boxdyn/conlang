//! Iterable ranges

/// An Exclusive Range `a .. b` iterates from a to b, excluding b
#[lang = "range_exc"]
pub struct RangeExc<T>(T, T)

/// An Inclusive Range `a ..= b` iterates from a to b, including b
#[lang = "range_inc"]
pub struct RangeInc<T>(T, T)

impl<T> RangeExc<T> {
    // fn next(self: &Self) -> T {
    //     let out = (*self.0);
    //     (*self).0 += 1;
    //     out
    // }
}
