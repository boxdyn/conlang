//! Compactly stores a set of immutable strings, producing a [Symbol] for each one
use super::symbol::Symbol;
use std::marker::PhantomData;
/// Compactly stores a set of immutable strings, producing a [Symbol] for each one
#[derive(Debug)]
pub struct StringArena<T: Symbol> {
    ends: Vec<usize>,
    buf: String,
    _t: PhantomData<fn(T)>,
}

impl<T: Symbol> StringArena<T> {
    pub fn new() -> Self {
        Default::default()
    }
    /// # May panic
    /// Panics if Symbol::from_usize would panic
    fn next_key(&self) -> T {
        Symbol::from_usize(self.ends.len())
    }

    fn get_span(&self, key: T) -> Option<(usize, usize)> {
        let key = key.into_usize();
        Some((*self.ends.get(key - 1)?, *self.ends.get(key)?))
    }

    pub fn get(&self, key: T) -> Option<&str> {
        let (start, end) = self.get_span(key)?;
        // Safety: start and end offsets were created by push_string
        Some(unsafe { self.buf.get_unchecked(start..end) })
    }

    pub fn push_string(&mut self, s: &str) -> T {
        if self.ends.is_empty() {
            self.ends.push(self.buf.len())
        }
        let key = self.next_key();
        self.buf.push_str(s);
        self.ends.push(self.buf.len());
        key
    }
}

impl<T: Symbol> Default for StringArena<T> {
    fn default() -> Self {
        Self { ends: Default::default(), buf: Default::default(), _t: PhantomData }
    }
}
