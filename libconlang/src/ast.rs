//! # The Abstract Syntax Tree
//! Contains definitions of AST Nodes, to be derived by a [parser](super::parser).
//!
//! Also contains a [Visitor](visitor::Visitor) trait for visiting nodes
//!
//! ## Syntax
//! ```ignore
//! Start      := expression::Expr
//! Identifier := IDENTIFIER
//! Literal    := STRING | CHAR | FLOAT | INT | TRUE | FALSE
//! ```
//! See [literal] and [expression] for more details.

pub mod preamble {
    //! Common imports for working with the [ast](super)
    pub use super::{
        expression::{
            self, control,
            math::{self, operator},
        },
        literal,
        visitor::{Visitor, Walk},
        Identifier, Start,
    };
}

mod visitor {
    use super::{
        expression::{control::*, math::*, Block, *},
        literal::*,
        *,
    };
    /// [Walk] is the lexical inverse of [Visitor]
    pub trait Walk<T: Visitor<R> + ?Sized, R> {
        ///
        fn walk(&self, visitor: &mut T) -> R;
    }
    pub mod walker {
        use super::*;
        macro_rules! impl_walk {
            ($($T:ty => $f:ident),*$(,)?) => {
                $(impl<T: Visitor<R> + ?Sized, R> Walk<T, R> for $T {
                    fn walk(&self, visitor: &mut T) -> R {
                        visitor.$f(self)
                    }
                })*
            };
        }
        impl_walk! {
            // ast
            Start => visit,
            // grouped expr
            Block => visit_block,
            Group => visit_group,
            // Identifier
            Identifier => visit_identifier,
            // ast::literal
            str   => visit_string_literal,
            char  => visit_char_literal,
            bool  => visit_bool_literal,
            u128  => visit_int_literal,
            Float => visit_float_literal,
            // ast::math
            Ignore  => visit_ignore,
            Assign  => visit_assign,
            Compare => visit_compare,
            Logic   => visit_logic,
            Bitwise => visit_bitwise,
            Shift   => visit_shift,
            Term    => visit_term,
            Factor  => visit_factor,
            Unary   => visit_unary,
            // ast::math::operator
            operator::Ignore  => visit_ignore_op,
            operator::Compare => visit_compare_op,
            operator::Assign  => visit_assign_op,
            operator::Logic   => visit_logic_op,
            operator::Bitwise => visit_bitwise_op,
            operator::Shift   => visit_shift_op,
            operator::Term    => visit_term_op,
            operator::Factor  => visit_factor_op,
            operator::Unary   => visit_unary_op,
            // ast::control::Branch
            While => visit_while,
            If    => visit_if,
            For   => visit_for,
            Else  => visit_else,
            // ast::control::Flow
            Continue => visit_continue,
            Return   => visit_return,
            Break    => visit_break,
        }
        impl<T: Visitor<R> + ?Sized, R> Walk<T, R> for () {
            fn walk(&self, visitor: &mut T) -> R {
                visitor.visit_empty()
            }
        }
        impl<T: Visitor<R> + ?Sized, R> Walk<T, R> for Expr {
            fn walk(&self, visitor: &mut T) -> R {
                visitor.visit_ignore(&self.ignore)
            }
        }
        impl<T: Visitor<R> + ?Sized, R> Walk<T, R> for Primary {
            fn walk(&self, visitor: &mut T) -> R {
                match self {
                    Primary::Identifier(i) => visitor.visit_identifier(i),
                    Primary::Literal(l) => visitor.visit_literal(l),
                    Primary::Block(b) => visitor.visit_block(b),
                    Primary::Group(g) => visitor.visit_group(g),
                    Primary::Branch(b) => visitor.visit_branch_expr(b),
                }
            }
        }
        impl<T: Visitor<R> + ?Sized, R> Walk<T, R> for Literal {
            fn walk(&self, visitor: &mut T) -> R {
                match self {
                    Literal::String(s) => visitor.visit_string_literal(s),
                    Literal::Char(c) => visitor.visit_char_literal(c),
                    Literal::Bool(b) => visitor.visit_bool_literal(b),
                    Literal::Float(f) => visitor.visit_float_literal(f),
                    Literal::Int(i) => visitor.visit_int_literal(i),
                }
            }
        }
        impl<T: Visitor<R> + ?Sized, R> Walk<T, R> for Flow {
            fn walk(&self, visitor: &mut T) -> R {
                match self {
                    Flow::While(w) => visitor.visit_while(w),
                    Flow::If(i) => visitor.visit_if(i),
                    Flow::For(f) => visitor.visit_for(f),
                    Flow::Continue(c) => visitor.visit_continue(c),
                    Flow::Return(r) => visitor.visit_return(r),
                    Flow::Break(b) => visitor.visit_break(b),
                }
            }
        }
    }

