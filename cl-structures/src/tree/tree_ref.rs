//! An element in a [Tree](super::Tree)
///
/// Contains a niche, and as such, [`Option<TreeRef<T>>`] is free :D
use std::{marker::PhantomData, num::NonZeroUsize};

/// An element of in a [Tree](super::Tree).
//? The index of the node is stored as a [NonZeroUsize] for space savings
//? Making Refs T-specific helps the user keep track of which Refs belong to which trees.
//? This isn't bulletproof, of course, but it'll keep Ref<Foo> from being used on Tree<Bar>
pub struct Ref<T: ?Sized>(NonZeroUsize, PhantomData<T>);

impl<T: ?Sized> Ref<T> {
    /// Constructs a new [Ref] with the given index
    pub fn new_unchecked(index: usize) -> Self {
        // Safety: index cannot be zero because we use saturating addition on unsigned type.
        Self(
            unsafe { NonZeroUsize::new_unchecked(index.saturating_add(1)) },
            PhantomData,
        )
    }
}

impl<T: ?Sized> From<Ref<T>> for usize {
    fn from(value: Ref<T>) -> Self {
        usize::from(value.0) - 1
    }
}

/* --- implementations of derivable traits, because we don't need bounds here --- */

impl<T: ?Sized> std::fmt::Debug for Ref<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TreeRef").field(&self.0).finish()
    }
}

impl<T: ?Sized> std::hash::Hash for Ref<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
        self.1.hash(state);
    }
}

impl<T: ?Sized> PartialEq for Ref<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl<T: ?Sized> Eq for Ref<T> {}

impl<T: ?Sized> PartialOrd for Ref<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: ?Sized> Ord for Ref<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: ?Sized> Clone for Ref<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for Ref<T> {}
