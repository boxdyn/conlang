//! Demonstrates modules

fn main() {
    use foo::bar::baz; // -> use super::baz::qux as baz: 10 -> fn baz::qux: 12
    baz()
}

mod foo {
    mod bar {
        use super::baz::qux as baz; // -> fn qux: 14
    }
    mod baz {
        fn qux() {
            print("foo, bar, baz, qux")
        }
    }
}
