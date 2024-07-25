
//! Tests for funky behavior
use super::preamble::*;

struct UnitLike;

struct TupleLike(super::num::i32, super::self::test::char)

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

mod horrible_imports {
    mod foo {
        use super::{bar::*, baz::*};
        struct Foo(&Foo, &Bar)
    }
    mod bar {
        use super::{foo::*, baz::*};
        struct Bar(&Foo, &Baz)
    }
    mod baz {
        use super::{foo::*, bar::*};
        struct Baz(&Foo, &Bar)
    }
}
