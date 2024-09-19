#![allow(unused_imports)]
use crate::{convalue::ConValue, env::Environment, Interpret};
use cl_ast::*;
use cl_lexer::Lexer;
use cl_parser::Parser;
pub use macros::*;

mod macros {
    //! Useful macros for parsing Conlang
    //!
    //! # Parsing
    //! Macro [`parse`] stringifies, lexes, and parses everything you give to it
    //! ```rust
    //! # use conlang::interpreter::tests::*;
    //! parse!{
    //!     fn main () {
    //!         "Hello, world!"
    //!     }
    //! }
    //! ```
    //!
    //! # Evaluating
    //! Macro [`eval`] parses code in the given [`Environment`].
    //! - [`assert_eval`] does the above, but expects [`Ok`]
    //! - [`assert_noeval`] does the above, but expects [`Err`]
    //! ```rust
    //! # use conlang::interpreter::tests::*;
    //! let mut env = Default::default();
    //! eval!{env,
    //!     let x = 2;
    //! }.expect("variable binding should succeed.");
    //! ```
    //!
    //! # Extracting Results
    //! Macros [`env_eq`] and [`env_ne`] take an "Environment Member Expression" and a value which
    //! implements [`Into<ConValue>`], and asserts that they are either equal or not equal,
    //! respectively.
    //!
    //! Macro [`conv_cmp`] takes two things that can be converted to [`ConValue`], and calls the
    //! provided comparison function on them, returning a [`bool`].
    //! ```rust
    //! # use conlang::interpreter::tests::*;
    //! let mut env = Default::default();
    //! assert_eval!{env,
    //!     let x = 10;
    //! }
    //! env_eq!(env.x, 10); // like assert_eq! for Environments
    //! ```
    #![allow(unused_macros)]
    use crate::IResult;
    use cl_parser::parser::Parse;

    use super::*;

    pub fn test_inside_block(block: &Block, env: &mut Environment) -> IResult<()> {
        let Block { stmts } = block;
        for stmt in stmts {
            stmt.interpret(env)?;
        }
        Ok(())
    }

    /// Stringifies, lexes, and parses everything you give to it
    ///
    /// Returns a `Result<`[`File`]`, ParseError>`
    pub macro file($($t:tt)*) {
        File::parse(&mut Parser::new(Lexer::new(stringify!( $($t)* ))))
    }

    /// Stringifies, lexes, and parses everything you give to it
    ///
    /// Returns a `Result<`[`Block`]`, ParseError>`
    pub macro block($($t:tt)*) {
        Block::parse(&mut Parser::new(Lexer::new(stringify!({ $($t)* }))))
    }

    /// Evaluates a block of code in the given environment
    ///
    /// ```rust,ignore
    /// eval!(env,
    ///     // Conlang code goes here
    ///     fn main () {
    ///         "Hello, world!"
    ///     }
    /// )
    /// ```
    pub macro eval($env: path, $($t:tt)*) {{
        test_inside_block(&block!($($t)*)
            .expect("code passed to eval! should parse correctly"),
            &mut $env)
    }}

    /// Evaluates a block of code in the given environment, expecting the interpreter to succeed
    ///
    /// ```rust,ignore
    /// assert_eval!(env,
    ///     // Conlang code goes here
    ///     fn main () {
    ///         "Hello, world!"
    ///     }
    /// )
    /// ```
    pub macro assert_eval($($t:tt)*) {
        eval!($($t)*)
            .expect(stringify!($($t)* should execute correctly))
    }

    /// Evaluates a block of code in the given environment, expecting the interpreter to fail
    ///
    /// ```rust,ignore
    /// assert_noeval!(env,
    ///     // Conlang code goes here
    ///     fn main () {
    ///         1 == "Hello world!" // type incompatibility
    ///     }
    /// )
    /// ```
    pub macro assert_noeval($($t:tt)*) {
        eval!($($t)*)
            .expect_err(stringify!($($t)* should not execute correctly))
    }

    pub macro conv_cmp($func: ident, $a: expr, $b: expr) {
        $a.$func(&($b).into())
            .expect(stringify!($a should be comparable to $b))
            .truthy()
            .expect(stringify!(result of comparison should be ConValue::Bool))
    }

    pub macro env_ne($env:ident.$var:ident, $expr:expr) {{
        let evaluated = $env.get(stringify!($var).into())
            .expect(stringify!($var should be defined and initialized));
        if !conv_cmp!(neq, evaluated, $expr) {
            panic!("assertion {} ({evaluated}) != {} failed.", stringify!($var), stringify!($expr))
        }
    }}

    pub macro env_eq($env:ident.$var:ident, $expr:expr) {{
        let evaluated = $env.get(stringify!($var).into())
            .expect(stringify!($var should be defined and initialized));
        if !conv_cmp!(eq, evaluated, $expr) {
            panic!("assertion {} ({evaluated}) == {} failed.", stringify!($var), stringify!($expr))
        }
    }}
    pub macro tuple($($expr:expr),*) {
        ConValue::from(vec![$(ConValue::from($expr)),*])
    }
}

