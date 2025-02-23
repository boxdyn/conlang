//! Iterable ranges

/// An Exclusive Range `a .. b` iterates from a to b, excluding b
#[intrinsic = "range_exc", T]
struct RangeExc(T, T)

/// An Inclusive Range `a ..= b` iterates from a to b, including b
#[intrinsic = "range_inc", T]
struct RangeInc(T, T)
