#!/usr/bin/env -S conlang test_my_int()

/// New-type wrapper around an integer
struct MyInt(i32);

impl MyInt {
    // unary operators
    /// -self
    fn neg(&MyInt(self)) -> Self = MyInt(-self);
    /// !self
    fn not(&MyInt(self)) -> Self = MyInt(!self);

    // binary operators
    /// self * other
    fn mul(&MyInt(self), MyInt(other)) -> Self = MyInt(self * other);
    /// self / other
    fn div(&MyInt(self), MyInt(other)) -> Self = MyInt(self / other);
    /// self % other
    fn rem(&MyInt(self), MyInt(other)) -> Self = MyInt(self % other);
    /// self + other
    fn add(&MyInt(self), MyInt(other)) -> Self = MyInt(self + other);
    /// self - other
    fn sub(&MyInt(self), MyInt(other)) -> Self = MyInt(self - other);
    /// self << other
    fn shl(&MyInt(self), MyInt(other)) -> Self = MyInt(self << other);
    /// self >> other
    fn shr(&MyInt(self), MyInt(other)) -> Self = MyInt(self >> other);
    /// self & other
    fn and(&MyInt(self), MyInt(other)) -> Self = MyInt(self & other);
    /// self | other
    fn or(&MyInt(self), MyInt(other)) -> Self = MyInt(self | other);
    /// self ^ other
    fn xor(&MyInt(self), MyInt(other)) -> Self = MyInt(self ^ other);
}

fn test_my_int() {
    let a, b = MyInt(10), MyInt(15);
    let MyInt(150)    = a * b  else panic(a * b, " != ", MyInt(150));
    let MyInt(0)      = a / b  else panic(a / b, " != ", MyInt(0));
    let MyInt(10)     = a % b  else panic(a % b, " != ", MyInt(10));
    let MyInt(25)     = a + b  else panic(a + b, " != ", MyInt(25));
    let MyInt(-5)     = a - b  else panic(a - b, " != ", MyInt(-5));
    let MyInt(327680) = a << b else panic(a << b, " != ", MyInt(327680));
    let MyInt(0)      = a >> b else panic(a >> b, " != ", MyInt(0));
    let MyInt(10)     = a & b  else panic(a & b, " != ", MyInt(10));
    let MyInt(15)     = a | b  else panic(a | b, " != ", MyInt(15));
    let MyInt(5)      = a ^ b  else panic(a ^ b, " != ", MyInt(5));
    let MyInt(-10)    = -a     else panic(-a, " != ", MyInt(-10));
    let MyInt(-11)    = !a     else panic(!a, " != ", MyInt(-11));
    println("All tests passed!")
}