    pub trait Visitor<R> {
        /// Visit the start of an AST
        fn visit(&mut self, start: &Start) -> R {
            self.visit_expr(&start.0)
        }

        /// Visit an [Expression](Expr)
        fn visit_expr(&mut self, expr: &Expr) -> R {
            expr.walk(self)
        }
        // Block expression
        /// Visit a [Block] expression
        fn visit_block(&mut self, expr: &Block) -> R {
            match &expr.expr {
                Some(expr) => self.visit_expr(expr),
                None => self.visit_empty(),
            }
        }
        /// Visit a [Group] expression
        fn visit_group(&mut self, group: &Group) -> R {
            match group {
                Group::Expr(expr) => self.visit_expr(expr),
                Group::Empty => self.visit_empty(),
            }
        }

        // Math expression
        fn visit_binary<F, Op>(&mut self, expr: &Binary<F, (Op, F)>) -> R
        where F: Walk<Self, R>, Op: Walk<Self, R>;
        /// Visit an [Ignore] expression
        fn visit_ignore(&mut self, expr: &Ignore) -> R {
            self.visit_binary(expr)
        }
        /// Visit an [Assign] expression
        fn visit_assign(&mut self, expr: &Assign) -> R {
            self.visit_binary(expr)
        }
        /// Visit a [Compare] expression
        fn visit_compare(&mut self, expr: &Compare) -> R {
            self.visit_binary(expr)
        }
        /// Visit a [Logic] expression
        fn visit_logic(&mut self, expr: &Logic) -> R {
            self.visit_binary(expr)
        }
        /// Visit a [Bitwise] expression
        fn visit_bitwise(&mut self, expr: &Bitwise) -> R {
            self.visit_binary(expr)
        }
        /// Visit a [Shift] expression
        fn visit_shift(&mut self, expr: &Shift) -> R {
            self.visit_binary(expr)
        }
        /// Visit a [Term] expression
        fn visit_term(&mut self, expr: &Term) -> R {
            self.visit_binary(expr)
        }
        /// Visit a [Factor] expression
        fn visit_factor(&mut self, expr: &Factor) -> R {
            self.visit_binary(expr)
        }
        /// Visit a [Unary] expression
        fn visit_unary(&mut self, expr: &Unary) -> R;
        /// Visit a [Primary] expression
        ///
        /// [Primary] := [Identifier] | [Literal] | [Block] | [Flow]
        fn visit_primary(&mut self, expr: &Primary) -> R {
            expr.walk(self)
        }
        // Math operators
        /// Visit an [Ignore] [operator](operator::Ignore)
        fn visit_ignore_op(&mut self, op: &operator::Ignore) -> R;
        /// Visit a [Compare] [operator](operator::Compare)
        fn visit_compare_op(&mut self, op: &operator::Compare) -> R;
        /// Visit an [Assign] [operator](operator::Assign)
        fn visit_assign_op(&mut self, op: &operator::Assign) -> R;
        /// Visit a [Logic] [operator](operator::Logic)
        fn visit_logic_op(&mut self, op: &operator::Logic) -> R;
        /// Visit a [Bitwise] [operator](operator::Bitwise)
        fn visit_bitwise_op(&mut self, op: &operator::Bitwise) -> R;
        /// Visit a [Shift] [operator](operator::Shift)
        fn visit_shift_op(&mut self, op: &operator::Shift) -> R;
        /// Visit a [Term] [operator](operator::Term)
        fn visit_term_op(&mut self, op: &operator::Term) -> R;
        /// Visit a [Factor] [operator](operator::Factor)
        fn visit_factor_op(&mut self, op: &operator::Factor) -> R;
        /// Visit a [Unary] [operator](operator::Unary)
        fn visit_unary_op(&mut self, op: &operator::Unary) -> R;

