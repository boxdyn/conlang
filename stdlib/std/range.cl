//! Iterable ranges

/// An Exclusive Range `a .. b` iterates from a to b, excluding b
// #[lang = "range_exc"]
pub struct RangeExc<T>(T, T)

/// An Inclusive Range `a ..= b` iterates from a to b, including b
// #[lang = "range_inc"]
pub struct RangeInc<T>(T, T)

impl RangeExc {
    fn next<T>(this: &RangeExc) -> T {
        (*this).0
    }
}
