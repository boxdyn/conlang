//! Reserves words upfront, rather than locking the global interner every time

use cl_ast::types::Symbol;

reserve_words! {
    /// A nominally immutable data structure containing globally-interned symbols
    /// which are important to the semantics of Conlang
    #[derive(Clone, Debug)]
    #[allow(non_snake_case)]
    pub struct ReservedWords {
        // soft keywords
        underscore: "_",
        kw_self: "self",
        kw_Self: "Self",
        kw_super: "super",

        // data structures
        kw_fn: "fn";
        unit,
        bool,
        char,
        str,
        never,
        f32,
        f64,
        i8,
        i16,
        i32,
        i64,
        i128,
        isize,
        int,
        u8,
        u16,
        u32,
        u64,
        u128,
        usize,
        uint,
        Ok,
        Err,
        Some,
        None,
        RangeInc,
        RangeExc,
        RangeTo,
        RangeToInc,

        // temporary soft keywords
        modules_enabled,
        SIGNED,
        SIZE,
        BITS,
        MIN,
        MAX,
        ARITY,
        TYPES,
        NAMES,
        MEMBERS,
        VARIANTS,
        COUNT,

        // operator overloads
        call,
        index,
        cmp,
        mul,
        div,
        rem,
        add,
        sub,
        shl,
        shr,
        and,
        or,
        xor,
        neg,
        not,
    }

    impl ReservedWords {
        /// Constructs a new [`ReservedWords`]
        pub fn new() -> Self {
            Self::default()
        }
    }
}

#[test]
fn contains_words() {
    let words = ReservedWords::new();
    assert_eq!(words.underscore, "_".into());
    assert_eq!(words.index, "index".into());
}

macro reserve_words(
    $(#[$($outer_meta:tt)*])*
    pub struct $Name:ident {
        $($(#[$($raw_meta:tt)*])*
        $raw_symbol:ident : $raw_value: literal ),*$(;
        $($(#[$($standard_meta:tt)*])*
        $symbol:ident                           ),*)?$(,)?
    }

    $($t:tt)* // allow arbitrary code at the same indentation
) {
    $(#[$($outer_meta)*])*
    pub struct $Name {
        $(  $(#[$($raw_meta)*])*
        pub $raw_symbol: Symbol,    )*
        $($($(#[$($standard_meta)*])*
        pub $symbol: Symbol,        )*)?
    }

    impl Default for ReservedWords {
        fn default() -> Self {
            Self {
                $($raw_symbol: $raw_value.into(),)*
                $($($symbol: stringify!($symbol).into(),)*)?
            }
        }
    }

    $($t)*
}
