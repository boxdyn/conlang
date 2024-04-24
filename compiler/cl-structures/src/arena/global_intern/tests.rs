//! Tests for the global intern pool
use super::*;

#[test]
fn globalsym_from_returns_unique_value_for_unique_keys() {
    let foo_bar = GlobalSym::from("foo_bar");
    let foo_baz = GlobalSym::from("foo_baz");
    assert_ne!(foo_bar, foo_baz);
    assert_eq!(foo_bar, GlobalSym::from("foo_bar"));
    assert_eq!(foo_baz, GlobalSym::from("foo_baz"));
}
#[test]
fn try_from_str_returns_none_before_init() {
    if let Some(value) = GlobalSym::try_from_str("") {
        panic!("{value}")
    }
}
#[test]
fn try_from_str_returns_some_when_key_exists() {
    let _ = GlobalSym::from("foo_bar");
    assert!(dbg!(GlobalSym::try_from_str("foo_bar")).is_some());
}

#[test]
fn try_from_str_returns_the_same_thing_as_globalsym_from() {
    let foo_bar = GlobalSym::from("foo_bar");
    assert_eq!(Some(foo_bar), GlobalSym::try_from_str("foo_bar"));
}

#[test]
fn map_works() {
    let foo_bar = GlobalSym::from("foo_bar");
    assert!(foo_bar.map(|sym| "foo_bar" == sym).unwrap());
}
