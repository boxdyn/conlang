//! Conlang is an expression-based programming language with similarities to Rust
#![warn(clippy::all)]
#![feature(decl_macro)]
pub mod token {
    //! Stores a component of a file as a type and span
    use std::ops::Range;

    mod token_type;
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Type {
        // Invalid syntax
        Invalid,
        // Any kind of comment
        Comment,
        // Any identifier
        Identifier,
        Keyword(Keyword),
        // Literals
        Integer,
        Float,
        String,
        Character,
        // Delimiters
        LCurly,
        RCurly,
        LBrack,
        RBrack,
        LParen,
        RParen,
        // Compound punctuation
        Lsh,
        Rsh,
        AmpAmp,
        BarBar,
        NotNot,
        CatEar,
        EqEq,
        GtEq,
        LtEq,
        NotEq,
        StarEq,
        DivEq,
        RemEq,
        AddEq,
        SubEq,
        AndEq,
        OrEq,
        XorEq,
        LshEq,
        RshEq,
        Arrow,
        FatArrow,
        // Simple punctuation
        Semi,
        Dot,
        Star,
        Div,
        Plus,
        Minus,
        Rem,
        Bang,
        Eq,
        Lt,
        Gt,
        Amp,
        Bar,
        Xor,
        Hash,
        At,
        Colon,
        Backslash,
        Question,
        Comma,
        Tilde,
        Grave,
    }

    /// Represents a reserved word.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Keyword {
        Break,
        Continue,
        Else,
        False,
        For,
        Fn,
        If,
        In,
        Let,
        Return,
        True,
        While,
    }
    impl std::str::FromStr for Keyword {
        type Err = ();
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Ok(match s {
                "break" => Self::Break,
                "continue" => Self::Continue,
                "else" => Self::Else,
                "false" => Self::False,
                "for" => Self::For,
                "fn" => Self::Fn,
                "if" => Self::If,
                "in" => Self::In,
                "let" => Self::Let,
                "return" => Self::Return,
                "true" => Self::True,
                "while" => Self::While,
                _ => Err(())?,
            })
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Token {
        ty: Type,
        pub head: usize,
        pub tail: usize,
        line: usize,
        col: usize,
    }
    impl Token {
        pub fn new(ty: Type, head: usize, tail: usize, line: usize, col: usize) -> Self {
            Self { ty, head, tail, line, col }
        }
        pub fn cast(self, ty: Type) -> Self {
            Self { ty, ..self }
        }
        // Hack to work around
        pub fn rebound(self, head: usize, tail: usize) -> Self {
            Self { head, tail, ..self }
        }
        pub fn line(&self) -> usize {
            self.line
        }
        pub fn col(&self) -> usize {
            self.col
        }
        pub fn is_empty(&self) -> bool {
            self.tail == self.head
        }
        /// Gets the length of the token, in bytes
        pub fn len(&self) -> usize {
            self.tail - self.head
        }
        /// Gets the [Type] of the token
        pub fn ty(&self) -> Type {
            self.ty
        }
        /// Gets the exclusive range of the token
        pub fn range(&self) -> Range<usize> {
            self.head..self.tail
        }
    }
}

