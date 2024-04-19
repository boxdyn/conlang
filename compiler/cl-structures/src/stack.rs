//! A contiguous collection with constant capacity.
//!
//! Since the capacity of a [Stack] may be [*known at compile time*](Sized),
//! it may live on the call stack.
//!
//!
//! # Examples
//!
//! Unlike a [Vec], the [Stack] doesn't grow when it reaches capacity.
//! ```should_panic
//! # use cl_structures::stack::*;
//! let mut v = stack![1];
//! v.push("This should work");
//! v.push("This will panic!");
//! ```
//! To get around this limitation, the methods [try_push](Stack::try_push) and
//! [try_insert](Stack::try_insert) are provided:
//! ```
//! # use cl_structures::stack::*;
//! let mut v = stack![1];
//! v.push("This should work");
//! v.try_push("This should produce an err").unwrap_err();
//! ```
//!
//! As the name suggests, a [Stack] enforces a stack discipline:
//! ```
//! # use cl_structures::stack::*;
//! let mut v = stack![100];
//!
//! assert_eq!(100, v.capacity());
//! assert_eq!(0, v.len());
//!
//! // Elements are pushed one at a time onto the stack
//! v.push("foo");
//! v.push("bar");
//! assert_eq!(2, v.len());
//!
//! // The stack can be used anywhere a slice is expected
//! assert_eq!(Some(&"foo"), v.get(0));
//! assert_eq!(Some(&"bar"), v.last());
//!
//! // Elements are popped from the stack in reverse order
//! assert_eq!(Some("bar"), v.pop());
//! assert_eq!(Some("foo"), v.pop());
//! assert_eq!(None, v.pop());
//! ```

// yar har! here there be unsafe code! Tread carefully.

use core::slice;
use std::{
    fmt::Debug,
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    ops::{Deref, DerefMut},
    ptr,
};

/// Creates a [`stack`] containing the arguments
///
/// # Examples
///
/// Creates a *full* [`Stack`] containing a list of elements
/// ```
/// # use cl_structures::stack::stack;
/// let mut v = stack![1, 2, 3];
///
/// assert_eq!(Some(3), v.pop());
/// assert_eq!(Some(2), v.pop());
/// assert_eq!(Some(1), v.pop());
/// assert_eq!(None, v.pop());
/// ```
///
/// Creates a *full* [`Stack`] from a given element and size
/// ```
/// # use cl_structures::stack::stack;
/// let mut v = stack![1; 2];
///
/// assert_eq!(Some(1), v.pop());
/// assert_eq!(Some(1), v.pop());
/// assert_eq!(None, v.pop());
/// ```
///
/// Creates an *empty* [`Stack`] from a given size
/// ```
/// # use cl_structures::stack::{Stack, stack};
/// let mut v = stack![10];
///
/// assert_eq!(0, v.len());
/// assert_eq!(10, v.capacity());
///
/// v.push(10);
/// assert_eq!(Some(&10), v.last());
/// ```
pub macro stack {
    ($count:literal) => {
        Stack::<_, $count>::new()
    },
    ($value:expr ; $count:literal) => {{
        let mut stack: Stack<_, $count> = Stack::new();
        for _ in 0..$count {
            stack.push($value)
        }
        stack
    }},
    ($($values:expr),* $(,)?) => {
        Stack::from([$($values),*])
    }
}

/// A contiguous collection with constant capacity
pub struct Stack<T, const N: usize> {
    _data: PhantomData<T>,
    buf: [MaybeUninit<T>; N],
    len: usize,
}

impl<T: Clone, const N: usize> Clone for Stack<T, N> {
    fn clone(&self) -> Self {
        let mut new = Self::new();
        for value in self.iter() {
            new.push(value.clone())
        }
        new
    }
}

impl<T: Debug, const N: usize> Debug for Stack<T, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T, const N: usize> Default for Stack<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Deref for Stack<T, N> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        // Safety:
        // - We have ensured all elements from 0 to len have been initialized
        // - self.elem[0] came from a reference, and so is aligned to T
        // unsafe { &*(&self.buf[0..self.len] as *const [_] as *const [T]) }
        unsafe { slice::from_raw_parts(self.buf.as_ptr().cast(), self.len) }
    }
}

impl<T, const N: usize> DerefMut for Stack<T, N> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety:
        // - See Deref
        unsafe { slice::from_raw_parts_mut(self.buf.as_mut_ptr().cast(), self.len) }
    }
}

