//! The primitive numeric types

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
