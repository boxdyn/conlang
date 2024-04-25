//! A string interner with deduplication

use super::{string_arena::StringArena, symbol::Symbol};
use hashbrown::hash_table::HashTable;
use std::hash::{BuildHasher, RandomState};

#[derive(Debug)]
pub struct Interner<Sym: Symbol, H: BuildHasher = RandomState> {
    set: HashTable<Sym>,
    arena: StringArena<Sym>,
    hasher: H,
}

impl<Sym: Symbol, H: BuildHasher + Default> Default for Interner<Sym, H> {
    fn default() -> Self {
        Self { set: Default::default(), arena: Default::default(), hasher: Default::default() }
    }
}

impl<Sym: Symbol, H: BuildHasher> Interner<Sym, H> {
    pub fn get_or_insert(&mut self, s: &str) -> Sym {
        let Self { set: map, arena, hasher } = self;
        let hash = hasher.hash_one(s);
        *map.entry(hash, is_match(s, arena), |t| {
            hasher.hash_one(arena.get(*t).unwrap())
        })
        .or_insert_with(|| arena.push_string(s))
        .get()
    }

    pub fn get(&self, s: &str) -> Option<Sym> {
        let Self { set: map, arena, hasher } = self;
        map.find(hasher.hash_one(s), is_match(s, arena)).copied()
    }

    pub fn get_str(&self, sym: Sym) -> Option<&str> {
        self.arena.get(sym)
    }
}

fn is_match<'a, Sym: Symbol>(
    s: &'a str,
    arena: &'a StringArena<Sym>,
) -> impl Fn(&Sym) -> bool + 'a {
    move |sym| match arena.get(*sym) {
        Some(sym) => sym == s,
        None => false,
    }
}