// requires dropck-eyepatch for elements with contravariant lifetimes
unsafe impl<#[may_dangle] T, const N: usize> Drop for Stack<T, N> {
    #[inline]
    fn drop(&mut self) {
        // Safety: We have ensured that all elements in the list are
        unsafe { core::ptr::drop_in_place(self.as_mut_slice()) };
    }
}

impl<T, const N: usize> Extend<T> for Stack<T, N> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for value in iter {
            self.push(value)
        }
    }
}

impl<T, const N: usize> From<[T; N]> for Stack<T, N> {
    fn from(value: [T; N]) -> Self {
        let value = ManuallyDrop::new(value);
        if std::mem::size_of::<[T; N]>() == 0 {
            // Safety: since [T; N] is zero-sized, and there are no other fields,
            // it should be okay to interpret N as Self
            unsafe { ptr::read(&N as *const _ as *const _) }
        } else {
            // Safety:
            // - `value` is ManuallyDrop, so its destructor won't run
            // - All elements are assumed to be initialized (so len is N)
            Self {
                buf: unsafe { ptr::read(&value as *const _ as *const _) },
                len: N,
                ..Default::default()
            }
        }
    }
}

impl<T, const N: usize> Stack<T, N> {
    /// Constructs a new, empty [Stack]
    ///
    /// # Examples
    ///
    /// ```
    /// # use cl_structures::stack::Stack;
    /// let mut v: Stack<_, 3> = Stack::new();
    ///
    /// v.try_push(1).unwrap();
    /// v.try_push(2).unwrap();
    /// v.try_push(3).unwrap();
    /// // Trying to push a 4th element will fail, and return the failed element
    /// assert_eq!(4, v.try_push(4).unwrap_err());
    ///
    /// assert_eq!(Some(3), v.pop());
    /// ```
    pub const fn new() -> Self {
        Self { buf: [const { MaybeUninit::uninit() }; N], len: 0, _data: PhantomData }
    }

    /// Constructs a new [Stack] from an array of [`MaybeUninit<T>`] and an initialized length
    ///
    /// # Safety
    ///
    /// - Elements from `0..len` must be initialized
    /// - len must not exceed the length of the array
    ///
    /// # Examples
    ///
    /// ```
    /// # use cl_structures::stack::Stack;
    /// # use core::mem::MaybeUninit;
    /// let mut v = unsafe { Stack::from_raw_parts([MaybeUninit::new(100)], 1) };
    ///
    /// assert_eq!(1, v.len());
    /// assert_eq!(1, v.capacity());
    /// assert_eq!(Some(100), v.pop());
    /// assert_eq!(None, v.pop());
    /// ```
    pub const unsafe fn from_raw_parts(buf: [MaybeUninit<T>; N], len: usize) -> Self {
        Self { buf, len, _data: PhantomData }
    }

    /// Converts a [Stack] into an array of [`MaybeUninit<T>`] and the initialized length
    ///
    /// # Examples
    ///
    /// ```
    /// # use cl_structures::stack::Stack;
    /// let mut v: Stack<_, 10> = Stack::new();
    /// v.push(0);
    /// v.push(1);
    ///
    /// let (buf, len) = v.into_raw_parts();
    ///
    /// assert_eq!(0, unsafe { buf[0].assume_init() });
    /// assert_eq!(1, unsafe { buf[1].assume_init() });
    /// assert_eq!(2, len);
    /// ```
    #[inline]
    pub fn into_raw_parts(self) -> ([MaybeUninit<T>; N], usize) {
        let this = ManuallyDrop::new(self);
        // Safety: since
        (unsafe { ptr::read(&this.buf) }, this.len)
    }

    /// Returns a raw pointer to the stack's buffer
    pub const fn as_ptr(&self) -> *const T {
        self.buf.as_ptr().cast()
    }

