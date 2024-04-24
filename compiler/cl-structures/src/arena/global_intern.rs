//!  A global intern pool for strings, represented by the [GlobalSym] symbol

use super::{intern::Interner, symbol::Symbol};
use std::{
    fmt::Display,
    num::NonZeroU32,
    sync::{OnceLock, RwLock},
};

/// Holds a globally accessible [Interner] which uses [GlobalSym] as its [Symbol]
static GLOBAL_INTERNER: OnceLock<RwLock<Interner<GlobalSym>>> = OnceLock::new();

/// A unique identifier corresponding to a particular interned [String].
///
/// Copies of that string can be obtained with [GlobalSym::get] or [String::try_from].
///
/// New strings can be interned with [GlobalSym::new] or [GlobalSym::from]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlobalSym(NonZeroU32);

impl GlobalSym {
    /// Gets the interned [GlobalSym] for the given value, or interns a new one.
    ///
    /// # Blocks
    /// This conversion blocks if the Global Interner lock is held.
    ///
    /// # May Panic
    /// Panics if the Global Interner's lock has been poisoned by a panic in another thread
    pub fn new(value: &str) -> Self {
        GLOBAL_INTERNER
            .get_or_init(Default::default)
            .write()
            .expect("global interner should not be poisoned in another thread")
            .get_or_insert(value)
    }
    /// Gets a [GlobalSym] associated with the given string, if one already exists
    pub fn try_from_str(value: &str) -> Option<Self> {
        GLOBAL_INTERNER.get()?.read().ok()?.get(value)
    }

    /// Gets a copy of the value of the [GlobalSym]
    // TODO: Make this copy-less
    pub fn get(self) -> Option<String> {
        String::try_from(self).ok()
    }

    /// Looks up the string associated with this [GlobalSym],
    /// and performs a transformation on it if it exists.
    pub fn map<T>(&self, f: impl Fn(&str) -> T) -> Option<T> {
        Some(f(GLOBAL_INTERNER.get()?.read().ok()?.get_str(*self)?))
    }
}

impl Display for GlobalSym {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Some(interner) = GLOBAL_INTERNER.get() else {
            return write!(f, "[sym@{} (uninitialized)]", self.0);
        };
        let Ok(interner) = interner.read() else {
            return write!(f, "[sym@{} (poisoned)]", self.0);
        };
        let Some(str) = interner.get_str(*self) else {
            return write!(f, "[sym@{} (invalid)]", self.0);
        };
        str.fmt(f)
    }
}

impl Symbol for GlobalSym {
    const MAX: usize = u32::MAX as usize - 1;
    fn try_from_usize(value: usize) -> Option<Self> {
        Some(Self(NonZeroU32::try_from_usize(value)?))
    }
    fn into_usize(self) -> usize {
        self.0.into_usize()
    }
}

impl<T: AsRef<str>> From<T> for GlobalSym {
    /// Converts to this type from the input type.
    ///
    /// # Blocks
    /// This conversion blocks if the Global Interner lock is held.
    ///
    /// # May Panic
    /// Panics if the Global Interner's lock has been poisoned by a panic in another thread
    fn from(value: T) -> Self {
        Self::new(value.as_ref())
    }
}

impl TryFrom<GlobalSym> for String {
    type Error = GlobalSymError;

    fn try_from(value: GlobalSym) -> Result<Self, Self::Error> {
        let Some(interner) = GLOBAL_INTERNER.get() else {
            Err(GlobalSymError::Uninitialized)?
        };
        let Ok(interner) = interner.write() else {
            Err(GlobalSymError::Poisoned)?
        };
        match interner.get_str(value) {
            None => Err(GlobalSymError::Unseen(value)),
            Some(string) => Ok(string.into()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobalSymError {
    Uninitialized,
    Poisoned,
    Unseen(GlobalSym),
}
impl std::error::Error for GlobalSymError {}
impl Display for GlobalSymError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GlobalSymError::Uninitialized => "String pool was not initialized".fmt(f),
            GlobalSymError::Poisoned => "String pool was held by panicking thread".fmt(f),
            GlobalSymError::Unseen(sym) => {
                write!(f, "Symbol {sym:?} not present in String pool")
            }
        }
    }
}

#[cfg(test)]
mod tests;
