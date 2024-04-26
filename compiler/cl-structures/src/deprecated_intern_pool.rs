//! Trivially-copyable, easily comparable typed indices, and a [Pool] to contain them
//!
//! # Examples
//!
//! ```rust
//! # use cl_structures::intern_pool::*;
//! // first, create a new InternKey type (this ensures type safety)
//! make_intern_key!{
//!     NumbersKey
//! }
//!
//! // then, create a pool with that type
//! let mut numbers: Pool<i32, NumbersKey> = Pool::new();
//! let first = numbers.insert(1);
//! let second = numbers.insert(2);
//! let third = numbers.insert(3);
//!
//! // You can access elements immutably with `get`
//! assert_eq!(Some(&3), numbers.get(third));
//! assert_eq!(Some(&2), numbers.get(second));
//! // or by indexing
//! assert_eq!(1, numbers[first]);
//!
//! // Or mutably
//! *numbers.get_mut(first).unwrap() = 100000;
//!
//! assert_eq!(Some(&100000), numbers.get(first));
//! ```

/// Creates newtype indices over [`usize`] for use as [Pool] keys.
#[macro_export]
macro_rules! make_intern_key {($($(#[$meta:meta])* $name:ident),*$(,)?) => {$(
    $(#[$meta])*
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct $name(usize);

    impl $crate::deprecated_intern_pool::InternKey for $name {
        #[doc = concat!("Constructs a [`", stringify!($name), "`] from a [`usize`] without checking bounds.\n")]
        /// # Safety
        ///
        /// The provided value should be within the bounds of its associated container
        fn from_raw_unchecked(value: usize) -> Self {
            Self(value)
        }
        fn get(&self) -> usize {
            self.0
        }
    }
    impl From< $name > for usize {
        fn from(value: $name) -> Self {
            value.0
        }
    }
)*}}
use core::slice::GetManyMutError;
use std::ops::{Index, IndexMut};

pub use make_intern_key;

use self::iter::InternKeyIter;

/// An index into a [Pool]. For full type-safety,
/// there should be a unique [InternKey] for each [Pool]
pub trait InternKey: std::fmt::Debug {
    /// Constructs an [`InternKey`] from a [`usize`] without checking bounds.
    ///
    /// # Safety
    ///
    /// The provided value should be within the bounds of its associated container.
    // ID::from_raw_unchecked here isn't *actually* unsafe, since bounds should always be
    // checked, however, the function has unverifiable preconditions.
    fn from_raw_unchecked(value: usize) -> Self;
    /// Gets the index of the [`InternKey`] by value
    fn get(&self) -> usize;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pool<T, ID: InternKey> {
    pool: Vec<T>,
    id_type: std::marker::PhantomData<ID>,
}

impl<T, ID: InternKey> Pool<T, ID> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get(&self, index: ID) -> Option<&T> {
        self.pool.get(index.get())
    }
    pub fn get_mut(&mut self, index: ID) -> Option<&mut T> {
        self.pool.get_mut(index.get())
    }

    pub fn get_many_mut<const N: usize>(
        &mut self,
        indices: [ID; N],
    ) -> Result<[&mut T; N], GetManyMutError<N>> {
        self.pool.get_many_mut(indices.map(|id| id.get()))
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.pool.iter()
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.pool.iter_mut()
    }
    pub fn key_iter(&self) -> iter::InternKeyIter<ID> {
        // Safety: Pool currently has pool.len() entries, and data cannot be removed
        InternKeyIter::new(0..self.pool.len())
    }

    /// Constructs an [ID](InternKey) from a [usize], if it's within bounds
    #[doc(hidden)]
    pub fn try_key_from(&self, value: usize) -> Option<ID> {
        (value < self.pool.len()).then(|| ID::from_raw_unchecked(value))
    }

    pub fn insert(&mut self, value: T) -> ID {
        let id = self.pool.len();
        self.pool.push(value);

        // Safety: value was pushed to `self.pool[id]`
        ID::from_raw_unchecked(id)
    }
}

impl<T, ID: InternKey> Default for Pool<T, ID> {
    fn default() -> Self {
        Self { pool: vec![], id_type: std::marker::PhantomData }
    }
}

impl<T, ID: InternKey> Index<ID> for Pool<T, ID> {
    type Output = T;

    fn index(&self, index: ID) -> &Self::Output {
        match self.pool.get(index.get()) {
            None => panic!("Index {:?} out of bounds in pool!", index),
            Some(value) => value,
        }
    }
}
impl<T, ID: InternKey> IndexMut<ID> for Pool<T, ID> {
    fn index_mut(&mut self, index: ID) -> &mut Self::Output {
        match self.pool.get_mut(index.get()) {
            None => panic!("Index {:?} out of bounds in pool!", index),
            Some(value) => value,
        }
    }
}

mod iter {
    use std::{marker::PhantomData, ops::Range};

    use super::InternKey;

    /// Iterates over the keys of a [Pool](super::Pool) independently of the pool itself.
    ///
    /// This is guaranteed to never overrun the length of the pool,
    /// but is *NOT* guaranteed to iterate over all elements of the pool
    /// if the pool is extended during iteration.
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct InternKeyIter<ID: InternKey> {
        range: Range<usize>,
        _id: PhantomData<ID>,
    }

    impl<ID: InternKey> InternKeyIter<ID> {
        /// Creates a new [InternKeyIter] producing the given [InternKey]
        ///
        /// # Safety:
        /// - Range must not exceed bounds of the associated [Pool](super::Pool)
        /// - Items must not be removed from the pool
        /// - Items must be contiguous within the pool
        pub(super) fn new(range: Range<usize>) -> Self {
            Self { range, _id: Default::default() }
        }
    }

    impl<ID: InternKey> Iterator for InternKeyIter<ID> {
        type Item = ID;

        fn next(&mut self) -> Option<Self::Item> {
            // Safety: InternKeyIter can only be created by InternKeyIter::new()
            Some(ID::from_raw_unchecked(self.range.next()?))
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.range.size_hint()
        }
    }
    impl<ID: InternKey> DoubleEndedIterator for InternKeyIter<ID> {
        fn next_back(&mut self) -> Option<Self::Item> {
            // Safety: see above
            Some(ID::from_raw_unchecked(self.range.next_back()?))
        }
    }
    impl<ID: InternKey> ExactSizeIterator for InternKeyIter<ID> {}
}
