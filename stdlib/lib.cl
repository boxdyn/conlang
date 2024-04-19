//! # The Conlang Standard Library

#[intrinsic = "bool"]
pub type bool;

#[intrinsic = "char"]
pub type char;

#[intrinsic = "i8"]
pub type i8;

#[intrinsic = "i16"]
pub type i16;

#[intrinsic = "i32"]
pub type i32;

#[intrinsic = "i64"]
pub type i64;

#[intrinsic = "u8"]
pub type u8;

#[intrinsic = "u16"]
pub type u16;

#[intrinsic = "u32"]
pub type u32;

#[intrinsic = "u64"]
pub type u64;

impl bool {
    const MIN: Self = false;
    const MAX: Self = {
        fn ret_true() -> Self {
            true
        }
        ret_true()
    };
}

fn if_else() -> i32 {
    // block 1
    let x = 10;
    let y = x + 2;

    if x > 10
    // compare x and 10
    // end block 1, goto block 2 else goto block 3
    {
        // block 2
        4
        // end block 2, goto block 4
    } else {
        // block 3
        5
        // end block 3, goto block 4
    }
    // block 4
}

#[cfg("test")]
mod test {
    //! Tests for funky behavior

    struct UnitLike;

    struct TupleLike(super::i32, super::self::test::super::char)

    struct StructLike {
        pub member1: UnitLike,
        member2: TupleLike,
    }

    enum NeverLike;

    enum EmptyLike {
        Empty,
    }

    enum Hodgepodge {
        Empty,
        UnitLike (),
        Tuple (TupleLike),
        StructEmpty {},
        StructLike { member1: UnitLike, member2: TupleLike },
    }

    fn noop () -> bool {
        loop if false {
            
        } else break loop if false {
            
        } else break loop if false {
            
        } else break true;
    }

    fn while_else() -> i32 {
        while conditional {
            pass
        } else {
            fail
        }
    }
}