        /// Visit a [Flow] expression.
        ///
        /// [Flow] := [While] | [If] | [For]
        fn visit_branch_expr(&mut self, expr: &Flow) -> R {
            expr.walk(self)
        }
        /// Visit an [If] expression
        fn visit_if(&mut self, expr: &If) -> R;
        /// Visit a [While] loop expression
        fn visit_while(&mut self, expr: &While) -> R;
        /// Visit a [For] loop expression
        fn visit_for(&mut self, expr: &For) -> R;
        /// Visit an [Else] expression
        fn visit_else(&mut self, expr: &Else) -> R;
        /// Visit a [Continue] expression
        fn visit_continue(&mut self, expr: &Continue) -> R;
        /// Visit a [Break] expression
        fn visit_break(&mut self, expr: &Break) -> R;
        /// Visit a [Return] expression
        fn visit_return(&mut self, expr: &Return) -> R;

        // primary symbols
        /// Visit an [Identifier]
        fn visit_identifier(&mut self, ident: &Identifier) -> R;
        /// Visit a [Literal]
        ///
        /// [Literal] := [String] | [char] | [bool] | [Float] | [u128]
        fn visit_literal(&mut self, literal: &Literal) -> R {
            literal.walk(self)
        }
        /// Visit a [string](str) literal
        fn visit_string_literal(&mut self, string: &str) -> R;
        /// Visit a [character](char) literal
        fn visit_char_literal(&mut self, char: &char) -> R;
        /// Visit a [boolean](bool) literal
        fn visit_bool_literal(&mut self, bool: &bool) -> R;
        /// Visit a [floating point](Float) literal
        fn visit_float_literal(&mut self, float: &Float) -> R;
        /// Visit an [integer](u128) literal
        fn visit_int_literal(&mut self, int: &u128) -> R;
        /// Visit an Empty literal
        fn visit_empty(&mut self) -> R;
    }
}
/// Marks the root of a tree
/// # Syntax
/// [`Start`] := [`expression::Expr`]
#[derive(Clone, Debug)]
pub struct Start(pub expression::Expr);

/// An Identifier stores the name of an item
/// # Syntax
/// [`Identifier`] := [`IDENTIFIER`](crate::token::Type::Identifier)
#[derive(Clone, Debug, Hash)]
pub struct Identifier(pub String);

pub mod todo {
    //! temporary storage for pending expression work.  \
    //! when an item is in progress, remove it from todo.
    //!
    //! # General TODOs:
    //! - [ ] Implement support for storing items in the AST
    //! - [ ] Implement paths
    //! - [ ] Implement functions
    //! - [ ] Implement structs
    //! - [ ] Implement enums
    //! - [ ] Implement implementation
    //! - [ ] Store token spans in AST
    pub mod path {
        //! Path support
        //! - [ ] Add namespace syntax (i.e. `::crate::foo::bar` | `foo::bar::Baz` | `foo::bar::*`)
        //!
        //! Path resolution will be vital to the implementation of structs, enums, impl blocks,
        //! traits, modules, etc.
    }
    pub mod function {
        //! Function support
        //! - [ ] Add function declaration expression (returns a function)
        //! - [ ] Add function call expression
    }

    pub mod structure {
        //! Struct support
        //! - [ ] Add struct declaration expression (returns a struct declaration)
        //! - [ ] Add struct value expression (returns a struct value)
        //! - [ ] Add struct update syntax (yippee!!)
    }

    pub mod enumeration {
        //! Enum support
        //! - [ ] Add enum declaration expression (returns an enum declaration)
        //! - [ ] Add enum value expression (returns an enum value)
    }

