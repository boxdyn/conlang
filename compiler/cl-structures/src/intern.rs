//! Interners for [strings](string_interner) and non-[Drop] [types](dropless_interner).
//!
//! An object is [Interned][1] if it is allocated within one of the interners
//! in this module. [Interned][1] values have referential equality semantics, and
//! [Deref](std::ops::Deref) to the value within their respective intern pool.
//!
//! This means, of course, that the same value interned in two different pools will be
//! considered *not equal* by [Eq] and [Hash](std::hash::Hash).
//!
//! [1]: interned::Interned

pub mod interned {
    //! An [Interned] reference asserts its wrapped value has referential equality.
    use super::string_interner::StringInterner;
    use std::{
        fmt::{Debug, Display},
        hash::Hash,
        ops::Deref,
    };

    /// An [Interned] value is one that is *referentially comparable*.
    /// That is, the interned value is unique in memory, simplifying
    /// its equality and hashing implementation.
    ///
    /// Comparing [Interned] values via [PartialOrd] or [Ord] will still
    /// dereference to the wrapped pointers, and as such, may produce
    /// results inconsistent with [PartialEq] or [Eq].
    #[repr(transparent)]
    #[expect(private_interfaces)]
    pub struct Interned<'a, T: ?Sized>(pub &'a T, pub Private);

    pub type Symbol = Interned<'static, str>;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct Private();

    impl<'a, T: ?Sized> Interned<'a, T> {
        /// Gets the internal value as a pointer
        pub fn as_ptr(interned: &Self) -> *const T {
            interned.0
        }

        /// Gets the internal value as a reference with the interner's lifetime
        pub fn to_ref(&self) -> &'a T {
            self.0
        }
    }

    impl<'a, T: ?Sized> Interned<'a, T> {
        pub(super) fn new(value: &'a T) -> Self {
            Self(value, Private())
        }
    }

    impl<T: ?Sized> Copy for Interned<'_, T> {}

    impl<T: ?Sized> Clone for Interned<'_, T> {
        fn clone(&self) -> Self {
            *self
        }
    }

    impl<T: ?Sized + Debug> Debug for Interned<'_, T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("~")?;
            self.0.fmt(f)
        }
    }

    impl<T: ?Sized> Deref for Interned<'_, T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            self.0
        }
    }

    impl<T: ?Sized> PartialEq for Interned<'_, T> {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self.0, other.0)
        }
    }

    impl<T: ?Sized> Eq for Interned<'_, T> {}

    impl<T: ?Sized + PartialOrd> PartialOrd for Interned<'_, T> {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            self.0.partial_cmp(other.0)
        }
    }

    impl<T: ?Sized + Ord> Ord for Interned<'_, T> {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.0.cmp(other.0)
        }
    }

    impl<T: ?Sized> Hash for Interned<'_, T> {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            Self::as_ptr(self).hash(state)
        }
    }

    impl<T: ?Sized + Display> Display for Interned<'_, T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    impl<'a> AsRef<str> for Interned<'a, str> {
        fn as_ref(&self) -> &str {
            self.0
        }
    }

    impl From<&str> for Symbol {
        /// Types which implement [`AsRef<str>`] will be stored in the global [StringInterner]
        fn from(value: &str) -> Self {
            from_str(value)
        }
    }

    impl Default for Symbol {
        fn default() -> Self {
            from_str("")
        }
    }

    fn from_str(value: &str) -> Symbol {
        let global_interner = StringInterner::global();
        global_interner.get_or_insert(value)
    }
}

pub mod string_interner {
    //! A [StringInterner] hands out [Interned] copies of each unique string given to it.

    use super::interned::Interned;
    use cl_arena::dropless_arena::DroplessArena;
    use std::{
        collections::HashSet,
        sync::{OnceLock, RwLock},
    };

