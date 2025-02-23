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

#[intrinsic = "i128"]
pub type i128;

#[intrinsic = "isize"]
pub type isize;

#[intrinsic = "u8"]
pub type u8;

#[intrinsic = "u16"]
pub type u16;

#[intrinsic = "u32"]
pub type u32;

#[intrinsic = "u64"]
pub type u64;

#[intrinsic = "u128"]
pub type u128;

#[intrinsic = "usize"]
pub type usize;

#[intrinsic = "f32"]
pub type f32;

#[intrinsic = "f64"]
pub type f64;

// Contains implementations for (TODO) overloaded operators on num types
pub mod ops {
    use super::*;
    // Represents an ordering comparison verdict
    pub enum Ordering {
        Less,
        Equal,
        Greater,
    }

    impl bool {
        pub const MIN: Self = false;
        pub const MAX: Self = true;
        pub const BIT_WIDTH: u32 = 1;
        pub fn default() -> Self {
            false
        }
        pub fn and(a: Self, b: Self) -> Self {
            a & b
        }
        pub fn or(a: Self, b: Self) -> Self {
            a | b
        }
        pub fn xor(a: Self, b: Self) -> Self {
            a ^ b
        }
    }

    impl char {
        pub fn default() -> Self {
            '\u{1f988}'
        }
    }

    impl u8 {
        pub const MIN: Self = 0;
        pub const MAX: Self = !0;
        pub const BIT_WIDTH: u32 = 1;
        pub fn default() -> Self {
            0
        }
        pub fn mul(a: Self, b: Self) -> Self {
            a * b
        }
        pub fn div(a: Self, b: Self) -> Self {
            a / b
        }
        pub fn rem(a: Self, b: Self) -> Self {
            a % b
        }
        pub fn add(a: Self, b: Self) -> Self {
            a + b
        }
        pub fn sub(a: Self, b: Self) -> Self {
            a - b
        }
        pub fn shl(a: Self, b: u32) -> Self {
            a << b
        }
        pub fn shr(a: Self, b: u32) -> Self {
            a >> b
        }
        pub fn and(a: Self, b: Self) -> Self {
            a & b
        }
        pub fn or(a: Self, b: Self) -> Self {
            a | b
        }
        pub fn xor(a: Self, b: Self) -> Self {
            a ^ b
        }
    }

    impl u16 {
        pub const MIN: Self = 0;
        pub const MAX: Self = !0;
        pub const BIT_WIDTH: u32 = 2;
        pub fn default() -> Self {
            0
        }
        pub fn mul(a: Self, b: Self) -> Self {
            a * b
        }
        pub fn div(a: Self, b: Self) -> Self {
            a / b
        }
        pub fn rem(a: Self, b: Self) -> Self {
            a % b
        }
        pub fn add(a: Self, b: Self) -> Self {
            a + b
        }
        pub fn sub(a: Self, b: Self) -> Self {
            a - b
        }
        pub fn shl(a: Self, b: u32) -> Self {
            a << b
        }
        pub fn shr(a: Self, b: u32) -> Self {
            a >> b
        }
        pub fn and(a: Self, b: Self) -> Self {
            a & b
        }
        pub fn or(a: Self, b: Self) -> Self {
            a | b
        }
        pub fn xor(a: Self, b: Self) -> Self {
            a ^ b
        }
    }

    impl u32 {
        pub const MIN: Self = 0;
        pub const MAX: Self = !0;
        pub const BIT_WIDTH: u32 = 4;
        pub fn default() -> Self {
            0
        }
        pub fn mul(a: Self, b: Self) -> Self {
            a * b
        }
        pub fn div(a: Self, b: Self) -> Self {
            a / b
        }
        pub fn rem(a: Self, b: Self) -> Self {
            a % b
        }
        pub fn add(a: Self, b: Self) -> Self {
            a + b
        }
        pub fn sub(a: Self, b: Self) -> Self {
            a - b
        }
        pub fn shl(a: Self, b: u32) -> Self {
            a << b
        }
        pub fn shr(a: Self, b: u32) -> Self {
            a >> b
        }
        pub fn and(a: Self, b: Self) -> Self {
            a & b
        }
        pub fn or(a: Self, b: Self) -> Self {
            a | b
        }
        pub fn xor(a: Self, b: Self) -> Self {
            a ^ b
        }
    }

