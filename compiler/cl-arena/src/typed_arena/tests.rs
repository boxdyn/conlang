use super::TypedArena;
    extern crate std;
    use std::{prelude::rust_2021::*, print, vec};
    #[test]
    fn pushing_to_arena() {
        let arena = TypedArena::new();
        let foo = arena.alloc("foo");
        let bar = arena.alloc("bar");
        let baz = arena.alloc("baz");

        assert_eq!("foo", *foo);
        assert_eq!("bar", *bar);
        assert_eq!("baz", *baz);
    }

    #[test]
    fn pushing_vecs_to_arena() {
        let arena = TypedArena::new();

        let foo = arena.alloc(vec!["foo"]);
        let bar = arena.alloc(vec!["bar"]);
        let baz = arena.alloc(vec!["baz"]);

        assert_eq!("foo", foo[0]);
        assert_eq!("bar", bar[0]);
        assert_eq!("baz", baz[0]);
    }

    #[test]
    fn pushing_zsts() {
        struct ZeroSized;
        impl Drop for ZeroSized {
            fn drop(&mut self) {
                print!("")
            }
        }

        let arena = TypedArena::new();

        for _ in 0..0x100 {
            arena.alloc(ZeroSized);
        }
    }

    #[test]
    fn pushing_nodrop_zsts() {
        struct ZeroSized;
        let arena = TypedArena::new();

        for _ in 0..0x1000 {
            arena.alloc(ZeroSized);
        }
    }
    #[test]
    fn resize() {
        let arena = TypedArena::new();

        for _ in 0..0x780 {
            arena.alloc(0u128);
        }
    }
