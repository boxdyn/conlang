//!  A global intern pool for strings, represented by the [Sym] symbol

use super::{intern::Interner, symbol::Symbol};
use std::{
    fmt::Display,
    num::NonZeroU32,
    sync::{OnceLock, RwLock},
};

/// Holds a globally accessible [Interner] which uses [Sym] as its [Symbol]
static GLOBAL_INTERNER: OnceLock<RwLock<Interner<Sym>>> = OnceLock::new();

/// A unique identifier corresponding to a particular globally-interned [String].
///
/// Copies of that string can be obtained with [Sym::get] or [String::try_from].
///
/// New strings can be interned with [Sym::new] or [Sym::from]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Sym(NonZeroU32);

impl Sym {
    /// Gets the interned [Sym] for the given value, or interns a new one.
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
    /// Gets a [Sym] associated with the given string, if one already exists
    pub fn try_from_str(value: &str) -> Option<Self> {
        GLOBAL_INTERNER.get()?.read().ok()?.get(value)
    }

    /// Gets a copy of the value of the [Sym]
    // TODO: Make this copy-less
    pub fn get(self) -> Option<String> {
        String::try_from(self).ok()
    }

    /// Looks up the string associated with this [Sym],
    /// and performs a transformation on it if it exists.
    pub fn map<T>(&self, f: impl Fn(&str) -> T) -> Option<T> {
        Some(f(GLOBAL_INTERNER.get()?.read().ok()?.get_str(*self)?))
    }
}

impl Display for Sym {
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

impl Symbol for Sym {
    const MAX: usize = u32::MAX as usize - 1;
    fn try_from_usize(value: usize) -> Option<Self> {
        Some(Self(NonZeroU32::try_from_usize(value)?))
    }
    fn into_usize(self) -> usize {
        self.0.into_usize()
    }
}

impl<T: AsRef<str>> From<T> for Sym {
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

impl TryFrom<Sym> for String {
    type Error = SymError;

    fn try_from(value: Sym) -> Result<Self, Self::Error> {
        let Some(interner) = GLOBAL_INTERNER.get() else {
            Err(SymError::Uninitialized)?
        };
        let Ok(interner) = interner.write() else {
            Err(SymError::Poisoned)?
        };
        match interner.get_str(value) {
            None => Err(SymError::Unseen(value)),
            Some(string) => Ok(string.into()),
        }
    }
}

/// Describes an error in [Sym] to [String] lookup
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymError {
    Uninitialized,
    Poisoned,
    Unseen(Sym),
}
impl std::error::Error for SymError {}
impl Display for SymError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymError::Uninitialized => "String pool was not initialized".fmt(f),
            SymError::Poisoned => "String pool was held by panicking thread".fmt(f),
            SymError::Unseen(sym) => {
                write!(f, "Symbol {sym:?} not present in String pool")
            }
        }
    }
}

#[cfg(test)]
mod tests;