    impl u64 {
        pub const MIN: Self = 0;
        pub const MAX: Self = !0;
        pub const BIT_WIDTH: u32 = 8;
        pub fn default() -> Self {
            0
        }
        pub fn mul(a: Self, b: Self) -> Self {
            a * b
        }
        pub fn div(a: Self, b: Self) -> Self {
            a / b
        }
        pub fn rem(a: Self, b: Self) -> Self {
            a % b
        }
        pub fn add(a: Self, b: Self) -> Self {
            a + b
        }
        pub fn sub(a: Self, b: Self) -> Self {
            a - b
        }
        pub fn shl(a: Self, b: u32) -> Self {
            a << b
        }
        pub fn shr(a: Self, b: u32) -> Self {
            a >> b
        }
        pub fn and(a: Self, b: Self) -> Self {
            a & b
        }
        pub fn or(a: Self, b: Self) -> Self {
            a | b
        }
        pub fn xor(a: Self, b: Self) -> Self {
            a ^ b
        }
    }

    impl u128 {
        pub const MIN: Self = 0;
        pub const MAX: Self = !0;
        pub const BIT_WIDTH: u32 = 8;
        pub fn default() -> Self {
            0
        }
        pub fn mul(a: Self, b: Self) -> Self {
            a * b
        }
        pub fn div(a: Self, b: Self) -> Self {
            a / b
        }
        pub fn rem(a: Self, b: Self) -> Self {
            a % b
        }
        pub fn add(a: Self, b: Self) -> Self {
            a + b
        }
        pub fn sub(a: Self, b: Self) -> Self {
            a - b
        }
        pub fn shl(a: Self, b: u32) -> Self {
            a << b
        }
        pub fn shr(a: Self, b: u32) -> Self {
            a >> b
        }
        pub fn and(a: Self, b: Self) -> Self {
            a & b
        }
        pub fn or(a: Self, b: Self) -> Self {
            a | b
        }
        pub fn xor(a: Self, b: Self) -> Self {
            a ^ b
        }
    }

    impl usize {
        pub const MIN: Self = (); // __march_ptr_width_unsigned_min(); // TODO: intrinsics
        pub const MAX: Self = (); // __march_ptr_width_unsigned_max(); // TODO: intrinsics
        pub const BIT_WIDTH: u32 = (); // __march_ptr_width_bits(); // TODO: intrinsics
        pub fn default() -> Self {
            0
        }
        pub fn mul(a: Self, b: Self) -> Self {
            a * b
        }
        pub fn div(a: Self, b: Self) -> Self {
            a / b
        }
        pub fn rem(a: Self, b: Self) -> Self {
            a % b
        }
        pub fn add(a: Self, b: Self) -> Self {
            a + b
        }
        pub fn sub(a: Self, b: Self) -> Self {
            a - b
        }
        pub fn shl(a: Self, b: u32) -> Self {
            a << b
        }
        pub fn shr(a: Self, b: u32) -> Self {
            a >> b
        }
        pub fn and(a: Self, b: Self) -> Self {
            a & b
        }
        pub fn or(a: Self, b: Self) -> Self {
            a | b
        }
        pub fn xor(a: Self, b: Self) -> Self {
            a ^ b
        }
    }

    impl i8 {
        pub const MIN: Self = -128;
        pub const MAX: Self = 127;
        pub const BIT_WIDTH: u32 = 1;
        pub fn default() -> Self {
            0
        }
        pub fn mul(a: Self, b: Self) -> Self {
            a * b
        }
        pub fn div(a: Self, b: Self) -> Self {
            a / b
        }
        pub fn rem(a: Self, b: Self) -> Self {
            a % b
        }
        pub fn add(a: Self, b: Self) -> Self {
            a + b
        }
        pub fn sub(a: Self, b: Self) -> Self {
            a - b
        }
        pub fn shl(a: Self, b: u32) -> Self {
            a << b
        }
        pub fn shr(a: Self, b: u32) -> Self {
            a >> b
        }
        pub fn and(a: Self, b: Self) -> Self {
            a & b
        }
        pub fn or(a: Self, b: Self) -> Self {
            a | b
        }
        pub fn xor(a: Self, b: Self) -> Self {
            a ^ b
        }
    }

    impl i16 {
        pub const MIN: Self = -32768;
        pub const MAX: Self = 32767;
        pub const BIT_WIDTH: u32 = 2;
        pub fn default() -> Self {
            0
        }
        pub fn mul(a: Self, b: Self) -> Self {
            a * b
        }
        pub fn div(a: Self, b: Self) -> Self {
            a / b
        }
        pub fn rem(a: Self, b: Self) -> Self {
            a % b
        }
        pub fn add(a: Self, b: Self) -> Self {
            a + b
        }
        pub fn sub(a: Self, b: Self) -> Self {
            a - b
        }
        pub fn shl(a: Self, b: u32) -> Self {
            a << b
        }
        pub fn shr(a: Self, b: u32) -> Self {
            a >> b
        }
        pub fn and(a: Self, b: Self) -> Self {
            a & b
        }
        pub fn or(a: Self, b: Self) -> Self {
            a | b
        }
        pub fn xor(a: Self, b: Self) -> Self {
            a ^ b
        }
    }

