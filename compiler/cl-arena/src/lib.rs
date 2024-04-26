//! Typed and dropless arena allocation, paraphrased from [the Rust Compiler's `rustc_arena`](https://github.com/rust-lang/rust/blob/master/compiler/rustc_arena/src/lib.rs). See [LICENSE][1].
//!
//! An Arena Allocator is a type of allocator which provides stable locations for allocations within
//! itself for the entire duration of its lifetime.
//! 
//! [1]: https://raw.githubusercontent.com/rust-lang/rust/master/LICENSE-MIT

#![feature(dropck_eyepatch, new_uninit, strict_provenance)]
#![no_std]

extern crate alloc;

pub(crate) mod constants {
    //! Size constants for arena chunk growth
    pub(crate) const MIN_CHUNK: usize = 4096;
    pub(crate) const MAX_CHUNK: usize = 2 * 1024 * 1024;
}

mod chunk {
    //! An [ArenaChunk] contains a block of raw memory for use in arena allocators.
    use alloc::boxed::Box;
    use core::{
        mem::{self, MaybeUninit},
        ptr::{self, NonNull},
    };

    pub struct ArenaChunk<T> {
        pub(crate) mem: NonNull<[MaybeUninit<T>]>,
        pub(crate) filled: usize,
    }

    impl<T: Sized> ArenaChunk<T> {
        pub fn new(cap: usize) -> Self {
            let slice = Box::new_uninit_slice(cap);
            Self { mem: NonNull::from(Box::leak(slice)), filled: 0 }
        }

        /// Drops all elements inside self, and resets the filled count to 0
        ///
        /// # Safety
        ///
        /// The caller must ensure that `self.filled` elements of self are currently initialized
        pub unsafe fn drop_elements(&mut self) {
            if mem::needs_drop::<T>() {
                // Safety: the caller has ensured that `filled` elements are initialized
                unsafe {
                    let slice = self.mem.as_mut();
                    for t in slice[..self.filled].iter_mut() {
                        t.assume_init_drop();
                    }
                }
                self.filled = 0;
            }
        }

        /// Gets a pointer to the start of the arena
        pub fn start(&mut self) -> *mut T {
            self.mem.as_ptr() as _
        }

        /// Gets a pointer to the end of the arena
        pub fn end(&mut self) -> *mut T {
            if mem::size_of::<T>() == 0 {
                ptr::without_provenance_mut(usize::MAX) // pointers to ZSTs must be unique
            } else {
                unsafe { self.start().add(self.mem.len()) }
            }
        }
    }

    impl<T> Drop for ArenaChunk<T> {
        fn drop(&mut self) {
            let _ = unsafe { Box::from_raw(self.mem.as_ptr()) };
        }
    }
}

pub mod typed_arena {
    //! A [TypedArena] can hold many instances of a single type, and will properly [Drop] them.
    use crate::{chunk::ArenaChunk, constants::*};
    use alloc::vec::Vec;
    use core::{
        cell::{Cell, RefCell},
        marker::PhantomData,
        mem, ptr,
    };

    /// A [TypedArena] can hold many instances of a single type, and will properly [Drop] them when
    /// it falls out of scope.
    pub struct TypedArena<T> {
        _drops: PhantomData<T>,
        chunks: RefCell<Vec<ArenaChunk<T>>>,
        head: Cell<*mut T>,
        tail: Cell<*mut T>,
    }

    impl<T> Default for TypedArena<T> {
        fn default() -> Self {
            Self {
                _drops: Default::default(),
                chunks: Default::default(),
                head: Cell::new(ptr::null_mut()),
                tail: Cell::new(ptr::null_mut()),
            }
        }
    }

    impl<T> TypedArena<T> {
        pub fn new() -> Self {
            Self::default()
        }

        #[allow(clippy::mut_from_ref)]
        pub fn alloc(&self, value: T) -> &mut T {
            if self.head == self.tail {
                self.grow(1);
            }

            let out = if mem::size_of::<T>() == 0 {
                self.head
                    .set(ptr::without_provenance_mut(self.head.get().addr() + 1));
                ptr::NonNull::<T>::dangling().as_ptr()
            } else {
                let out = self.head.get();
                self.head.set(unsafe { out.add(1) });
                out
            };

            unsafe {
                ptr::write(out, value);
                &mut *out
            }
        }

        #[cold]
        #[inline(never)]
        fn grow(&self, len: usize) {
            let size = mem::size_of::<T>().max(1);

            let mut chunks = self.chunks.borrow_mut();

            let capacity = if let Some(last) = chunks.last_mut() {
                last.filled = self.get_filled_of_chunk(last);
                last.mem.len().min(MAX_CHUNK / size) * 2
            } else {
                MIN_CHUNK / size
            }
            .max(len);

            let mut chunk = ArenaChunk::<T>::new(capacity);

            self.head.set(chunk.start());
            self.tail.set(chunk.end());
            chunks.push(chunk);
        }

        fn get_filled_of_chunk(&self, chunk: &mut ArenaChunk<T>) -> usize {
            let Self { head: tail, .. } = self;
            let head = chunk.start();
            if mem::size_of::<T>() == 0 {
                tail.get().addr() - head.addr()
            } else {
                unsafe { tail.get().offset_from(head) as usize }
            }
        }
    }