    pub mod implementation {
        //! Impl block support
        //! - [ ] Add impl block expression? Statement?
        //! - [ ] Add member function call expression
    }
}

pub mod literal {
    //! # Literal Expressions
    //! Evaluate to the literal they contain
    //! # Syntax
    //! ```ignore
    //! Literal := String | Char | Float | Int
    //! String  := STRING
    //! Float   := FLOAT
    //! Char    := CHARACTER
    //! Bool    := TRUE | FALSE
    //! Int     := INTEGER
    //! ```

    /// Represents a literal value
    /// # Syntax
    /// [`Literal`] := [`String`] | [`char`] | [`bool`] | [`Float`] | [`u128`]
    #[derive(Clone, Debug)]
    pub enum Literal {
        /// Represents a literal string value
        /// # Syntax
        /// [`Literal::String`] := [`STRING`](crate::token::Type::String)
        String(String),
        /// Represents a literal [char] value
        /// # Syntax
        /// [`Literal::Char`] := [`CHARACTER`](crate::token::Type::Character)
        Char(char),
        /// Represents a literal [bool] value
        /// # Syntax
        /// [`Literal::Bool`] :=
        ///     [`TRUE`](crate::token::Keyword::True)
        ///   | [`FALSE`](crate::token::Keyword::False)
        Bool(bool),
        /// Represents a literal float value
        /// # Syntax
        /// [`Float`] := [`FLOAT`](crate::token::Type::Float)
        Float(Float),
        /// Represents a literal integer value
        /// # Syntax
        /// [`u128`] := [`INTEGER`](crate::token::Type::Integer)
        Int(u128),
    }

    /// Represents a literal float value
    /// # Syntax
    /// [`Float`] := [`FLOAT`](crate::token::Type::Float)
    #[derive(Clone, Debug)]
    pub struct Float {
        pub sign: bool,
        pub exponent: i32,
        pub mantissa: u64,
    }
}

pub mod expression {
    //! # Expressions
    //!
    //! The [expression] is the backbone of Conlang: everything is an expression.
    //!
    //! ## Grammar
    //! Higher number = higher precedence.
    //!
    //! |  # |              Node | Function                                      
    //! |----|------------------:|:----------------------------------------------
    //! |  0 |          [`Expr`] | Contains an expression
    //! |  1 |  [`math::Ignore`] | Ignores the preceding sub-expression's result
    //! |  2 |  [`math::Assign`] | Assignment
    //! |  3 | [`math::Compare`] | Value Comparison
    //! |  4 |   [`math::Logic`] | Boolean And, Or, Xor
    //! |  5 | [`math::Bitwise`] | Bitwise And, Or, Xor
    //! |  6 |   [`math::Shift`] | Shift Left/Right
    //! |  7 |    [`math::Term`] | Add, Subtract
    //! |  8 |  [`math::Factor`] | Multiply, Divide, Remainder
    //! |  9 |   [`math::Unary`] | Unary Dereference, Reference, Negate, Not
    //! | 10 | [`control::Flow`] | Branch expressions (`if`, `while`, `for`, `return`, `break`, `continue`), `else`
    //! | 10 |         [`Group`] | Group expressions `(` [Expr]? `)` /* Can evaluate to Empty! */
    //! | 10 |         [`Block`] | Block expressions `{` [Expr] `}`
    //! | 10 |       [`Primary`] | Contains an [Identifier], [Literal](literal::Literal), [Block], or [Flow](control::Flow)
    //!
    //! ## Syntax
    //! ```ignore
    //! Expr  := control::Flow | math::Ignore
    //! Block := '{' Expr '}'
    //! Group := '(' Expr? ')'
    //! Primary := Identifier | Literal | Block | control::Branch
    //! ```
    //! See [control] and [math] for their respective production rules.
    use super::*;

    /// Contains an expression
    ///
    /// # Syntax
    /// [`Expr`] := [`math::Ignore`]
    #[derive(Clone, Debug)]
    pub struct Expr {
        pub ignore: math::Ignore,
    }

