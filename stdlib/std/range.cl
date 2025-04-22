//! Iterable ranges

type T = _;

/// An Exclusive Range `a .. b` iterates from a to b, excluding b
// #[lang = "range_exc", T]
pub struct RangeExc(T, T)

/// An Inclusive Range `a ..= b` iterates from a to b, including b
// #[lang = "range_inc", T]
pub struct RangeInc(T, T)

impl RangeExc {
    fn next(this: &RangeInc) -> T {
        (*this).0
    }
}
