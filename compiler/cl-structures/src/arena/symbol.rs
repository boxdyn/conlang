use std::{fmt, hash, num::*};

pub trait Symbol: Copy + fmt::Debug + fmt::Display + Eq + hash::Hash {
    /// The largest [`usize`] that may be stored in the [Symbol]
    const MAX: usize;
    /// Returns [`Some(Self)`](Some) if `value` is in range 0..=[Symbol::MAX]
    fn try_from_usize(value: usize) -> Option<Self>;
    /// # May Panic
    /// May panic if `value` is not in range 0..=[Symbol::MAX]
    fn from_usize(value: usize) -> Self {
        Self::try_from_usize(value).expect("should be within MIN and MAX")
    }
    fn into_usize(self) -> usize;
}

#[rustfmt::skip]
impl Symbol for usize {
    const MAX: usize = usize::MAX;
    fn try_from_usize(value: usize) -> Option<Self> { Some(value) }
    fn into_usize(self) -> usize { self }
}

macro_rules! impl_symbol_for_nonzero{($($int:ident: $nonzero:ident),* $(,)?) => {$(
    impl Symbol for $nonzero {
        const MAX: usize = $int::MAX as usize - 1;
        fn try_from_usize(value: usize) -> Option<Self> {
            $nonzero::try_from(value.wrapping_add(1) as $int).ok()
        }
        fn into_usize(self) -> usize {
            self.get() as usize - 1
        }
    }
)*}}

impl_symbol_for_nonzero!(u8: NonZeroU8, u16: NonZeroU16, u32: NonZeroU32, u64: NonZeroU64, usize: NonZeroUsize);