mod let_declarations {
    use super::*;

    #[test]
    fn let_binding_uninit() {
        let mut env = Environment::new();
        assert_eval!(
            env,
            let x;
        );
    }
    #[test]
    fn let_binding_with_init() {
        let mut env = Environment::new();
        assert_eval!( env,
            let x = 10;
        );

        env_eq!(env.x, 10)
    }
    #[test]
    fn let_binding_from_another_variable() {
        let mut env = Environment::new();
        assert_eval!(env,
            let x = 10;
            let y = x;
        );

        env_eq!(env.x, 10);
        env_eq!(env.y, 10);
    }
}

mod fn_declarations {
    use super::*;
    #[test]
    fn empty_fn() {
        let mut env = Environment::new();
        assert_eval!(env, fn empty_fn() {});
        // TODO: true equality for functions
        assert_eq!(
            "fn empty_fn () {}",
            format!(
                "{}",
                env.get("empty_fn".into())
                    .expect(stringify!(empty_fn should be defined and initialized))
            )
        )
    }
    #[test]
    fn identity_fn() {
        let mut env = Environment::new();
        assert_eval!(
            env,
            fn identity(input: i32) -> i32 {
                input
            }
            let output = identity(12);
        );
        env_eq!(env.output, 12);
    }
}

mod operators {
    use cl_ast::Tuple;

    use super::*;
    #[test]
    fn unary() {
        let mut env = Default::default();
        assert_eval!(env,
            let neg_one = -1;
            let pos_one = -neg_one;
            let not_true = !true;
            let not_false = !false;
        );
        env_eq!(env.neg_one, -1);
        env_eq!(env.pos_one, 1);
        env_eq!(env.not_true, false);
        env_eq!(env.not_false, true);
    }
    #[test]
    fn mul_div_rem() {
        let mut env = Default::default();
        assert_eval!(env,
            let one = 129 % 32;
            let twelve = 144 / 12;
            let one_forty_four = 12 * 12;
        );
        env_eq!(env.one, 1);
        env_eq!(env.twelve, 12);
        env_eq!(env.one_forty_four, 144);
    }
    #[test]
    fn add_sub() {
        let mut env = Default::default();
        assert_eval!(env,
            let is_42 = 17 + 25;
            let also_42 = 165 - 123;
        );
        env_eq!(env.is_42, 42);
        env_eq!(env.also_42, 42);
    }
    #[test]
    fn shift() {
        let mut env = Default::default();
        assert_eval!(env,
            let eight = 1<<3;
            let one = 8>>3;
        );
        env_eq!(env.eight, 8);
        env_eq!(env.one, 1);
    }
    #[test]
    fn bitwise() {
        let mut env = Default::default();
        assert_eval!(env,
            let and_b1010 = 0b1111 & 0b1010;
            let or__b1111 = 0b1111 | 0b1010;
            let xor_b0101 = 0b1111 ^ 0b1010;
        );
        env_eq!(env.and_b1010, 0b1010);
        env_eq!(env.or__b1111, 0b1111);
        env_eq!(env.xor_b0101, 0b0101);
    }
    #[test]
    fn logical() {
        let mut env = Default::default();
        assert_eval!(env,
            let t_and_t = true && true;
            let t_and_f = true && false;
            let f_and_t = false && true;
            let f_and_f = false && false;

            let t_or_t = true || true;
            let t_or_f = true || false;
            let f_or_t = false || true;
            let f_or_f = false || false;

            let t_xor_t = true ^^ true;
            let t_xor_f = true ^^ false;
            let f_xor_t = false ^^ true;
            let f_xor_f = false ^^ false;
        );
        env_eq!(env.t_and_t, true);
        env_eq!(env.t_and_f, false);
        env_eq!(env.f_and_t, false);
        env_eq!(env.f_and_f, false);

        env_eq!(env.t_or_t, true);
        env_eq!(env.t_or_f, true);
        env_eq!(env.f_or_t, true);
        env_eq!(env.f_or_f, false);

        env_eq!(env.t_xor_t, false);
        env_eq!(env.t_xor_f, true);
        env_eq!(env.f_xor_t, true);
        env_eq!(env.f_xor_f, false);
    }
    #[test]
    fn logical_short_circuits() {
        let mut env = Default::default();
        assert_eval!(env,
            let mut and_short_circuits = true;
            false && { and_short_circuits = false; false };

            let mut or_short_circuits = true;
            true || { or_short_circuits = false; false };
        );
        env_eq!(env.and_short_circuits, true);
        env_eq!(env.or_short_circuits, true);
    }
    #[test]
    fn range() {
        let mut env = Default::default();
        assert_eval!(env,
            let inclusive = 0..=10;
            let exclusive = 0..10;
        );
        // TODO: extract the ranges and actually check them
    }