    /// A string interner hands out [Interned] copies of each unique string given to it.
    #[derive(Default)]
    pub struct StringInterner<'a> {
        arena: DroplessArena<'a>,
        keys: RwLock<HashSet<&'a str>>,
    }

    impl StringInterner<'static> {
        /// Gets a reference to a global string interner whose [Interned] strings are `'static`
        pub fn global() -> &'static Self {
            static GLOBAL_INTERNER: OnceLock<StringInterner<'static>> = OnceLock::new();

            // SAFETY: The RwLock within the interner's `keys` protects the arena
            // from being modified concurrently.
            GLOBAL_INTERNER.get_or_init(|| StringInterner {
                arena: DroplessArena::new(),
                keys: Default::default(),
            })
        }
    }

    impl<'a> StringInterner<'a> {
        /// Creates a new [StringInterner] backed by the provided [DroplessArena]
        pub fn new(arena: DroplessArena<'a>) -> Self {
            Self { arena, keys: RwLock::new(HashSet::new()) }
        }

        /// Returns an [Interned] copy of the given string,
        /// allocating a new one if it doesn't already exist.
        ///
        /// # Blocks
        /// This function blocks when the interner is held by another thread.
        pub fn get_or_insert(&'a self, value: &str) -> Interned<'a, str> {
            let Self { arena, keys } = self;

            // Safety: Holding this write guard for the entire duration of this
            // function enforces a safety invariant. See StringInterner::global.
            let mut keys = keys.write().expect("should not be poisoned");

            Interned::new(match keys.get(value) {
                Some(value) => value,
                None => {
                    let value = match value {
                        "" => "", // Arena will panic if passed an empty string
                        _ => arena.alloc_str(value),
                    };
                    keys.insert(value);
                    value
                }
            })
        }
        /// Gets a reference to the interned copy of the given value, if it exists
        /// # Blocks
        /// This function blocks when the interner is held by another thread.
        pub fn get(&'a self, value: &str) -> Option<Interned<'a, str>> {
            let keys = self.keys.read().expect("should not be poisoned");
            keys.get(value).copied().map(Interned::new)
        }
    }

    impl std::fmt::Debug for StringInterner<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Interner")
                .field("keys", &self.keys)
                .finish()
        }
    }

    impl std::fmt::Display for StringInterner<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Ok(keys) = self.keys.read() else {
                return write!(f, "Could not lock StringInterner key map.");
            };
            let mut keys: Vec<_> = keys.iter().collect();
            keys.sort();

            writeln!(f, "Keys:")?;
            for (idx, key) in keys.iter().enumerate() {
                writeln!(f, "{idx}:\t\"{key}\"")?
            }
            writeln!(f, "Count: {}", keys.len())?;

            Ok(())
        }
    }

    // # Safety:
    // This is fine because StringInterner::get_or_insert(v) holds a RwLock
    // for its entire duration, and doesn't touch the non-(Send+Sync) arena
    // unless the lock is held by a write guard.
    unsafe impl Send for StringInterner<'_> {}
    unsafe impl Sync for StringInterner<'_> {}

    #[cfg(test)]
    mod tests {
        use super::StringInterner;

        macro_rules! ptr_eq {
        ($a: expr, $b: expr $(, $($t:tt)*)?) => {
            assert_eq!(std::ptr::addr_of!($a), std::ptr::addr_of!($b) $(, $($t)*)?)
        };
    }
        macro_rules! ptr_ne {
        ($a: expr, $b: expr $(, $($t:tt)*)?) => {
            assert_ne!(std::ptr::addr_of!($a), std::ptr::addr_of!($b) $(, $($t)*)?)
        };
    }

        #[test]
        fn empties_is_unique() {
            let interner = StringInterner::global();
            let empty = interner.get_or_insert("");
            let empty2 = interner.get_or_insert("");
            ptr_eq!(*empty, *empty2);
        }
        #[test]
        fn non_empty_is_unique() {
            let interner = StringInterner::global();
            let nonempty1 = interner.get_or_insert("not empty!");
            let nonempty2 = interner.get_or_insert("not empty!");
            let different = interner.get_or_insert("different!");
            ptr_eq!(*nonempty1, *nonempty2);
            ptr_ne!(*nonempty1, *different);
        }
    }
}

