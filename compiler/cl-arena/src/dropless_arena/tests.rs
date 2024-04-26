use super::DroplessArena;
    extern crate std;
    use core::alloc::Layout;
    use std::{prelude::rust_2021::*, vec};

    #[test]
    fn alloc_raw() {
        let arena = DroplessArena::new();
        let bytes = arena.alloc_raw(Layout::for_value(&0u128));
        let byte2 = arena.alloc_raw(Layout::for_value(&0u128));

        assert_ne!(bytes, byte2);
    }

    #[test]
    fn alloc() {
        let arena = DroplessArena::new();
        let mut allocations = vec![];
        for i in 0..0x400 {
            allocations.push(arena.alloc(i));
        }
    }

    #[test]
    fn alloc_strings() {
        const KW: &[&str] = &["pub", "mut", "fn", "mod", "conlang", "sidon", "🦈"];
        let arena = DroplessArena::new();
        let mut allocations = vec![];
        for _ in 0..100 {
            for kw in KW {
                allocations.push(arena.alloc_str(kw));
            }
        }
    }

    #[test]
    #[should_panic]
    fn alloc_zsts() {
        struct Zst;
        let arena = DroplessArena::new();
        arena.alloc(Zst);
    }
