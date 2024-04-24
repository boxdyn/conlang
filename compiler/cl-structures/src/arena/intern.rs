//! A string interner with deduplication

use super::{symbol::Symbol, StringArena};
use std::{
    collections::{hash_map::RawEntryMut, HashMap},
    hash::{BuildHasher, RandomState},
};

#[derive(Debug)]
pub struct Interner<T: Symbol, H: BuildHasher = RandomState> {
    map: HashMap<T, ()>,
    arena: StringArena<T>,
    hasher: H,
}

impl<T: Symbol, H: BuildHasher + Default> Default for Interner<T, H> {
    fn default() -> Self {
        Self { map: Default::default(), arena: Default::default(), hasher: Default::default() }
    }
}

impl<T: Symbol, H: BuildHasher> Interner<T, H> {
    pub fn get_or_insert(&mut self, s: &str) -> T {
        let Self { map, arena, hasher } = self;
        let hash = hasher.hash_one(s);
        match map.raw_entry_mut().from_hash(hash, is_match(s, arena)) {
            RawEntryMut::Occupied(entry) => *entry.into_key(),
            RawEntryMut::Vacant(entry) => {
                let tok = arena.push_string(s);
                *(entry.insert_hashed_nocheck(hash, tok, ()).0)
            }
        }
    }

    pub fn get(&self, s: &str) -> Option<T> {
        let Self { map, arena, hasher } = self;
        map.raw_entry()
            .from_hash(hasher.hash_one(s), is_match(s, arena))
            .map(|entry| *entry.0)
    }

    pub fn get_str(&self, sym: T) -> Option<&str> {
        self.arena.get(sym)
    }
}

fn is_match<'a, T: Symbol>(
    target: &'a str,
    arena: &'a StringArena<T>,
) -> impl Fn(&T) -> bool + 'a {
    move |sym| match arena.get(*sym) {
        Some(sym) => sym == target,
        None => false,
    }
}