pub mod dropless_interner {
    //! A [DroplessInterner] hands out [Interned] references for arbitrary types.
    //!
    //! Note: It is a *logic error* to modify the returned reference via interior mutability
    //! in a way that changes the values produced by [Eq] and [Hash].
    //!
    //! See the standard library [HashSet] for more details.
    //!
    //! ```rust
    //! use doughlang::intern::dropless_interner::DroplessInterner;
    //! use cl_arena::dropless_arena::DroplessArena;
    //!
    //! let da = DroplessArena::new();
    //! let di: DroplessInterner<'_, ()> = DroplessInterner::new(da);
    //! let unit1 = di.get_or_insert(());
    //! let unit2 = di.get_or_insert(());
    //! let unit3 = di.get(&()).unwrap();
    //! assert_eq!(unit1, unit3);
    //! assert_eq!(unit2, unit1);
    //! assert_eq!(unit3, unit2);
    //! ```
    //!
    //! ```rust
    //! use doughlang::intern::dropless_interner::DroplessInterner;
    //! use cl_arena::dropless_arena::DroplessArena;
    //!
    //! let da = DroplessArena::new();
    //! let di: DroplessInterner<'_, [i32; 10]> = DroplessInterner::new(da);
    //! let arr1 = di.get_or_insert([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    //! let arr2 = di.get_or_insert([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    //! let arr3 = di.get(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]).unwrap();
    //! assert_eq!(arr1, arr3);
    //! assert_eq!(arr2, arr1);
    //! assert_eq!(arr3, arr2);
    //!
    //! let arr4 = di.get_or_insert([10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
    //! assert_ne!(arr1, arr4);
    //! ```
    use super::interned::Interned;
    use cl_arena::dropless_arena::DroplessArena;
    use std::{collections::HashSet, hash::Hash, sync::RwLock};

    /// A [DroplessInterner] hands out [Interned] references for arbitrary types.
    ///
    /// See the [module-level documentation](self) for more information.
    pub struct DroplessInterner<'a, T: Eq + Hash> {
        arena: DroplessArena<'a>,
        keys: RwLock<HashSet<&'a T>>,
    }

    impl<'a, T: Eq + Hash> Default for DroplessInterner<'a, T> {
        fn default() -> Self {
            Self { arena: Default::default(), keys: Default::default() }
        }
    }

    impl<'a, T: Eq + Hash> DroplessInterner<'a, T> {
        /// Creates a new [DroplessInterner] backed by the provided [`DroplessArena`]
        ///
        /// # Panics
        /// Panics if T [needs drop](std::mem::needs_drop)
        pub fn new(arena: DroplessArena<'a>) -> Self {
            Self { arena, keys: RwLock::new(HashSet::new()) }
        }

        /// Converts the given value into an [Interned] value.
        ///
        /// # Blocks
        /// This function blocks when the interner is held by another thread.
        pub fn get_or_insert(&'a self, value: T) -> Interned<'a, T> {
            let Self { arena, keys } = self;

            // Safety: Locking the keyset for the entire duration of this function
            // enforces a safety invariant when the interner is stored in a global.
            let mut keys = keys.write().expect("should not be poisoned");

            Interned::new(match keys.get(&value) {
                Some(value) => value,
                None => {
                    let value = if std::mem::size_of::<T>() == 0 {
                        Box::leak(Box::new(value))
                    } else {
                        arena.alloc(value)
                    };
                    keys.insert(value);
                    value
                }
            })
        }

        /// Returns the [Interned] copy of the given value, if one already exists
        ///
        /// # Blocks
        /// This function blocks when the interner is being written to by another thread.
        pub fn get(&self, value: &T) -> Option<Interned<'a, T>> {
            let keys = self.keys.read().expect("should not be poisoned");
            keys.get(value).copied().map(Interned::new)
        }
    }

    /// # Safety
    /// This should be safe because references yielded by
    /// [get_or_insert](DroplessInterner::get_or_insert) are unique, and the function uses
    /// the [RwLock] around the [HashSet] to ensure mutual exclusion
    unsafe impl<'a, T: Eq + Hash + Send> Send for DroplessInterner<'a, T> where &'a T: Send {}
    unsafe impl<T: Eq + Hash + Send + Sync> Sync for DroplessInterner<'_, T> {}
}