    /// Returns an unsafe mutable pointer to the stack's buffer
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.buf.as_mut_ptr().cast()
    }

    /// Extracts a slice containing the entire vector
    pub fn as_slice(&self) -> &[T] {
        self
    }

    /// Extracts a mutable slice containing the entire vector
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self
    }

    /// Returns the total number of elements the stack can hold
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Moves an existing stack into an allocation of a (potentially) different size,
    /// truncating if necessary.
    ///
    /// This can be used to easily construct a half-empty stack
    ///
    /// # Examples
    ///
    /// You can grow a stack to fit more elements
    /// ```
    /// # use cl_structures::stack::Stack;
    /// let v = Stack::from([0, 1, 2, 3, 4]);
    /// assert_eq!(5, v.capacity());
    ///
    /// let mut v = v.resize::<10>();
    /// assert_eq!(10, v.capacity());
    ///
    /// v.push(5);
    /// ```
    ///
    /// You can truncate a stack, dropping elements off the end
    /// ```
    /// # use cl_structures::stack::Stack;
    /// let v = Stack::from([0, 1, 2, 3, 4, 5, 6, 7]);
    /// assert_eq!(8, v.capacity());
    ///
    /// let v = v.resize::<5>();
    /// assert_eq!(5, v.capacity());
    /// ```
    pub fn resize<const M: usize>(mut self) -> Stack<T, M> {
        // Drop elements until new length is reached
        while self.len > M {
            drop(self.pop());
        }
        let (old, len) = self.into_raw_parts();
        let mut new: Stack<T, M> = Stack::new();

        // Safety:
        // - new and old are separate allocations
        // - len <= M
        unsafe {
            ptr::copy_nonoverlapping(old.as_ptr(), new.buf.as_mut_ptr(), len);
        }

        new.len = len;
        new
    }

    /// Push a new element onto the end of the stack
    ///
    /// # May Panic
    ///
    /// Panics if the new length exceeds capacity
    ///
    /// # Examples
    ///
    /// ```
    /// # use cl_structures::stack::Stack;
    /// let mut v: Stack<_, 4> = Stack::new();
    ///
    /// v.push(0);
    /// v.push(1);
    /// v.push(2);
    /// v.push(3);
    /// assert_eq!(&[0, 1, 2, 3], v.as_slice());
    /// ```
    pub fn push(&mut self, value: T) {
        if self.len >= N {
            panic!("Attempted to push into full stack")
        }
        // Safety: len is confirmed to be less than capacity
        unsafe { self.push_unchecked(value) };
    }

    /// Push a new element onto the end of the stack
    ///
    /// Returns [`Err(value)`](Result::Err) if the new length would exceed capacity
    pub fn try_push(&mut self, value: T) -> Result<(), T> {
        if self.len >= N {
            return Err(value);
        }
        // Safety: len is confirmed to be less than capacity
        unsafe { self.push_unchecked(value) };
        Ok(())
    }

    /// Push a new element onto the end of the stack, without checking capacity
    ///
    /// # Safety
    ///
    /// len after push must not exceed capacity N
    #[inline]
    unsafe fn push_unchecked(&mut self, value: T) {
        unsafe { ptr::write(self.as_mut_ptr().add(self.len), value) }
        self.len += 1; // post inc
    }

    /// Pops the last element off the end of the stack, and returns it
    ///
    /// Returns None if the stack is empty
    ///
    /// # Examples
    ///
    /// ```
    /// # use cl_structures::stack::Stack;
    /// let mut v = Stack::from([0, 1, 2, 3]);
    ///
    /// assert_eq!(Some(3), v.pop());
    /// assert_eq!(Some(2), v.pop());
    /// assert_eq!(Some(1), v.pop());
    /// assert_eq!(Some(0), v.pop());
    /// assert_eq!(None, v.pop());
    /// ```
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            // Safety: MaybeUninit<T> implies ManuallyDrop<T>,
            // therefore should not get dropped twice
            Some(unsafe { ptr::read(self.as_ptr().add(self.len).cast()) })
        }
    }

    /// Removes and returns the element at the given index,
    /// shifting other elements toward the start
    ///
    /// # Examples
    ///
    /// ```
    /// # use cl_structures::stack::Stack;
    /// let mut v = Stack::from([0, 1, 2, 3, 4]);
    ///
    /// assert_eq!(2, v.remove(2));
    /// assert_eq!(&[0, 1, 3, 4], v.as_slice());
    /// ```
    pub fn remove(&mut self, index: usize) -> T {
        if index >= self.len {
            panic!("Index {index} exceeded length {}", self.len)
        }
        let len = self.len - 1;
        let base = self.as_mut_ptr();
        let out = unsafe { ptr::read(base.add(index)) };

        unsafe { ptr::copy(base.add(index + 1), base.add(index), len - index) };
        self.len = len;
        out
    }

    /// Removes and returns the element at the given index,
    /// swapping it with the last element.
    ///
    /// # May Panic
    ///
    /// Panics if `index >= len`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cl_structures::stack::Stack;
    /// let mut v = Stack::from([0, 1, 2, 3, 4]);
    ///
    /// assert_eq!(2, v.swap_remove(2));
    ///
    /// assert_eq!(&[0, 1, 4, 3], v.as_slice());
    /// ```
    pub fn swap_remove(&mut self, index: usize) -> T {
        if index >= self.len {
            panic!("Index {index} exceeds length {}", self.len);
        }
        let len = self.len - 1;
        let ptr = self.as_mut_ptr();
        let out = unsafe { ptr::read(ptr.add(index)) };

        unsafe { ptr::copy(ptr.add(len), ptr.add(index), 1) };
        self.len = len;
        out
    }

    /// Inserts an element at position `index` in the stack,
    /// shifting all elements after it to the right.
    ///
    /// # May Panic
    ///
    /// Panics if `index > len` or [`self.is_full()`](Stack::is_full)
    ///
    /// # Examples
    ///
    /// ```
    /// # use cl_structures::stack::Stack;
    /// let mut v = Stack::from([0, 1, 2, 3, 4]).resize::<6>();
    ///
    /// v.insert(3, 0xbeef);
    /// assert_eq!(&[0, 1, 2, 0xbeef, 3, 4], v.as_slice());
    /// ```
    pub fn insert(&mut self, index: usize, data: T) {
        if index > self.len {
            panic!("Index {index} exceeded length {}", self.len)
        }
        if self.is_full() {
            panic!("Attempted to insert into full stack")
        }
        unsafe { self.insert_unchecked(index, data) };
    }

    /// Attempts to insert an element at position `index` in the stack,
    /// shifting all elements after it to the right.
    ///
    /// If the stack is at capacity, returns the original element and an [InsertFailed] error.
    ///
    /// # Examples
    ///
    /// ```
    /// use cl_structures::stack::Stack;
    /// let mut v: Stack<_, 2> = Stack::new();
    ///
    /// assert_eq!(Ok(()), v.try_insert(0, 0));
    /// ```
    pub fn try_insert(&mut self, index: usize, data: T) -> Result<(), (T, InsertFailed<N>)> {
        if index > self.len {
            return Err((data, InsertFailed::Bounds(index)));
        }
        if self.is_full() {
            return Err((data, InsertFailed::Full));
        }
        // Safety: index < self.len && !self.is_full()
        unsafe { self.insert_unchecked(index, data) };
        Ok(())
    }

    /// # Safety:
    /// - index must be less than self.len
    /// - length after insertion must be <= N
    #[inline]
    unsafe fn insert_unchecked(&mut self, index: usize, data: T) {
        let base = self.as_mut_ptr();

        unsafe { ptr::copy(base.add(index), base.add(index + 1), self.len - index) }

        self.len += 1;
        self.buf[index] = MaybeUninit::new(data);
    }

    /// Clears the stack
    ///
    /// # Examples
    ///
    /// ```
    /// # use cl_structures::stack::Stack;
    ///
    /// let mut v = Stack::from([0, 1, 2, 3, 4]);
    /// assert_eq!(v.as_slice(), &[0, 1, 2, 3, 4]);
    ///
    /// v.clear();
    /// assert_eq!(v.as_slice(), &[]);
    /// ```
    pub fn clear(&mut self) {
        // Hopefully copy elision takes care of this lmao
        drop(std::mem::take(self))
    }

    /// Returns the number of elements in the stack
    /// ```
    /// # use cl_structures::stack::*;
    /// let v = Stack::from([0, 1, 2, 3, 4]);
    ///
    /// assert_eq!(5, v.len());
    /// ```
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if the stack is at (or over) capacity
    ///
    /// # Examples
    ///
    /// ```
    /// # use cl_structures::stack::*;
    /// let v = Stack::from([(); 10]);
    ///
    /// assert!(v.is_full());
    /// ```
    #[inline]
    pub fn is_full(&self) -> bool {
        self.len >= N
    }

    /// Returns true if the stack contains no elements
    ///
    /// # Examples
    ///
    /// ```
    /// # use cl_structures::stack::*;
    /// let v: Stack<(), 10> = Stack::new();
    ///
    /// assert!(v.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InsertFailed<const N: usize> {
    Bounds(usize),
    Full,
}
impl<const N: usize> std::error::Error for InsertFailed<N> {}
impl<const N: usize> std::fmt::Display for InsertFailed<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsertFailed::Bounds(idx) => write!(f, "Index {idx} exceeded length {N}"),
            InsertFailed::Full => {
                write!(f, "Attempt to insert into full stack (length {N})")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_sized() {
        let v: Stack<(), { usize::MAX }> = Stack::new();
        assert_eq!(usize::MAX, v.capacity());
        assert_eq!(std::mem::size_of::<usize>(), std::mem::size_of_val(&v))
    }
    #[test]
    #[cfg_attr(debug_assertions, ignore = "calls ().drop() usize::MAX times")]
    fn from_usize_max_zst_array() {
        let mut v = Stack::from([(); usize::MAX]);
        assert_eq!(v.len(), usize::MAX);
        v.pop();
        assert_eq!(v.len(), usize::MAX - 1);
    }
    #[test]
    fn new() {
        let v: Stack<(), 255> = Stack::new();
        assert_eq!(0, v.len());
        assert_eq!(255, v.capacity());
    }

    #[test]
    fn push() {
        let mut v: Stack<_, 64> = Stack::new();
        v.push(10);
    }

    #[test]
    #[should_panic = "Attempted to push into full stack"]
    fn push_overflow() {
        let mut v = Stack::from([]);
        v.push(10);
    }

    #[test]
    fn pop() {
        let mut v = Stack::from([1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(Some(9), v.pop());
        assert_eq!(Some(8), v.pop());
        assert_eq!(Some(7), v.pop());
        assert_eq!(Some(6), v.pop());
        assert_eq!(Some(5), v.pop());
        assert_eq!(Some(4), v.pop());
        assert_eq!(Some(3), v.pop());
        assert_eq!(Some(2), v.pop());
        assert_eq!(Some(1), v.pop());
        assert_eq!(None, v.pop());
    }

    #[test]
    fn resize_smaller() {
        let v = Stack::from([1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let mut v = v.resize::<2>();

        assert_eq!(2, v.capacity());

        assert_eq!(Some(2), v.pop());
        assert_eq!(Some(1), v.pop());
        assert_eq!(None, v.pop());
    }

    #[test]
    fn resize_bigger() {
        let v = Stack::from([1, 2, 3, 4]);
        let mut v: Stack<_, 10> = v.resize();

        assert_eq!(Some(4), v.pop());
        assert_eq!(Some(3), v.pop());
        assert_eq!(Some(2), v.pop());
        assert_eq!(Some(1), v.pop());
        assert_eq!(None, v.pop());
    }
    #[test]
    fn dangle() {
        let mut v: Stack<_, 2> = Stack::new();
        let a = 0;
        let b = 1;
        v.push(&a);
        v.push(&b);
        println!("{v:?}");
    }
    #[test]
    fn remove() {
        let mut v = Stack::from([0, 1, 2, 3, 4, 5]);

        assert_eq!(3, v.remove(3));
        assert_eq!(4, v.remove(3));
        assert_eq!(5, v.remove(3));
        assert_eq!(Some(2), v.pop());
        assert_eq!(Some(1), v.pop());
        assert_eq!(Some(0), v.pop());
        assert_eq!(None, v.pop());
    }

    #[test]
    fn swap_remove() {
        let mut v = Stack::from([0, 1, 2, 3, 4, 5]);
        assert_eq!(3, v.swap_remove(3));
        assert_eq!(&[0, 1, 2, 5, 4], v.as_slice());
    }
    #[test]
    fn swap_remove_last() {
        let mut v = Stack::from([0, 1, 2, 3, 4, 5]);
        assert_eq!(5, v.swap_remove(5));
        assert_eq!(&[0, 1, 2, 3, 4], v.as_slice())
    }

    #[test]
    fn insert() {
        let mut v = Stack::from([0, 1, 2, 4, 5, 0x41414141]);
        v.pop();
        v.insert(3, 3);

        assert_eq!(&[0, 1, 2, 3, 4, 5], v.as_slice())
    }

    #[test]
    #[should_panic = "Attempted to insert into full stack"]
    fn insert_overflow() {
        let mut v = Stack::from([0]);
        v.insert(0, 1);
    }

    #[test]
    fn drop() {
        let v = Stack::from([
            Box::new(0),
            Box::new(1),
            Box::new(2),
            Box::new(3),
            Box::new(4),
        ]);
        std::mem::drop(std::hint::black_box(v));
    }
}