    unsafe impl<T: Send> Send for TypedArena<T> {}

    unsafe impl<#[may_dangle] T> Drop for TypedArena<T> {
        fn drop(&mut self) {
            let mut chunks = self.chunks.borrow_mut();

            if let Some(last) = chunks.last_mut() {
                last.filled = self.get_filled_of_chunk(last);
                self.tail.set(self.head.get());
            }

            for chunk in chunks.iter_mut() {
                unsafe { chunk.drop_elements() }
            }
        }
    }

    #[cfg(test)]
    mod tests;
}

pub mod dropless_arena {
    //! A [DroplessArena] can hold *any* combination of types as long as they don't implement
    //! [Drop].
    use crate::{chunk::ArenaChunk, constants::*};
    use alloc::vec::Vec;
    use core::{
        alloc::Layout,
        cell::{Cell, RefCell},
        mem, ptr, slice,
    };

    pub struct DroplessArena {
        chunks: RefCell<Vec<ArenaChunk<u8>>>,
        head: Cell<*mut u8>,
        tail: Cell<*mut u8>,
    }

    impl Default for DroplessArena {
        fn default() -> Self {
            Self {
                chunks: Default::default(),
                head: Cell::new(ptr::null_mut()),
                tail: Cell::new(ptr::null_mut()),
            }
        }
    }

    impl DroplessArena {
        pub fn new() -> Self {
            Self::default()
        }

        /// Allocates a `T` in the [DroplessArena], and returns a mutable reference to it.
        ///
        /// # Panics
        /// - Panics if T implements [Drop]
        /// - Panics if T is zero-sized
        #[allow(clippy::mut_from_ref)]
        pub fn alloc<T>(&self, value: T) -> &mut T {
            assert!(!mem::needs_drop::<T>());
            assert!(mem::size_of::<T>() != 0);

            let out = self.alloc_raw(Layout::new::<T>()) as *mut T;

            unsafe {
                ptr::write(out, value);
                &mut *out
            }
        }

        /// Allocates a slice of `T`s`, copied from the given slice, returning a mutable reference
        /// to it.
        ///
        /// # Panics
        /// - Panics if T implements [Drop]
        /// - Panics if T is zero-sized
        /// - Panics if the slice is empty
        #[allow(clippy::mut_from_ref)]
        pub fn alloc_slice<T: Copy>(&self, slice: &[T]) -> &mut [T] {
            assert!(!mem::needs_drop::<T>());
            assert!(mem::size_of::<T>() != 0);
            assert!(!slice.is_empty());

            let mem = self.alloc_raw(Layout::for_value::<[T]>(slice)) as *mut T;

            unsafe {
                mem.copy_from_nonoverlapping(slice.as_ptr(), slice.len());
                slice::from_raw_parts_mut(mem, slice.len())
            }
        }

        /// Allocates a copy of the given [`&str`](str), returning a reference to the allocation.
        ///
        /// # Panics
        /// Panics if the string is empty.
        pub fn alloc_str(&self, string: &str) -> &str {
            let slice = self.alloc_slice(string.as_bytes());

            // Safety: This is a clone of the input string, which was valid
            unsafe { core::str::from_utf8_unchecked(slice) }
        }

        /// Allocates some [bytes](u8) based on the given [Layout].
        ///
        /// # Panics
        /// Panics if the provided [Layout] has size 0
        pub fn alloc_raw(&self, layout: Layout) -> *mut u8 {
            /// Rounds the given size (or pointer value) *up* to the given alignment
            fn align_up(size: usize, align: usize) -> usize {
                (size + align - 1) & !(align - 1)
            }
            /// Rounds the given size (or pointer value) *down* to the given alignment
            fn align_down(size: usize, align: usize) -> usize {
                size & !(align - 1)
            }

            assert!(layout.size() != 0);
            loop {
                let Self { head, tail, .. } = self;
                let start = head.get().addr();
                let end = tail.get().addr();

                let align = 8.max(layout.align());

                let bytes = align_up(layout.size(), align);

                if let Some(end) = end.checked_sub(bytes) {
                    let end = align_down(end, layout.align());

                    if start <= end {
                        tail.set(tail.get().with_addr(end));
                        return tail.get();
                    }
                }

                self.grow(layout.size());
            }
        }

        /// Grows the allocator, doubling the chunk size until it reaches [MAX_CHUNK].
        #[cold]
        #[inline(never)]
        fn grow(&self, len: usize) {
            let mut chunks = self.chunks.borrow_mut();

            let capacity = if let Some(last) = chunks.last_mut() {
                last.mem.len().min(MAX_CHUNK / 2) * 2
            } else {
                MIN_CHUNK
            }
            .max(len);

            let mut chunk = ArenaChunk::<u8>::new(capacity);

            self.head.set(chunk.start());
            self.tail.set(chunk.end());
            chunks.push(chunk);
        }

        /// Checks whether the given slice is allocated in this arena
        pub fn contains_slice<T>(&self, slice: &[T]) -> bool {
            let ptr = slice.as_ptr().cast::<u8>().cast_mut();
            for chunk in self.chunks.borrow_mut().iter_mut() {
                if chunk.start() <= ptr && ptr <= chunk.end() {
                    return true;
                }
            }
            false
        }
    }

    unsafe impl Send for DroplessArena {}

    #[cfg(test)]
    mod tests;
}