    impl i32 {
        pub const MIN: Self = -2147483648;
        pub const MAX: Self = 2147483647;
        pub const BIT_WIDTH: u32 = 4;
        pub fn default() -> Self {
            0
        }
        pub fn mul(a: Self, b: Self) -> Self {
            a * b
        }
        pub fn div(a: Self, b: Self) -> Self {
            a / b
        }
        pub fn rem(a: Self, b: Self) -> Self {
            a % b
        }
        pub fn add(a: Self, b: Self) -> Self {
            a + b
        }
        pub fn sub(a: Self, b: Self) -> Self {
            a - b
        }
        pub fn shl(a: Self, b: u32) -> Self {
            a << b
        }
        pub fn shr(a: Self, b: u32) -> Self {
            a >> b
        }
        pub fn and(a: Self, b: Self) -> Self {
            a & b
        }
        pub fn or(a: Self, b: Self) -> Self {
            a | b
        }
        pub fn xor(a: Self, b: Self) -> Self {
            a ^ b
        }
    }

    impl i64 {
        pub const MIN: Self = -9223372036854775808;
        pub const MAX: Self = 9223372036854775807;
        pub const BIT_WIDTH: u32 = 8;
        pub fn default() -> Self {
            0
        }
        pub fn mul(a: Self, b: Self) -> Self {
            a * b
        }
        pub fn div(a: Self, b: Self) -> Self {
            a / b
        }
        pub fn rem(a: Self, b: Self) -> Self {
            a % b
        }
        pub fn add(a: Self, b: Self) -> Self {
            a + b
        }
        pub fn sub(a: Self, b: Self) -> Self {
            a - b
        }
        pub fn shl(a: Self, b: u32) -> Self {
            a << b
        }
        pub fn shr(a: Self, b: u32) -> Self {
            a >> b
        }
        pub fn and(a: Self, b: Self) -> Self {
            a & b
        }
        pub fn or(a: Self, b: Self) -> Self {
            a | b
        }
        pub fn xor(a: Self, b: Self) -> Self {
            a ^ b
        }
    }

    impl i128 {
        pub const MIN: Self = !(1 << 128);
        pub const MAX: Self = 1 << 128;
        pub const BIT_WIDTH: u32 = 8;
        pub fn default() -> Self {
            0
        }
        pub fn mul(a: Self, b: Self) -> Self {
            a * b
        }
        pub fn div(a: Self, b: Self) -> Self {
            a / b
        }
        pub fn rem(a: Self, b: Self) -> Self {
            a % b
        }
        pub fn add(a: Self, b: Self) -> Self {
            a + b
        }
        pub fn sub(a: Self, b: Self) -> Self {
            a - b
        }
        pub fn shl(a: Self, b: u32) -> Self {
            a << b
        }
        pub fn shr(a: Self, b: u32) -> Self {
            a >> b
        }
        pub fn and(a: Self, b: Self) -> Self {
            a & b
        }
        pub fn or(a: Self, b: Self) -> Self {
            a | b
        }
        pub fn xor(a: Self, b: Self) -> Self {
            a ^ b
        }
    }

    impl isize {
        pub const MIN: Self = (); // __march_ptr_width_signed_min(); // TODO: intrinsics
        pub const MAX: Self = (); // __march_ptr_width_signed_max(); // TODO: intrinsics
        pub const BIT_WIDTH: u32 = (); // __march_ptr_width_bits(); // TODO: intrinsics
        pub fn default() -> Self {
            0
        }
        pub fn mul(a: Self, b: Self) -> Self {
            a * b
        }
        pub fn div(a: Self, b: Self) -> Self {
            a / b
        }
        pub fn rem(a: Self, b: Self) -> Self {
            a % b
        }
        pub fn add(a: Self, b: Self) -> Self {
            a + b
        }
        pub fn sub(a: Self, b: Self) -> Self {
            a - b
        }
        pub fn shl(a: Self, b: u32) -> Self {
            a << b
        }
        pub fn shr(a: Self, b: u32) -> Self {
            a >> b
        }
        pub fn and(a: Self, b: Self) -> Self {
            a & b
        }
        pub fn or(a: Self, b: Self) -> Self {
            a | b
        }
        pub fn xor(a: Self, b: Self) -> Self {
            a ^ b
        }
    }
}