    #[test]
    fn compare() {
        let mut env = Default::default();
        assert_eval!(env,
            // Less than
            let is_10_lt_20 = 10 <  20;
            let is_10_le_20 = 10 <= 20;
            let is_10_eq_20 = 10 == 20;
            let is_10_ne_20 = 10 != 20;
            let is_10_ge_20 = 10 >= 20;
            let is_10_gt_20 = 10 >  20;
            // Equal to
            let is_10_lt_10 = 10 <  10;
            let is_10_le_10 = 10 <= 10;
            let is_10_eq_10 = 10 == 10;
            let is_10_ne_10 = 10 != 10;
            let is_10_ge_10 = 10 >= 10;
            let is_10_gt_10 = 10 >  10;
            // Greater than
            let is_20_lt_10 = 20 <  10;
            let is_20_le_10 = 20 <= 10;
            let is_20_eq_10 = 20 == 10;
            let is_20_ne_10 = 20 != 10;
            let is_20_ge_10 = 20 >= 10;
            let is_20_gt_10 = 20 >  10;
            dump();
        );

        // Less than
        env_eq!(env.is_10_lt_20, true); //  <
        env_eq!(env.is_10_le_20, true); //  <=
        env_eq!(env.is_10_eq_20, false); // ==
        env_eq!(env.is_10_ne_20, true); //  !=
        env_eq!(env.is_10_ge_20, false); // >=
        env_eq!(env.is_10_gt_20, false); // >
                                         // Equal to
        env_eq!(env.is_10_lt_10, false);
        env_eq!(env.is_10_le_10, true);
        env_eq!(env.is_10_eq_10, true);
        env_eq!(env.is_10_ne_10, false);
        env_eq!(env.is_10_ge_10, true);
        env_eq!(env.is_10_gt_10, false);
        // Greater than
        env_eq!(env.is_20_lt_10, false);
        env_eq!(env.is_20_le_10, false);
        env_eq!(env.is_20_eq_10, false);
        env_eq!(env.is_20_ne_10, true);
        env_eq!(env.is_20_ge_10, true);
        env_eq!(env.is_20_gt_10, true);
    }
    #[test]
    fn assign() {
        let mut env = Default::default();
        assert_eval!(env,
            let base = 10;
            let mut assign = base;
            let mut add = base;
            let mut sub = base;
            let mut mul = base;
            let mut div = base;
            let mut rem = base;
            let mut and = base;
            let mut or_ = base;
            let mut xor = base;
            let mut shl = base;
            let mut shr = base;

            let modifier = 3;
            assign = modifier;
            add += modifier;
            sub -= modifier;
            mul *= modifier;
            div /= modifier;
            rem %= modifier;
            and &= modifier;
            or_ |= modifier;
            xor ^= modifier;
            shl <<= modifier;
            shr >>= modifier;

        );
        let (base, modifier) = (10, 3);
        env_eq!(env.assign, modifier);
        env_eq!(env.add, base + modifier);
        env_eq!(env.sub, base - modifier);
        env_eq!(env.mul, base * modifier);
        env_eq!(env.div, base / modifier);
        env_eq!(env.rem, base % modifier);
        env_eq!(env.and, base & modifier);
        env_eq!(env.or_, base | modifier);
        env_eq!(env.xor, base ^ modifier);
        env_eq!(env.shl, base << modifier);
        env_eq!(env.shr, base >> modifier);
    }
    #[test]
    fn assignment_is_left_assoc_and_returns_empty() {
        let mut env = Default::default();
        assert_eval!(env,
            let x; // uninitialized (no type)
            let y = 0xdeadbeef;
            let z = 10;

            x = y = z;
        );
        env_eq!(env.x, ());
        env_eq!(env.y, 10);
        env_eq!(env.z, 10);
    }
    #[test]
    #[should_panic]
    fn assignment_accounts_for_type() {
        let mut env = Default::default();
        assert_eval!(env,
            let x = "a string";
            let y = 0xdeadbeef;
            y = x; // should crash: type error
        );
    }
    #[test]
    fn precedence() {
        let mut env = Default::default();
        assert_eval!(env,
            // mul/div/rem > add/sub
            let a = 2 * 3 + 4 * 5 / 6; // = 9
            // add/sub > shift
            let b = 1 << 3 + 1 << 2; // 1 << 6 = 64
            // shift > bitwise
            let c = 4 | 4 << 4; // 4 | 64 = 68
            // all together now!
            let d = 1 << 2 + 3 * 4; // 2 << 14 = 16384
            let e = 4 * 3 + 2 << 1; // 14 << 1 = 28
        );
        env_eq!(env.a, 9);
        env_eq!(env.b, 64);
        env_eq!(env.c, 68);
        env_eq!(env.d, 16384);
        env_eq!(env.e, 28);
    }
}

#[allow(dead_code)]
fn test_template() {
    let mut env = Default::default();
    assert_eval!(env,);
    //env_eq!(, );
}