pub mod ast {
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
        ///
        /// # Examples
        /// ```rust,ignore
        /// ```
        pub trait Walk<T: Visitor<R> + ?Sized, R> {
            ///
            fn walk(&self, visitor: &mut T) -> R;
        }
        pub mod walker {
            use super::*;
            macro_rules! impl_walk {
                ($($T:ty => $f:ident),*$(,)?) => {
                    $(impl<T: Visitor<R>, R> Walk<T, R> for $T {
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
                &str  => visit_string_literal,
                char  => visit_char_literal,
                bool  => visit_bool_literal,
                u128   => visit_int_literal,
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
                Return   =>visit_return,
                Break    => visit_break,
            }
            impl<T: Visitor<R> + ?Sized, R> Walk<T, R> for Expr {
                fn walk(&self, visitor: &mut T) -> R {
                    match self {
                        Expr::Flow(f) => visitor.visit_control_flow(f),
                        Expr::Ignore(i) => visitor.visit_ignore(i),
                    }
                }
            }
            impl<T: Visitor<R> + ?Sized, R> Walk<T, R> for Final {
                fn walk(&self, visitor: &mut T) -> R {
                    match self {
                        Final::Identifier(i) => visitor.visit_identifier(i),
                        Final::Literal(l) => visitor.visit_literal(l),
                        Final::Block(b) => visitor.visit_block(b),
                        Final::Group(g) => visitor.visit_group(g),
                        Final::Branch(b) => visitor.visit_branch_expr(b),
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
            impl<T: Visitor<R> + ?Sized, R> Walk<T, R> for Branch {
                fn walk(&self, visitor: &mut T) -> R {
                    match self {
                        Branch::While(w) => visitor.visit_while(w),
                        Branch::If(i) => visitor.visit_if(i),
                        Branch::For(f) => visitor.visit_for(f),
                    }
                }
            }
            impl<T: Visitor<R> + ?Sized, R> Walk<T, R> for Flow {
                fn walk(&self, visitor: &mut T) -> R {
                    match self {
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
                self.visit_expr(&expr.expr)
            }
            /// Visit a [Group] expression
            fn visit_group(&mut self, expr: &Group) -> R {
                self.visit_expr(&expr.expr)
            }

            // Math expression
            /// Visit an [Ignore] expression
            fn visit_ignore(&mut self, expr: &Ignore) -> R;
            /// Visit an [Assign] expression
            fn visit_assign(&mut self, expr: &Assign) -> R;
            /// Visit a [Compare] expression
            fn visit_compare(&mut self, expr: &Compare) -> R;
            /// Visit a [Logic] expression
            fn visit_logic(&mut self, expr: &Logic) -> R;
            /// Visit a [Bitwise] expression
            fn visit_bitwise(&mut self, expr: &Bitwise) -> R;
            /// Visit a [Shift] expression
            fn visit_shift(&mut self, expr: &Shift) -> R;
            /// Visit a [Term] expression
            fn visit_term(&mut self, expr: &Term) -> R;
            /// Visit a [Factor] expression
            fn visit_factor(&mut self, expr: &Factor) -> R;
            /// Visit a [Unary] expression
            fn visit_unary(&mut self, expr: &Unary) -> R;
            /// Visit a [Final] expression
            ///
            /// [Final] := [Identifier] | [Literal] | [Block] | [Branch]
            fn visit_final(&mut self, expr: &Final) -> R {
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

            /// Visit a [Branch] expression.
            ///
            /// [Branch] := [While] | [If] | [For]
            fn visit_branch_expr(&mut self, expr: &Branch) -> R {
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
            /// Visit a [Control Flow](control::Flow) expression
            ///
            /// [`Flow`] := [`Continue`] | [`Return`] | [`Break`]
            fn visit_control_flow(&mut self, expr: &control::Flow) -> R {
                expr.walk(self)
            }
            /// Visit a [Continue] expression
            fn visit_continue(&mut self, expr: &Continue) -> R;
            /// Visit a [Break] expression
            fn visit_break(&mut self, expr: &Break) -> R;
            /// Visit a [Return] expression
            fn visit_return(&mut self, expr: &Return) -> R;

            // final symbols
            /// Visit an [Identifier]
            fn visit_identifier(&mut self, ident: &Identifier) -> R;
            /// Visit a [Literal]
            ///
            /// [Literal] := [String] | [char] | [bool] | [Float] | [Int]
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
            /// Visit an [integer](Int) literal
            fn visit_int_literal(&mut self, int: &u128) -> R;
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
            //! - [ ] Add namespace syntax (i.e. `::crate::foo::bar` | `foo::bar::Baz` |
            //!   `foo::bar::*`)
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
        /// [`Literal`] := [`String`] | [`char`] | [`bool`] | [`Float`] | [`Int`]
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
            /// [`Int`] := [`INTEGER`](crate::token::Type::Integer)
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
        //! |  0 |           [`Expr`]| Contains an expression
        //! |  1 |  [`control::Flow`]| Unconditional branches (`return`, `break`, `continue`)
        //! |  2 |   [`math::Ignore`]| Ignores the preceding sub-expression's result
        //! |  3 |   [`math::Assign`]| Assignment
        //! |  4 |  [`math::Compare`]| Value Comparison
        //! |  5 |    [`math::Logic`]| Boolean And, Or, Xor
        //! |  6 |  [`math::Bitwise`]| Bitwise And, Or, Xor
        //! |  7 |    [`math::Shift`]| Shift Left/Right
        //! |  8 |     [`math::Term`]| Add, Subtract
        //! |  9 |   [`math::Factor`]| Multiply, Divide, Remainder
        //! | 10 |    [`math::Unary`]| Unary Dereference, Reference, Negate, Not
        //! | 11 |[`control::Branch`]| Conditional branches (`if`, `while`, `for`), `else`
        //! | 12 |          [`Group`]| Group expressions `(` [Expr] `)`
        //! | 12 |          [`Block`]| Block expressions `{` [Expr] `}`
        //! | 12 |          [`Final`]| Contains an [Identifier], [Literal](literal::Literal), [Block], or [Branch](control::Branch)
        //!
        //! ## Syntax
        //! ```ignore
        //! Expr  := control::Flow | math::Ignore
        //! Block := '{' Expr '}'
        //! Group := '(' Expr ')'
        //! Final := Identifier | Literal | Block | control::Branch
        //! ```
        //! See [control] and [math] for their respective production rules.

        #[allow(unused_imports)]
        use super::*;

        /// Contains an expression
        ///
        /// # Syntax
        /// [`Expr`] := [`control::Flow`] | [`math::Ignore`]
        #[derive(Clone, Debug)]
        pub enum Expr {
            Flow(control::Flow),
            Ignore(math::Ignore),
        }

        /// A [Final] Expression is the expression with the highest precedence (i.e. the deepest
        /// derivation)
        /// # Syntax
        /// [`Final`] :=
        ///     [`IDENTIFIER`](Identifier)
        ///   | [`Literal`](literal::Literal)
        ///   | [`Block`]
        ///   | [`Branch`](control::Branch)
        #[derive(Clone, Debug)]
        pub enum Final {
            Identifier(Identifier),
            Literal(literal::Literal),
            Block(Block),
            Group(Group),
            Branch(control::Branch),
        }

        /// Contains a Block Expression
        /// # Syntax
        /// [`Block`] := `'{'` [`Expr`] `'}'`
        #[derive(Clone, Debug)]
        pub struct Block {
            pub expr: Box<Expr>,
        }

        /// Contains a Parenthesized Expression
        /// # Syntax
        /// [`Group`] := `'('` [`Expr`] `')'`
        #[derive(Clone, Debug)]
        pub struct Group {
            pub expr: Box<Expr>,
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
            //! Unary   := (UnaryOp)* Final
            //! ```
            use super::*;

            /// Ignores the result of the left sub-expression.
            /// Great if you only want the side-effects.
            /// # Syntax
            /// [`Ignore`] := [`Assign`] ([`operator::Ignore`] [`Assign`])*
            #[derive(Clone, Debug)]
            pub struct Ignore(pub Assign, pub Vec<(operator::Ignore, Assign)>);

            /// Assigns the result of the right sub-expression to the left sub-expression.
            /// Resolves to the Empty type.
            /// # Syntax
            /// [`Assign`] := [`Compare`] ([`operator::Assign`] [`Compare`])?
            #[derive(Clone, Debug)]
            pub struct Assign(pub Compare, pub Vec<(operator::Assign, Compare)>);

            /// Compares the values of the right and left sub-expressions,
            /// and resolves to a boolean.
            /// # Syntax
            /// [`Compare`] := [`Logic`] ([`operator::Compare`] [`Logic`])*
            #[derive(Clone, Debug)]
            pub struct Compare(pub Logic, pub Vec<(operator::Compare, Logic)>);

            /// Performs a boolean logic operation on the left and right sub-expressions.
            /// # Syntax
            /// [`Logic`] := [`Bitwise`] ([`operator::Logic`] [`Bitwise`])*
            #[derive(Clone, Debug)]
            pub struct Logic(pub Bitwise, pub Vec<(operator::Logic, Bitwise)>);

            /// Performs a bitwise opration on the left and right sub-expressions.
            /// # Syntax
            /// [`Bitwise`] := [`Shift`] ([`operator::Bitwise`] [`Shift`])*
            #[derive(Clone, Debug)]
            pub struct Bitwise(pub Shift, pub Vec<(operator::Bitwise, Shift)>);

            /// Shifts the left sub-expression by the right sub-expression
            /// # Syntax
            /// [`Shift`] := [`Term`] ([`operator::Shift`] [`Term`])*
            #[derive(Clone, Debug)]
            pub struct Shift(pub Term, pub Vec<(operator::Shift, Term)>);

            /// Adds or subtracts the right sub-expression from the left sub-expression
            /// # Syntax
            /// [`Term`] := [`Factor`] ([`operator::Term`] [`Factor`])*
            #[derive(Clone, Debug)]
            pub struct Term(pub Factor, pub Vec<(operator::Term, Factor)>);

            /// Multiplies, Divides, or finds the remainder of the right sub-expression
            /// from the left sub-expression
            /// # Syntax
            /// [`Factor`] := [`Unary`] ([`operator::Factor`] [`Unary`])*
            #[derive(Clone, Debug)]
            pub struct Factor(pub Unary, pub Vec<(operator::Factor, Unary)>);

            /// Performs a unary operation on the right sub-expression.
            /// # Syntax
            /// [`Unary`] := ([`operator::Unary`])* [`Final`]
            #[derive(Clone, Debug)]
            pub struct Unary(pub Vec<operator::Unary>, pub Final);

            pub mod operator {
                //! | # | Operators                             | Associativity
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
                macro operator ($($(#[$doc:meta])* $T:ident {
                    $( $v:ident := $tty:pat ),*$(,)?
                })*) {$(
                    #[doc = concat!("[`",stringify!($T),"`](super::",stringify!($T),") operators")]
                    $(#[$doc])* #[derive(Clone, Copy, Debug, PartialEq, Eq)]
                    pub enum $T { $($v,)* }
                    impl From<Type> for Option<$T> {
                        fn from(value: Type) -> Option<$T> {
                            match value { $($tty => Some(<$T>::$v),)* _ => None }
                        }
                    }
                )*}

                operator! {
                    /// (`*`, `&`, `-`, `!`)
                    Unary {
                        Deref := Type::Star,
                        Ref   := Type::Amp,
                        Neg   := Type::Minus,
                        Not   := Type::Bang,
                        At    := Type::At,
                        Hash  := Type::Hash,
                        Tilde := Type::Tilde,
                    }
                    /// (`*`, `/`, `%`)
                    Factor {
                        Mul := Type::Star,
                        Div := Type::Div,
                        Rem := Type::Rem,
                    }
                    /// (`+`, `-`)
                    Term {
                        Add := Type::Plus,
                        Sub := Type::Minus,
                    }
                    /// (`<<`, `>>`)
                    Shift {
                        Lsh := Type::Lsh,
                        Rsh := Type::Rsh,
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
                        LogXor := Type::CatEar,
                    }
                    /// (`<`, `<=`, `==`, `!=`, `>=`, `>`)
                    Compare {
                        Less := Type::Lt,
                        LessEq := Type::LtEq,
                        Equal := Type::EqEq,
                        NotEq := Type::NotEq,
                        GreaterEq := Type::GtEq,
                        Greater := Type::Gt,
                    }
                    /// (`=`, `+=`, `-=`, `*=`, `/=`,
                    ///  `&=`, `|=`, `^=`, `<<=`, `>>=`)
                    Assign {
                        Assign := Type::Eq,
                        AddAssign := Type::AddEq,
                        SubAssign := Type::SubEq,
                        MulAssign := Type::StarEq,
                        DivAssign := Type::DivEq,
                        BitAndAssign := Type::AndEq,
                        BitOrAssign := Type::OrEq,
                        BitXorAssign := Type::XorEq,
                        ShlAssign := Type::LshEq,
                        ShrAssign := Type::RshEq,
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
            //! [7]: Flow::Continue
            use super::*;

            /// Contains a [ConditionalBranch Expression](control).
            ///
            /// [While], [If], [For]
            #[derive(Clone, Debug)]
            pub enum Branch {
                While(While),
                If(If),
                For(For),
            }

            /// Contains an [Unconditional Branch Expression](control).
            ///
            /// [Continue](Flow::Continue), [Return], [Break]
            #[derive(Clone, Debug)]
            pub enum Flow {
                /// Represents a [`continue` expression](Flow::Continue)
                ///
                /// # Syntax
                /// [`Flow::Continue`] := `"continue"`
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
}

pub mod lexer {
    //! Converts a text file into tokens
    use crate::token::{Token, Type};
    use lerox::Combinator;

    #[allow(dead_code)]
    pub struct Lexer<'t> {
        text: &'t str,
        cursor: usize,
        line: usize,
        col: usize,
    }
    /// Implements the non-terminals of a language
    impl<'t> Lexer<'t> {
        pub fn new(text: &'t str) -> Self {
            Self { text, cursor: 0, line: 1, col: 1 }
        }
        /// Counts some length
        #[inline]
        fn count_len(&mut self, len: usize) -> &mut Self {
            self.cursor += len;
            self.col += len;
            self
        }
        /// Counts a line
        #[inline]
        fn count_line(&mut self, lines: usize) -> &mut Self {
            self.line += lines;
            self.col = 1;
            self
        }
        /// Skips whitespace in the text
        fn skip_whitespace(&mut self) {
            self.count_len(
                Rule::new(self.text())
                    .and_any(Rule::whitespace_not_newline)
                    .end()
                    .unwrap_or_default(),
            );
            if Rule::new(self.text()).char('\n').end().is_some() {
                // recurse until all newlines are skipped
                self.count_len(1).count_line(1).skip_whitespace();
            }
        }
        /// Advances the cursor and produces a token from a provided [Rule] function
        fn map_rule<F>(&mut self, rule: F, ty: Type) -> Option<Token>
        where F: Fn(Rule) -> Rule {
            self.skip_whitespace();
            let (line, col, start) = (self.line, self.col, self.cursor);
            self.count_len(Rule::new(self.text()).and(rule).end()?);
            Some(Token::new(ty, start, self.cursor, line, col))
        }
        /// Gets a slice of text beginning at the cursor
        fn text(&self) -> &str {
            &self.text[self.cursor..]
        }
        // classifies a single arbitrary token
        /// Returns the result of the rule with the highest precedence, if any matches
        pub fn any(&mut self) -> Option<Token> {
            None.or_else(|| self.comment())
                .or_else(|| self.identifier())
                .or_else(|| self.literal())
                .or_else(|| self.delimiter())
                .or_else(|| self.punctuation())
                .or_else(|| self.invalid())
        }
        }
        /// Attempts to produce a [Type::String], [Type::Float], or [Type::Integer]
        pub fn literal(&mut self) -> Option<Token> {
            None.or_else(|| self.string())
                .or_else(|| self.character())
                .or_else(|| self.float())
                .or_else(|| self.integer())
        }
        /// Evaluates delimiter rules
        pub fn delimiter(&mut self) -> Option<Token> {
            None.or_else(|| self.l_brack())
                .or_else(|| self.r_brack())
                .or_else(|| self.l_curly())
                .or_else(|| self.r_curly())
                .or_else(|| self.l_paren())
                .or_else(|| self.r_paren())
        }
        /// Evaluates punctuation rules
        pub fn punctuation(&mut self) -> Option<Token> {
            None.or_else(|| self.amp_amp())
                .or_else(|| self.bar_bar())
                .or_else(|| self.not_not())
                .or_else(|| self.cat_ear())
                .or_else(|| self.eq_eq())
                .or_else(|| self.gt_eq())
                .or_else(|| self.lt_eq())
                .or_else(|| self.not_eq())
                .or_else(|| self.lsh_eq())
                .or_else(|| self.rsh_eq())
                .or_else(|| self.star_eq())
                .or_else(|| self.div_eq())
                .or_else(|| self.rem_eq())
                .or_else(|| self.add_eq())
                .or_else(|| self.sub_eq())
                .or_else(|| self.and_eq())
                .or_else(|| self.or_eq())
                .or_else(|| self.xor_eq())
                .or_else(|| self.lsh())
                .or_else(|| self.rsh())
                .or_else(|| self.arrow())
                .or_else(|| self.fatarrow())
                .or_else(|| self.semi())
                .or_else(|| self.dot())
                .or_else(|| self.star())
                .or_else(|| self.div())
                .or_else(|| self.plus())
                .or_else(|| self.sub())
                .or_else(|| self.rem())
                .or_else(|| self.bang())
                .or_else(|| self.eq())
                .or_else(|| self.lt())
                .or_else(|| self.gt())
                .or_else(|| self.amp())
                .or_else(|| self.bar())
                .or_else(|| self.xor())
                .or_else(|| self.hash())
                .or_else(|| self.at())
                .or_else(|| self.colon())
                .or_else(|| self.backslash())
                .or_else(|| self.question())
                .or_else(|| self.comma())
                .or_else(|| self.tilde())
                .or_else(|| self.grave())
        }
        pub fn unary_op(&mut self) -> Option<Token> {
            self.bang().or_else(|| self.sub())
        }
        // functions for lexing individual tokens
        pub fn invalid(&mut self) -> Option<Token> {
            self.map_rule(|r| r.invalid(), Type::Invalid)
        }
        // comments
        pub fn comment(&mut self) -> Option<Token> {
            self.map_rule(|r| r.comment(), Type::Comment)
        }
        // identifiers
        pub fn identifier(&mut self) -> Option<Token> {
            self.map_rule(|r| r.identifier(), Type::Identifier)
                .map(|token| match self.text[token.range()].parse() {
                    Ok(kw) => token.cast(Type::Keyword(kw)),
                    Err(_) => token,
                })
        }
        // literals
        pub fn integer(&mut self) -> Option<Token> {
            self.map_rule(|r| r.integer(), Type::Integer)
        }
        pub fn float(&mut self) -> Option<Token> {
            self.map_rule(|r| r.float(), Type::Float)
        }
        pub fn string(&mut self) -> Option<Token> {
            // TODO: count lines and columns properly within string
            self.map_rule(|r| r.string(), Type::String)
                .map(|t| t.rebound(t.head + 1, t.tail - 1))
        }
        pub fn character(&mut self) -> Option<Token> {
            self.map_rule(|r| r.character(), Type::Character)
                .map(|t| t.rebound(t.head + 1, t.tail - 1))
        }
        // delimiters
        pub fn l_brack(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('['), Type::LBrack)
        }
        pub fn r_brack(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char(']'), Type::RBrack)
        }
        pub fn l_curly(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('{'), Type::LCurly)
        }
        pub fn r_curly(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('}'), Type::RCurly)
        }
        pub fn l_paren(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('('), Type::LParen)
        }
        pub fn r_paren(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char(')'), Type::RParen)
        }
        // compound punctuation
        pub fn lsh(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("<<"), Type::Lsh)
        }
        pub fn rsh(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str(">>"), Type::Rsh)
        }
        pub fn amp_amp(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("&&"), Type::AmpAmp)
        }
        pub fn bar_bar(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("||"), Type::BarBar)
        }
        pub fn not_not(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("!!"), Type::NotNot)
        }
        pub fn cat_ear(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("^^"), Type::CatEar)
        }
        pub fn eq_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("=="), Type::EqEq)
        }
        pub fn gt_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str(">="), Type::GtEq)
        }
        pub fn lt_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("<="), Type::LtEq)
        }
        pub fn not_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("!="), Type::NotEq)
        }
        pub fn star_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("*="), Type::StarEq)
        }
        pub fn div_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("/="), Type::DivEq)
        }
        pub fn rem_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("%="), Type::RemEq)
        }
        pub fn add_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("+="), Type::AddEq)
        }
        pub fn sub_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("-="), Type::SubEq)
        }
        pub fn and_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("&="), Type::AndEq)
        }
        pub fn or_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("|="), Type::OrEq)
        }
        pub fn xor_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("^="), Type::XorEq)
        }
        pub fn lsh_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("<<="), Type::LshEq)
        }
        pub fn rsh_eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str(">>="), Type::RshEq)
        }
        pub fn arrow(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("->"), Type::Arrow)
        }
        pub fn fatarrow(&mut self) -> Option<Token> {
            self.map_rule(|r| r.str("=>"), Type::FatArrow)
        }
        // simple punctuation
        pub fn semi(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char(';'), Type::Semi)
        }
        pub fn dot(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('.'), Type::Dot)
        }
        pub fn star(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('*'), Type::Star)
        }
        pub fn div(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('/'), Type::Div)
        }
        pub fn plus(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('+'), Type::Plus)
        }
        pub fn sub(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('-'), Type::Minus)
        }
        pub fn rem(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('%'), Type::Rem)
        }
        pub fn bang(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('!'), Type::Bang)
        }
        pub fn eq(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('='), Type::Eq)
        }
        pub fn lt(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('<'), Type::Lt)
        }
        pub fn gt(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('>'), Type::Gt)
        }
        pub fn amp(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('&'), Type::Amp)
        }
        pub fn bar(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('|'), Type::Bar)
        }
        pub fn xor(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('^'), Type::Xor)
        }
        pub fn hash(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('#'), Type::Hash)
        }
        pub fn at(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('@'), Type::At)
        }
        pub fn colon(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char(':'), Type::Colon)
        }
        pub fn question(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('?'), Type::Question)
        }
        pub fn comma(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char(','), Type::Comma)
        }
        pub fn tilde(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('~'), Type::Tilde)
        }
        pub fn grave(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('`'), Type::Grave)
        }
        pub fn backslash(&mut self) -> Option<Token> {
            self.map_rule(|r| r.char('\\'), Type::Backslash)
        }
    }

    // TODO: use real, functional parser-combinators here to produce tokens
    /// A lexer [Rule] matches patterns in text in a declarative manner
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Rule<'t> {
        text: &'t str,
        taken: usize,
        is_alright: bool,
    }
    impl<'t> Rule<'t> {
        pub fn new(text: &'t str) -> Self {
            Self { text, taken: 0, is_alright: true }
        }
        pub fn end(self) -> Option<usize> {
            self.is_alright.then_some(self.taken)
        }
        pub fn remaining(&self) -> &str {
            self.text
        }
    }

    impl<'t> Rule<'t> {
        /// Matches any sequence of non-whitespace characters
        pub fn invalid(self) -> Self {
            self.and_many(Self::not_whitespace)
        }
        /// Matches a block, line, or shebang comment
        pub fn comment(self) -> Self {
            self.and_either(Self::line_comment, Self::block_comment)
        }
        /// Matches a line or shebang comment
        fn line_comment(self) -> Self {
            // line_comment := ("//" | "#!/") (!newline)*
            self.str("//")
                .or(|r| r.str("#!/"))
                .and_any(|r| r.not_char('\n'))
        }
        /// Matches a block comment
        fn block_comment(self) -> Self {
            // block_comment := "/*" (block_comment | all_but("*/"))* "*/"
            self.str("/*")
                .and_any(|r| r.and_either(|f| f.block_comment(), |g| g.not_str("*/")))
                .str("*/")
        }
        /// Matches a Rust-style identifier
        pub fn identifier(self) -> Self {
            // identifier := ('_' | XID_START) ~ XID_CONTINUE*
            self.char('_')
                .or(Rule::xid_start)
                .and_any(Rule::xid_continue)
        }
        /// Matches a Rust-style base-prefixed int literal
        fn integer_kind(self, prefix: &str, digit: impl Fn(Self) -> Self) -> Self {
            // int_kind<Prefix, Digit> := Prefix '_'* Digit (Digit | '_')*
            self.str(prefix)
                .and_any(|r| r.char('_'))
                .and(&digit)
                .and_any(|r| r.and(&digit).or(|r| r.char('_')))
        }
        /// Matches a Rust-style integer literal
        pub fn integer(self) -> Self {
            // integer = (int_kind<0d, dec_digit> | int_kind<0x, hex_digit>
            //           | int_kind<0o, oct_digit> | int_kind<0b, bin_digit> | dec_digit (dec_digit | '_')*)
            self.and_one_of(&[
                &|rule| rule.integer_kind("0d", Rule::dec_digit),
                &|rule| rule.integer_kind("0x", Rule::hex_digit),
                &|rule| rule.integer_kind("0o", Rule::oct_digit),
                &|rule| rule.integer_kind("0b", Rule::bin_digit),
                &|rule| {
                    rule.dec_digit()
                        .and_any(|r| r.dec_digit().or(|r| r.char('_')))
                },
            ])
        }
        /// Matches a float literal
        // TODO: exponent form
        pub fn float(self) -> Self {
            self.and_any(Rule::dec_digit)
                .char('.')
                .and_many(Rule::dec_digit)
        }
        /// Matches one apostrophe-delimited char literal
        pub fn character(self) -> Self {
            self.char('\'').character_continue().char('\'')
        }
        pub fn character_continue(self) -> Self {
            self.and(|rule| rule.string_escape().or(|rule| rule.not_char('\'')))
        }
        /// Matches one quote-delimited string literal
        pub fn string(self) -> Self {
            self.char('"').and_any(Rule::string_continue).char('"')
        }
        /// Matches one string escape sequence or non-`"` characcter
        pub fn string_continue(self) -> Self {
            self.and(Rule::string_escape).or(|rule| rule.not_char('"'))
        }
    }

    impl<'t> Rule<'t> {
        /// Matches a char lexicographically between start and end
        pub fn char_between(self, start: char, end: char) -> Self {
            self.char_fn(|c| start <= c && c <= end)
        }
        /// Matches a single char
        pub fn char(self, c: char) -> Self {
            self.has(|rule| rule.text.starts_with(c), 1)
        }
        /// Matches the entirety of a string slice
        pub fn str(self, s: &str) -> Self {
            self.has(|rule| rule.text.starts_with(s), s.len())
        }
        /// Matches a char based on the output of a function
        pub fn char_fn(self, f: impl Fn(char) -> bool) -> Self {
            self.and(|rule| match rule.text.strip_prefix(&f) {
                Some(text) => Self { text, taken: rule.taken + next_utf8(rule.text, 1), ..rule },
                None => Self { is_alright: false, ..rule },
            })
        }
        /// Matches a single char except c
        pub fn not_char(self, c: char) -> Self {
            self.has(|rule| !rule.text.starts_with(c), 1)
        }
        /// Matches a single char unless the text starts with s
        pub fn not_str(self, s: &str) -> Self {
            self.has(|rule| !rule.text.starts_with(s), 1)
        }
        // commonly used character classes
        /// Matches one of any character
        pub fn any(self) -> Self {
            self.has(|_| true, 1)
        }
        /// Matches one whitespace
        pub fn whitespace(self) -> Self {
            self.char_fn(|c| c.is_whitespace())
        }
        /// Matches one whitespace, except `'\n'`
        pub fn whitespace_not_newline(self) -> Self {
            self.char_fn(|c| '\n' != c && c.is_whitespace())
        }
        /// Matches anything but whitespace
        pub fn not_whitespace(self) -> Self {
            self.char_fn(|c| !c.is_whitespace())
        }
        /// Matches one XID_START
        pub fn xid_start(self) -> Self {
            use unicode_xid::UnicodeXID;
            self.char_fn(UnicodeXID::is_xid_start)
        }
        /// Matches one XID_CONTINUE
        pub fn xid_continue(self) -> Self {
            use unicode_xid::UnicodeXID;
            self.char_fn(UnicodeXID::is_xid_continue)
        }
        /// Matches one hexadecimal digit
        pub fn hex_digit(self) -> Self {
            self.char_fn(|c| c.is_ascii_hexdigit())
        }
        /// Matches one decimal digit
        pub fn dec_digit(self) -> Self {
            self.char_fn(|c| c.is_ascii_digit())
        }
        /// Matches one octal digit
        pub fn oct_digit(self) -> Self {
            self.char_between('0', '7')
        }
        /// Matches one binary digit
        pub fn bin_digit(self) -> Self {
            self.char_between('0', '1')
        }
        /// Matches any string escape "\."
        pub fn string_escape(self) -> Self {
            self.char('\\').and(Rule::any)
        }
        /// Performs a consuming condition assertion on the input
        fn has(self, condition: impl Fn(&Self) -> bool, len: usize) -> Self {
            let len = next_utf8(self.text, len);
            self.and(|rule| match condition(&rule) && !rule.text.is_empty() {
                true => Self { text: &rule.text[len..], taken: rule.taken + len, ..rule },
                false => Self { is_alright: false, ..rule },
            })
        }
    }

    impl<'t> lerox::Combinator for Rule<'t> {
        fn is_alright(&self) -> bool {
            self.is_alright
        }
        fn into_alright(self) -> Self {
            Self { is_alright: true, ..self }
        }
    }

    /// Returns the index of the next unicode character, rounded up
    fn next_utf8(text: &str, mut index: usize) -> usize {
        index = index.min(text.len());
        while !text.is_char_boundary(index) {
            index += 1
        }
        index
    }
}

