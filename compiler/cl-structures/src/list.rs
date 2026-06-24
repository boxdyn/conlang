//! A stack or arena-allocable append-only linked list.
//!
//! This is pretty good for expressing recursive algorithms which
//! follow a stack discipline but are expressed easier with context
//! further up the stack.
//!
//! ```rust
//! # use cl_structures::list::List;
//! let l1 = List::new("which borrows the rest");
//! let l2 = l1.enter("I must construct a new node");
//! let l3 = l2.enter("For each new item");
//!
//! // You can iterate over the items from head to tail
//! let mut list = Vec::from_iter(l3.iter().copied());
//!
//! assert_eq!(list[..], [
//!     "For each new item",
//!     "I must construct a new node",
//!     "which borrows the rest",
//! ])
//! ```

/// A stack or arena-allocable append-only linked list.
#[derive(Clone, Copy, Debug, Default)]
pub enum List<'parent, T> {
    /// A list with one more element than its parent
    Cons(&'parent List<'parent, T>, T),
    /// The base case: an empty list
    #[default]
    Nil,
}

impl<'parent, T> List<'parent, T> {
    /// Creates a new single-element [List]
    pub fn new(value: T) -> List<'static, T> {
        List::Cons(&List::Nil, value)
    }

    /// Creates a [List] with one more element than `self`
    pub fn enter<'a>(&'a self, value: T) -> List<'a, T>
    where T: 'a {
        List::Cons(self, value)
    }

    /// Gets the value contained in this [List] (the `car`)
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Cons(_, value) => Some(value),
            Self::Nil => None,
        }
    }

    /// Gets the parent element of this [List] (the `cdr`)
    pub fn parent(&self) -> Option<&List<'parent, T>> {
        match self {
            Self::Cons(parent, _) => Some(parent),
            Self::Nil => None,
        }
    }

    /// Gets an [Iterator] over the elements of this [List].
    ///
    /// Elements are yielded in reverse insertion order (most to least recent)
    pub fn iter(&self) -> ListIter<'_, T> {
        ListIter(self)
    }

    pub fn get_reverse(&self) -> Vec<&T> {
        fn inner<'i, T>(parent: &'i List<'i, T>, mut vec: Vec<&'i T>) -> Vec<&'i T> {
            if let List::Cons(parent, value) = parent {
                vec = inner(parent, vec);
                vec.push(value)
            }
            vec
        }
        inner(self, vec![])
    }
}

/// Iterates over the items of a List from most recent to least
pub struct ListIter<'p, T>(&'p List<'p, T>);
impl<'p, T> Iterator for ListIter<'p, T> {
    type Item = &'p T;

    fn next(&mut self) -> Option<Self::Item> {
        let List::Cons(list, value) = self.0 else {
            return None;
        };
        self.0 = *list;
        Some(value)
    }
}
