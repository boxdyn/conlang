//!  A global intern pool for strings, represented by the [GlobalSym] symbol

use super::{intern::Interner, symbol::Symbol};
use std::{
    fmt::Display,
    num::NonZeroU32,
    sync::{OnceLock, RwLock},
};

static GLOBAL_INTERNER: OnceLock<RwLock<Interner<GlobalSym>>> = OnceLock::new();

/// Gets the [GlobalSym] associated with this string, if there is one, or creates a new one
///
/// # Blocks
/// Locks the Global Interner for writing. If it is already locked, 
/// # May Panic
/// T
pub fn get_or_insert(s: &str) -> GlobalSym {
    GLOBAL_INTERNER
        .get_or_init(Default::default)
        .write()
        .expect("global interner should not have been held by a panicked thread")
        .get_or_insert(s)
}

/// Gets the [GlobalSym] associated with this string, if there is one
pub fn get(s: &str) -> Option<GlobalSym> {
    GLOBAL_INTERNER.get()?.read().ok()?.get(s)
}

/// Gets the [String] associated with this [GlobalSym], if there is one
///
/// Returns none if the global symbol table is poisoned.
pub fn get_string(sym: GlobalSym) -> Option<String> {
    sym.try_into().ok()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlobalSym(NonZeroU32);

impl GlobalSym {
    /// Gets a [GlobalSym] associated with the given string, if one exists
    pub fn try_from_str(value: &str) -> Option<Self> {
        GLOBAL_INTERNER.get()?.read().ok()?.get(value)
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

impl From<&str> for GlobalSym {
    /// Converts to this type from the input type.
    ///
    /// # Blocks
    /// This conversion blocks if the Global Interner lock is held.
    ///
    /// # May Panic
    /// Panics if the Global Interner's lock has been poisoned by a panic in another thread
    fn from(value: &str) -> Self {
        GLOBAL_INTERNER
            .get_or_init(Default::default)
            .write()
            .expect("global interner should not be poisoned in another thread")
            .get_or_insert(value)
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