pub mod parser {
    //! Parses tokens into an AST
}

pub mod interpreter {
    //! Interprets an AST as a program
}

#[cfg(test)]
mod tests {
    mod token {
        use crate::token::*;
        #[test]
        fn token_has_type() {
            assert_eq!(Token::new(Type::Comment, 0, 10, 1, 1).ty(), Type::Comment);
            assert_eq!(
                Token::new(Type::Identifier, 0, 10, 1, 1).ty(),
                Type::Identifier
            );
        }
        #[test]
        fn token_has_range() {
            let t = Token::new(Type::Comment, 0, 10, 1, 1);
            assert_eq!(t.range(), 0..10);
        }
    }
    mod ast {
        // TODO
    }
    mod lexer {
        use std::ops::Range;

        use crate::{
            lexer::*,
            token::{Token, Type},
        };

        fn assert_whole_input_is_token<'t, F>(input: &'t str, f: F, ty: Type)
        where F: FnOnce(&mut Lexer<'t>) -> Option<Token> {
            assert_has_type_and_range(input, f, ty, 0..input.len())
        }
        fn assert_has_type_and_range<'t, F>(input: &'t str, f: F, ty: Type, range: Range<usize>)
        where F: FnOnce(&mut Lexer<'t>) -> Option<Token> {
            let tok =
                f(&mut Lexer::new(input)).unwrap_or_else(|| panic!("Should be {ty:?}, {range:?}"));
            assert_eq!(ty, tok.ty());
            assert_eq!(range, tok.range());
        }

        mod comment {
            use super::*;

            #[test]
            fn line_comment() {
                assert_whole_input_is_token("// comment!", Lexer::comment, Type::Comment);
            }
            #[test]
            #[should_panic]
            fn not_line_comment() {
                assert_whole_input_is_token("fn main() {}", Lexer::comment, Type::Comment);
            }
            #[test]
            fn block_comment() {
                assert_whole_input_is_token("/* comment! */", Lexer::comment, Type::Comment);
            }
            #[test]
            fn nested_block_comment() {
                assert_whole_input_is_token(
                    "/* a /* nested */ comment */",
                    Lexer::comment,
                    Type::Comment,
                );
            }
            #[test]
            #[should_panic]
            fn unclosed_nested_comment() {
                assert_whole_input_is_token(
                    "/* improperly /* nested */ comment",
                    Lexer::comment,
                    Type::Comment,
                );
            }
            #[test]
            #[should_panic]
            fn not_block_comment() {
                assert_whole_input_is_token("fn main() {}", Lexer::comment, Type::Comment);
            }
            #[test]
            fn shebang_comment() {
                assert_whole_input_is_token("#!/ comment!", Lexer::comment, Type::Comment);
            }
            #[test]
            #[should_panic]
            fn not_shebang_comment() {
                assert_whole_input_is_token("fn main() {}", Lexer::comment, Type::Comment);
            }
        }
        mod identifier {
            use super::*;

            #[test]
            fn identifier() {
                assert_whole_input_is_token(
                    "valid_identifier",
                    Lexer::identifier,
                    Type::Identifier,
                );
                assert_whole_input_is_token("_0", Lexer::identifier, Type::Identifier);
                assert_whole_input_is_token("_", Lexer::identifier, Type::Identifier);
            }
            #[test]
            fn unicode_identifier() {
                assert_whole_input_is_token("ζ_ζζζ_ζζζ_ζζζ", Lexer::identifier, Type::Identifier);
                assert_whole_input_is_token("_ζζζ_ζζζ_ζζζ_", Lexer::identifier, Type::Identifier);
            }
            #[test]
            #[should_panic]
            fn not_identifier() {
                assert_whole_input_is_token("123456789", Lexer::identifier, Type::Identifier);
            }
        }
        mod literal {
            use super::*;
            #[test]
            fn literal_class() {
                assert_whole_input_is_token("1_00000", Lexer::literal, Type::Integer);
                assert_whole_input_is_token("1.00000", Lexer::literal, Type::Float);
                assert_has_type_and_range("\"1.0\"", Lexer::literal, Type::String, 1..4);
                assert_has_type_and_range("'\"'", Lexer::literal, Type::Character, 1..2);
            }
            mod integer {
                use super::*;
                #[test]
                fn bare() {
                    assert_whole_input_is_token("10010110", Lexer::integer, Type::Integer);
                    assert_whole_input_is_token("12345670", Lexer::integer, Type::Integer);
                    assert_whole_input_is_token("1234567890", Lexer::integer, Type::Integer);
                }
                #[test]
                fn base16() {
                    assert_has_type_and_range("0x1234", Lexer::integer, Type::Integer, 0..6);
                    assert_has_type_and_range(
                        "0x1234 \"hello\"",
                        Lexer::integer,
                        Type::Integer,
                        0..6,
                    );
                }
                #[test]
                fn base10() {
                    assert_whole_input_is_token("0d1234", Lexer::integer, Type::Integer);
                }
                #[test]
                fn base8() {
                    assert_whole_input_is_token("0o1234", Lexer::integer, Type::Integer);
                }
                #[test]
                fn base2() {
                    assert_whole_input_is_token("0b1010", Lexer::integer, Type::Integer);
                }
            }
            mod float {
                use super::*;
                #[test]
                fn number_dot_number_is_float() {
                    assert_whole_input_is_token("1.0", Lexer::float, Type::Float);
                }
                #[test]
                fn nothing_dot_number_is_float() {
                    assert_whole_input_is_token(".0", Lexer::float, Type::Float);
                }
                #[test]
                #[should_panic]
                fn number_dot_nothing_is_not_float() {
                    assert_whole_input_is_token("1.", Lexer::float, Type::Float);
                }
                #[test]
                #[should_panic]
                fn nothing_dot_nothing_is_not_float() {
                    assert_whole_input_is_token(".", Lexer::float, Type::Float);
                }
            }
            mod string {
                use super::*;
                #[test]
                fn empty_string() {
                    assert_has_type_and_range("\"\"", Lexer::string, Type::String, 1..1);
                }
                #[test]
                fn unicode_string() {
                    assert_has_type_and_range("\"I 💙 🦈!\"", Lexer::string, Type::String, 1..13);
                }
                #[test]
                fn escape_string() {
                    assert_has_type_and_range(
                        "\" \\\"This is a quote\\\" \"",
                        Lexer::string,
                        Type::String,
                        1..22
                    );
                }
            }
            mod char {
                use super::*;
                #[test]
                fn plain_char() {
                    assert_has_type_and_range("'A'", Lexer::character, Type::Character, 1..2);
                    assert_has_type_and_range("'a'", Lexer::character, Type::Character, 1..2);
                    assert_has_type_and_range("'#'", Lexer::character, Type::Character, 1..2);
                }
                #[test]
                fn unicode_char() {
                    assert_has_type_and_range("'ε'", Lexer::character, Type::Character, 1..3);
                }
                #[test]
                fn escaped_char() {
                    assert_has_type_and_range("'\\n'", Lexer::character, Type::Character, 1..3);
                }
                #[test]
                #[should_panic]
                fn no_char() {
                    assert_has_type_and_range("''", Lexer::character, Type::Character, 1..1);
                }
            }
        }
        mod delimiter {
            use super::*;
            #[test]
            fn delimiter_class() {
                assert_whole_input_is_token("[", Lexer::delimiter, Type::LBrack);
                assert_whole_input_is_token("]", Lexer::delimiter, Type::RBrack);
                assert_whole_input_is_token("{", Lexer::delimiter, Type::LCurly);
                assert_whole_input_is_token("}", Lexer::delimiter, Type::RCurly);
                assert_whole_input_is_token("(", Lexer::delimiter, Type::LParen);
                assert_whole_input_is_token(")", Lexer::delimiter, Type::RParen);
            }
            #[test]
            fn l_brack() {
                assert_whole_input_is_token("[", Lexer::l_brack, Type::LBrack);
            }
            #[test]
            fn r_brack() {
                assert_whole_input_is_token("]", Lexer::r_brack, Type::RBrack);
            }
            #[test]
            fn l_curly() {
                assert_whole_input_is_token("{", Lexer::l_curly, Type::LCurly);
            }
            #[test]
            fn r_curly() {
                assert_whole_input_is_token("}", Lexer::r_curly, Type::RCurly);
            }

            #[test]
            fn l_paren() {
                assert_whole_input_is_token("(", Lexer::l_paren, Type::LParen);
            }
            #[test]
            fn r_paren() {
                assert_whole_input_is_token(")", Lexer::r_paren, Type::RParen);
            }
        }
        mod punctuation {
            use super::*;
            mod compound {
                use super::*;

                #[test]
                fn lsh() {
                    assert_whole_input_is_token("<<", Lexer::lsh, Type::Lsh)
                }
                #[test]
                fn rsh() {
                    assert_whole_input_is_token(">>", Lexer::rsh, Type::Rsh)
                }
                #[test]
                fn amp_amp() {
                    assert_whole_input_is_token("&&", Lexer::amp_amp, Type::AmpAmp)
                }
                #[test]
                fn bar_bar() {
                    assert_whole_input_is_token("||", Lexer::bar_bar, Type::BarBar)
                }
                #[test]
                fn not_not() {
                    assert_whole_input_is_token("!!", Lexer::not_not, Type::NotNot)
                }
                #[test]
                fn cat_ear() {
                    assert_whole_input_is_token("^^", Lexer::cat_ear, Type::CatEar)
                }
                #[test]
                fn eq_eq() {
                    assert_whole_input_is_token("==", Lexer::eq_eq, Type::EqEq)
                }
                #[test]
                fn gt_eq() {
                    assert_whole_input_is_token(">=", Lexer::gt_eq, Type::GtEq)
                }
                #[test]
                fn lt_eq() {
                    assert_whole_input_is_token("<=", Lexer::lt_eq, Type::LtEq)
                }
                #[test]
                fn not_eq() {
                    assert_whole_input_is_token("!=", Lexer::not_eq, Type::NotEq)
                }
                #[test]
                fn star_eq() {
                    assert_whole_input_is_token("*=", Lexer::star_eq, Type::StarEq)
                }
                #[test]
                fn div_eq() {
                    assert_whole_input_is_token("/=", Lexer::div_eq, Type::DivEq)
                }
                #[test]
                fn add_eq() {
                    assert_whole_input_is_token("+=", Lexer::add_eq, Type::AddEq)
                }
                #[test]
                fn sub_eq() {
                    assert_whole_input_is_token("-=", Lexer::sub_eq, Type::SubEq)
                }
                #[test]
                fn and_eq() {
                    assert_whole_input_is_token("&=", Lexer::and_eq, Type::AndEq)
                }
                #[test]
                fn or_eq() {
                    assert_whole_input_is_token("|=", Lexer::or_eq, Type::OrEq)
                }
                #[test]
                fn xor_eq() {
                    assert_whole_input_is_token("^=", Lexer::xor_eq, Type::XorEq)
                }
                #[test]
                fn lsh_eq() {
                    assert_whole_input_is_token("<<=", Lexer::lsh_eq, Type::LshEq)
                }
                #[test]
                fn rsh_eq() {
                    assert_whole_input_is_token(">>=", Lexer::rsh_eq, Type::RshEq)
                }
            }

            mod simple {
                use super::*;
                #[test]
                fn punctuation_class() {
                    assert_whole_input_is_token(";", Lexer::punctuation, Type::Semi);
                    assert_whole_input_is_token(".", Lexer::punctuation, Type::Dot);
                    assert_whole_input_is_token("*", Lexer::punctuation, Type::Star);
                    assert_whole_input_is_token("/", Lexer::punctuation, Type::Div);
                    assert_whole_input_is_token("+", Lexer::punctuation, Type::Plus);
                    assert_whole_input_is_token("-", Lexer::punctuation, Type::Minus);
                    assert_whole_input_is_token("%", Lexer::punctuation, Type::Rem);
                    assert_whole_input_is_token("!", Lexer::punctuation, Type::Bang);
                    assert_whole_input_is_token("=", Lexer::punctuation, Type::Eq);
                    assert_whole_input_is_token("<", Lexer::punctuation, Type::Lt);
                    assert_whole_input_is_token(">", Lexer::punctuation, Type::Gt);
                    assert_whole_input_is_token("&", Lexer::punctuation, Type::Amp);
                    assert_whole_input_is_token("|", Lexer::punctuation, Type::Bar);
                    assert_whole_input_is_token("^", Lexer::punctuation, Type::Xor);
                    assert_whole_input_is_token("#", Lexer::punctuation, Type::Hash);
                    assert_whole_input_is_token("@", Lexer::punctuation, Type::At);
                    assert_whole_input_is_token(":", Lexer::punctuation, Type::Colon);
                    assert_whole_input_is_token("?", Lexer::punctuation, Type::Question);
                    assert_whole_input_is_token(",", Lexer::punctuation, Type::Comma);
                    assert_whole_input_is_token("~", Lexer::punctuation, Type::Tilde);
                    assert_whole_input_is_token("`", Lexer::punctuation, Type::Grave);
                    assert_whole_input_is_token("\\", Lexer::punctuation, Type::Backslash);
                }
                // individual functions below
                #[test]
                fn semi() {
                    assert_whole_input_is_token(";", Lexer::semi, Type::Semi)
                }
                #[test]
                fn dot() {
                    assert_whole_input_is_token(".", Lexer::dot, Type::Dot)
                }
                #[test]
                fn star() {
                    assert_whole_input_is_token("*", Lexer::star, Type::Star)
                }
                #[test]
                fn div() {
                    assert_whole_input_is_token("/", Lexer::div, Type::Div)
                }
                #[test]
                fn plus() {
                    assert_whole_input_is_token("+", Lexer::plus, Type::Plus)
                }
                #[test]
                fn minus() {
                    assert_whole_input_is_token("-", Lexer::sub, Type::Minus)
                }
                #[test]
                fn rem() {
                    assert_whole_input_is_token("%", Lexer::rem, Type::Rem)
                }
                #[test]
                fn bang() {
                    assert_whole_input_is_token("!", Lexer::bang, Type::Bang)
                }
                #[test]
                fn eq() {
                    assert_whole_input_is_token("=", Lexer::eq, Type::Eq)
                }
                #[test]
                fn lt() {
                    assert_whole_input_is_token("<", Lexer::lt, Type::Lt)
                }
                #[test]
                fn gt() {
                    assert_whole_input_is_token(">", Lexer::gt, Type::Gt)
                }
                #[test]
                fn and() {
                    assert_whole_input_is_token("&", Lexer::amp, Type::Amp)
                }
                #[test]
                fn or() {
                    assert_whole_input_is_token("|", Lexer::bar, Type::Bar)
                }
                #[test]
                fn xor() {
                    assert_whole_input_is_token("^", Lexer::xor, Type::Xor)
                }
                #[test]
                fn hash() {
                    assert_whole_input_is_token("#", Lexer::hash, Type::Hash)
                }
                #[test]
                fn at() {
                    assert_whole_input_is_token("@", Lexer::at, Type::At)
                }
                #[test]
                fn colon() {
                    assert_whole_input_is_token(":", Lexer::colon, Type::Colon)
                }
                #[test]
                fn backslash() {
                    assert_whole_input_is_token("\\", Lexer::backslash, Type::Backslash)
                }
                #[test]
                fn question() {
                    assert_whole_input_is_token("?", Lexer::question, Type::Question)
                }
                #[test]
                fn comma() {
                    assert_whole_input_is_token(",", Lexer::comma, Type::Comma)
                }
                #[test]
                fn tilde() {
                    assert_whole_input_is_token("~", Lexer::tilde, Type::Tilde)
                }
                #[test]
                fn grave() {
                    assert_whole_input_is_token("`", Lexer::grave, Type::Grave)
                }
            }
        }
    }
    mod parser {
        // TODO
    }
    mod interpreter {
        // TODO
    }
}