    /// A [Primary] Expression is the expression with the highest precedence (i.e. the deepest
    /// derivation)
    /// # Syntax
    /// [`Primary`] :=
    ///     [`IDENTIFIER`](Identifier)
    ///   | [`Literal`](literal::Literal)
    ///   | [`Block`]
    ///   | [`Branch`](control::Flow)
    #[derive(Clone, Debug)]
    pub enum Primary {
        Identifier(Identifier),
        Literal(literal::Literal),
        Block(Block),
        Group(Group),
        Branch(control::Flow),
    }

    /// Contains a Block Expression
    /// # Syntax
    /// [`Block`] := `'{'` [`Expr`] `'}'`
    #[derive(Clone, Debug)]
    pub struct Block {
        pub expr: Option<Box<Expr>>,
    }

    /// Contains a Parenthesized Expression
    /// # Syntax
    /// [`Group`] := `'('` [`Expr`]? `')'`
    #[derive(Clone, Debug)]
    pub enum Group {
        Expr(Box<Expr>),
        Empty,
    }

    pub mod math {
        //! # Arithmetic and Logical Expressions
        //!
        //! ## Precedence Order
        //! Operator associativity is always left-to-right among members of the same group
        //!
        //! | # |      Name | Operators                             | Associativity
        //! |---|----------:|:--------------------------------------|---------------
        //  |   | TODO: Try | `?`                                   |
        //! | 1 |   [Unary] | `*` `&` `-` `!`                       | Right
        //! | 2 |  [Factor] | `*` `/` `%`                           | Left to Right
        //! | 3 |    [Term] | `+` `-`                               | Left to Right
        //! | 4 |   [Shift] | `<<` `>>`                             | Left to Right
        //! | 5 | [Bitwise] | `&` <code>&#124;</code>               | Left to Right
        //! | 6 |   [Logic] | `&&` <code>&#124;&#124;</code> `^^`   | Left to Right
        //! | 7 | [Compare] | `<` `<=` `==` `!=` `>=` `>`           | Left to Right
        #![doc = concat!( //|                                       |
        r"  | 8 |  [Assign] |", r"`*=`, `/=`, `%=`, `+=`, `-=`, ",//|
        /*  |   |           |*/ r"`&=`, <code>&#124;=</code>, ",  //|
        /*  |   |           |*/ r"`^=`, `<<=`, `>>=`",            r"| Right to Left")]
        //! | 9 |  [Ignore] | `;` |
        //!
        //! <!-- Note: &#124; == | /-->
        //!
        //! ## Syntax
        //! ```ignore
        //! Ignore  := Assign  (CompareOp Assign )*
        //! Assign  := Compare (IgnoreOp  Compare)*
        //! Compare := Logic   (AssignOp  Logic  )*
        //! Logic   := Bitwise (LogicOp   Bitwise)*
        //! Bitwise := Shift   (BitOp     Shift  )*
        //! Shift   := Term    (ShiftOp   Term   )*
        //! Term    := Factor  (TermOp    Factor )*
        //! Factor  := Unary   (FactorOp  Unary  )*
        //! Unary   := (UnaryOp)* Primary
        //! ```
        use super::*;

        /// The template for [Binary] operations.
        /// # Syntax
        /// [`Binary`] := `First` (`Other`)*
        #[derive(Clone, Debug)]
        pub struct Binary<First, Other> {
            pub first: Box<First>,
            pub other: Vec<Other>,
        }
        impl<First, Other> Binary<First, Other> {
            pub fn new(first: First, other: Vec<Other>) -> Self {
                Self { first: Box::new(first), other }
            }
            pub fn first(&self) -> &First {
                &self.first
            }
            pub fn other(&self) -> &[Other] {
                &self.other
            }
        }

        /// Ignores the result of the leading sub-expression.
        /// Great if you only want the side-effects.
        /// # Syntax
        /// [`Ignore`] := [`Assign`] ([`operator::Ignore`] [`Assign`])*
        pub type Ignore = Binary<Assign, (operator::Ignore, Assign)>;

        /// Assigns the result of the trailing sub-expression to the leading sub-expression.
        /// Resolves to the Empty type.
        /// # Syntax
        /// [`Assign`] := [`Compare`] ([`operator::Assign`] [`Compare`])?
        pub type Assign = Binary<Compare, (operator::Assign, Compare)>;

        /// Compares the values of the trailing and leading sub-expressions,
        /// and resolves to a boolean.
        /// # Syntax
        /// [`Compare`] := [`Logic`] ([`operator::Compare`] [`Logic`])*
        pub type Compare = Binary<Logic, (operator::Compare, Logic)>;

        /// Performs a boolean logic operation on the leading and trailing sub-expressions.
        /// # Syntax
        /// [`Logic`] := [`Bitwise`] ([`operator::Logic`] [`Bitwise`])*
        pub type Logic = Binary<Bitwise, (operator::Logic, Bitwise)>;

        /// Performs a bitwise opration on the leading and trailing sub-expressions.
        /// # Syntax
        /// [`Bitwise`] := [`Shift`] ([`operator::Bitwise`] [`Shift`])*
        pub type Bitwise = Binary<Shift, (operator::Bitwise, Shift)>;

        /// Shifts the leading sub-expression by the trailing sub-expression
        /// # Syntax
        /// [`Shift`] := [`Term`] ([`operator::Shift`] [`Term`])*
        pub type Shift = Binary<Term, (operator::Shift, Term)>;

        /// Adds or subtracts the trailing sub-expression from the leading sub-expression
        /// # Syntax
        /// [`Term`] := [`Factor`] ([`operator::Term`] [`Factor`])*
        pub type Term = Binary<Factor, (operator::Term, Factor)>;

        /// Multiplies, Divides, or finds the remainder of the trailing sub-expression
        /// from the leading sub-expression
        /// # Syntax
        /// [`Factor`] := [`Unary`] ([`operator::Factor`] [`Unary`])*
        pub type Factor = Binary<Unary, (operator::Factor, Unary)>;

        /// Performs a unary operation on the trailing sub-expression.
        /// # Syntax
        /// [`Unary`] := ([`operator::Unary`])* [`Primary`]
        #[derive(Clone, Debug)]
        pub struct Unary(pub Vec<operator::Unary>, pub Primary);

        pub mod operator {
            //! | # | [Operators](Operator)                 | Associativity
            //! |---|---------------------------------------|--------------
            //! | 0 | ([Unary]) `*`, `&`, `-`, `!`          | Left to Right
            //! | 1 | `*`, `/`, `%`                         | Left to Right
            //! | 2 | `+`, `-`                              | Left to Right
            //! | 3 | `<<`, `>>`                            | Left to Right
            //! | 4 | `&`, <code>&#124;</code>, `^`         | Left to Right
            //! | 5 | `&&`, <code>&#124;&#124;</code>, `^^` | Left to Right
            //! | 6 | `>`. `>=`. `==`. `!=`. `<=`. `<`      | Left to Right
            #![doc = concat!(
              r"| 7 |", r"`*=`, `/=`, `%=`, `+=`, `-=`, ",//|
            /*  |   |*/ r"`&=`, <code>&#124;=</code>, ",  //|
            /*  |   |*/ r"`^=`, `<<=`, `>>=`, `=`",       r"| Left to Right")]
            //! | 8 | `;`                                   |
            use crate::token::Type;
            /// Defines an operator enum and a conversion
            macro operator ($(
                $(#[$doc:meta])* $T:ident { $( $v:ident := $tty:pat ),*$(,)? }
            )*) {
                $(#[doc = concat!("[`",stringify!($T),"`](super::",stringify!($T),") operators")]
                $(#[$doc])* #[derive(Clone, Copy, Debug, PartialEq, Eq)]
                pub enum $T { $($v,)* }
                impl From<Type> for Option<$T> {
                    fn from(value: Type) -> Option<$T> {
                        match value { $($tty => Some(<$T>::$v),)* _ => None }
                    }
                })*
            }
            operator! {
                /// (`*`, `&`, `-`, `!`, `@`, `#`, `~`)
                Unary {
                    RefRef := Type::AmpAmp,
                    Deref  := Type::Star,
                    Ref    := Type::Amp,
                    Neg    := Type::Minus,
                    Not    := Type::Bang,
                    At     := Type::At,
                    Hash   := Type::Hash,
                    Tilde  := Type::Tilde,
                }
                /// (`*`, `/`, `%`)
                Factor {
                    Mul := Type::Star,
                    Div := Type::Slash,
                    Rem := Type::Rem,
                }
                /// (`+`, `-`)
                Term {
                    Add := Type::Plus,
                    Sub := Type::Minus,
                }
                /// (`<<`, `>>`)
                Shift {
                    Lsh := Type::LtLt,
                    Rsh := Type::GtGt,
                }
                /// (`&`, `|`, `^`)
                Bitwise {
                    BitAnd := Type::Amp,
                    BitOr := Type::Bar,
                    BitXor := Type::Xor,
                }
                /// (`&&`, `||`, `^^`)
                Logic {
                    LogAnd := Type::AmpAmp,
                    LogOr := Type::BarBar,
                    LogXor := Type::XorXor,
                }
                /// (`<`, `<=`, `==`, `!=`, `>=`, `>`)
                Compare {
                    Less := Type::Lt,
                    LessEq := Type::LtEq,
                    Equal := Type::EqEq,
                    NotEq := Type::BangEq,
                    GreaterEq := Type::GtEq,
                    Greater := Type::Gt,
                }
                /// (`=`, `+=`, `-=`, `*=`, `/=`,
                ///  `&=`, `|=`, `^=`, `<<=`, `>>=`)
                Assign {
                    Assign := Type::Eq,
                    AddAssign := Type::PlusEq,
                    SubAssign := Type::MinusEq,
                    MulAssign := Type::StarEq,
                    DivAssign := Type::SlashEq,
                    BitAndAssign := Type::AmpEq,
                    BitOrAssign := Type::BarEq,
                    BitXorAssign := Type::XorEq,
                    ShlAssign := Type::LtLtEq,
                    ShrAssign := Type::GtGtEq,
                }
                /// (`;`)
                Ignore {
                    Ignore := Type::Semi,
                }
            }
        }
    }

    pub mod control {
        //! # Control Flow Expressions
        //! ## Conditional Branch Expressions
        //! [`if` expressions][1] split a program's control flow based on a boolean
        //! condition.  \
        //! It is equivalent to a [`while` expression][2] that runs at most once.
        //!
        //! [`while` expressions][2] repeat a block of code (the loop body) until either
        //!   - a boolean condition fails
        //!   - a value is returned from the loop with a [`break` expression][5]
        //!
        //! [`for` expressions][3] repeat a block of code (the loop body) until either
        //!   - an iterable expression fails to return a value
        //!   - a value is returned from the loop with a [`break` expression][5]
        //!
        //! [`else` expressions][4] are evaluated when the body of a
        //! conditional branch expression does not return a value:
        //!   - If the body was never run (`if false`, `while false`)
        //!   - If the loop exited without encountering a [`break` expression][5]
        //! ## Unconditional Branch Expressions
        //! [`break` expressions][5] return a value from within a loop
        //!
        //! [`return` expressions][6] return a value from within a function
        //!
        //! [`continue` expressions][7] skip to the next iteration of a loop
        //! # Syntax
        //! ```rust,ignore
        //! Branch := While | If | For
        //! If     := "if" Expr Block Else?
        //! While  := "while" Expr Block Else?
        //! For    := "for" Identifier "in" Expr Block Else?
        //! Else   := "else" Block
        //!
        //! Break := "break" Expr
        //! ```
        //!
        //! [1]: If
        //! [2]: While
        //! [3]: For
        //! [4]: Else
        //! [5]: Break
        //! [6]: Return
        //! [7]: Continue
        use super::*;

        /// Contains a [Control Flow Expression](control).
        ///
        /// See the module-level documentation for more information.
        ///
        /// [While], [If], [For], [Continue], [Return], or [Break]
        #[derive(Clone, Debug)]
        pub enum Flow {
            /// Represents a [`while` expression](While)
            While(While),
            /// Represents a [`if` expression](If)
            If(If),
            /// Represents a [`for` expression](For)
            For(For),
            /// Represents a [`continue` expression](Continue)
            Continue(Continue),
            /// Represents a [`return` expression](Return)
            Return(Return),
            /// Represents a [`break` expression](Break)
            Break(Break),
        }

        /// Represents a [`while` loop](control).
        ///
        /// A [`while` expression](While) contains a [loop condition expression](Expr),
        /// a [block expression, (the loop body,)](Block) and
        /// an optional¹ [else expression](Else).
        ///
        /// ¹ A value can be returned from within the body using a
        /// [`break` expression](Break)  \
        /// If a `break` expression is used in this way, the `else` block is mandatory.
        ///
        /// # Examples
        /// ```rust,ignore
        /// let var = while boolean_variable {
        ///     break true
        /// } else {
        ///     false
        /// }
        /// ```
        /// # Syntax
        /// [`While`] := `"while"` [`Expr`] [`Block`] [`Else`]`?`
        #[derive(Clone, Debug)]
        pub struct While {
            pub cond: Box<Expr>,
            pub body: Block,
            pub else_: Option<Else>,
        }

        /// Represents an [`if`-`else` control flow structure](control).
        ///
        /// An [`if` expression](If) contains a [condition expression](Expr),
        /// a [block expression](Block) to be executed,
        /// and an optional¹ [`else` block](Else).
        ///
        /// ¹ If the body evaluates to anything other than the Empty type,
        /// the `else` block is mandatory.
        /// # Syntax
        /// [`If`] := `"if"` [`Expr`] [`Block`] [`Else`]`?`
        #[derive(Clone, Debug)]
        pub struct If {
            pub cond: Box<Expr>,
            pub body: Block,
            pub else_: Option<Else>,
        }

        /// Represents a [`for` loop](control).
        ///
        /// A [`for` expression](For) contains a [loop variable](Identifier),
        /// an [iterable expression, (TBD,)](Expr),
        /// a [block expression(the loop body)](Block),
        /// and an optional¹ [`else` block](Else)
        ///
        ///
        /// ¹ A value can be returned from within the body using a
        /// [`break` expression](Break)  \
        /// If a `break` expression is used in this way, the `else` block is mandatory.
        /// # Syntax
        /// [`For`] := `"for"` [`Identifier`] `"in"` [`Expr`]² [`Block`] [`Else`]`?`
        ///
        /// ² [`Expr`] returns something Iterable
        #[derive(Clone, Debug)]
        pub struct For {
            pub var: Identifier,
            pub iter: Box<Expr>,
            pub body: Block,
            pub else_: Option<Else>,
        }

        /// Represents an [`else` block](control).
        ///
        /// An [`else` block](Else) contains instructions to be executed if
        /// the corresponding body refused to produce a value. In the case of
        /// [`if` expressions](If), this happens if the condition fails.
        /// In the case of loop ([`while`](While), [`for`](For))expressions,
        /// this executes when the loop does *not* [`break`](Break).
        ///
        /// If one of the aforementioned control flow expressions evaluates
        /// to something other than the Empty type, this block is mandatory.
        ///
        /// # Syntax
        /// [`Else`] := `"else"` [`Block`]
        #[derive(Clone, Debug)]
        pub struct Else {
            pub block: Block,
        }

        /// Represents a [`continue` expression][control]
        ///
        /// # Syntax
        /// [`Continue`] := `"continue"`
        #[derive(Clone, Debug)]
        pub struct Continue;

        /// Represents a [`break` expression][control].
        ///
        /// # Syntax
        /// [`Break`] := `"break"` [`Expr`]
        #[derive(Clone, Debug)]
        pub struct Break {
            pub expr: Box<Expr>,
        }
        /// Represents a [`return` expression][control].
        ///
        /// # Syntax
        /// [`Return`] := `"return"` [`Expr`]
        #[derive(Clone, Debug)]
        pub struct Return {
            pub expr: Box<Expr>,
        }
    }
}
