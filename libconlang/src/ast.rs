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

pub mod visitor {
    //! A [`Visitor`] visits every kind of node in the [Abstract Syntax Tree](super). Nodes, conversely are [`Walkers`](Walk) for Visitors which return a [`Result<(), E>`](Result)
    use super::{
        expression::{
            control::*,
            math::{operator::*, *},
            Block, *,
        },
        literal::*,
        *,
    };
    /// A [Walker](Walk) is a node in the AST, and calls [`Visitor::visit_*()`](Visitor) on all its children
    pub trait Walk<T: Visitor<R> + ?Sized, R> {
        /// Traverses the children of this node in order, calling the appropriate [Visitor] function
        fn walk(&self, visitor: &mut T) -> R;
    }
    mod walker {
        use super::*;
        macro leaf($($T:ty),*$(,)?) {$(
            impl<T: Visitor<Result<(), E>> + ?Sized, E> Walk<T, Result<(), E>> for $T {
                #[doc = concat!("A(n) [`", stringify!($T), "`] is a leaf node.")]
                /// Calling this will do nothing.
                fn walk(&self, _visitor: &mut T) -> Result<(), E> { Ok(()) }
            }
        )*}
        leaf!(Binary, bool, char, Continue, Float, Identifier, str, u128, Unary);
        impl<T: Visitor<Result<(), E>> + ?Sized, E> Walk<T, Result<(), E>> for While {
            fn walk(&self, visitor: &mut T) -> Result<(), E> {
                visitor.visit_expr(&self.cond)?;
                visitor.visit_block(&self.body)?;
                match &self.else_ {
                    Some(expr) => visitor.visit_else(expr),
                    None => Ok(()),
                }
            }
        }
        impl<T: Visitor<Result<(), E>> + ?Sized, E> Walk<T, Result<(), E>> for If {
            fn walk(&self, visitor: &mut T) -> Result<(), E> {
                visitor.visit_expr(&self.cond)?;
                visitor.visit_block(&self.body)?;
                match &self.else_ {
                    Some(expr) => visitor.visit_else(expr),
                    None => Ok(()),
                }
            }
        }
        impl<T: Visitor<Result<(), E>> + ?Sized, E> Walk<T, Result<(), E>> for For {
            fn walk(&self, visitor: &mut T) -> Result<(), E> {
                visitor.visit_identifier(&self.var)?;
                visitor.visit_expr(&self.iter)?;
                visitor.visit_block(&self.body)?;
                match &self.else_ {
                    Some(expr) => visitor.visit_else(expr),
                    None => Ok(()),
                }
            }
        }
        impl<T: Visitor<Result<(), E>> + ?Sized, E> Walk<T, Result<(), E>> for Else {
            fn walk(&self, visitor: &mut T) -> Result<(), E> {
                visitor.visit_block(&self.block)
            }
        }
        impl<T: Visitor<Result<(), E>> + ?Sized, E> Walk<T, Result<(), E>> for Return {
            fn walk(&self, visitor: &mut T) -> Result<(), E> {
                visitor.visit_expr(&self.expr)
            }
        }
        impl<T: Visitor<Result<(), E>> + ?Sized, E> Walk<T, Result<(), E>> for Break {
            fn walk(&self, visitor: &mut T) -> Result<(), E> {
                visitor.visit_expr(&self.expr)
            }
        }
        impl<T: Visitor<Result<(), E>> + ?Sized, E> Walk<T, Result<(), E>> for Start {
            fn walk(&self, visitor: &mut T) -> Result<(), E> {
                visitor.visit_expr(&self.0)
            }
        }
        impl<T: Visitor<Result<(), E>> + ?Sized, E> Walk<T, Result<(), E>> for Expr {
            fn walk(&self, visitor: &mut T) -> Result<(), E> {
                visitor.visit_operation(&self.ignore)
            }
        }
        impl<T: Visitor<Result<(), E>> + ?Sized, E> Walk<T, Result<(), E>> for Group {
            fn walk(&self, visitor: &mut T) -> Result<(), E> {
                match self {
                    Group::Expr(expr) => visitor.visit_expr(expr),
                    Group::Empty => visitor.visit_empty(),
                }
            }
        }
        impl<T: Visitor<Result<(), E>> + ?Sized, E> Walk<T, Result<(), E>> for Block {
            fn walk(&self, visitor: &mut T) -> Result<(), E> {
                visitor.visit_expr(&self.expr)
            }
        }
        impl<T: Visitor<Result<(), E>> + ?Sized, E> Walk<T, Result<(), E>> for Operation {
            fn walk(&self, visitor: &mut T) -> Result<(), E> {
                match self {
                    Operation::Binary { first, other } => {
                        visitor.visit_operation(first)?;
                        for (op, other) in other {
                            visitor.visit_binary_op(op)?;
                            visitor.visit_operation(other)?;
                        }
                        Ok(())
                    }
                    Operation::Unary { operators, operand } => {
                        for op in operators {
                            visitor.visit_unary_op(op)?;
                        }
                        visitor.visit_primary(operand)
                    }
                }
            }
        }
        impl<T: Visitor<Result<(), E>> + ?Sized, E> Walk<T, Result<(), E>> for Primary {
            fn walk(&self, visitor: &mut T) -> Result<(), E> {
                match self {
                    Primary::Identifier(i) => visitor.visit_identifier(i),
                    Primary::Literal(l) => visitor.visit_literal(l),
                    Primary::Block(b) => visitor.visit_block(b),
                    Primary::Group(g) => visitor.visit_group(g),
                    Primary::Branch(b) => visitor.visit_branch_expr(b),
                }
            }
        }
        impl<T: Visitor<Result<(), E>> + ?Sized, E> Walk<T, Result<(), E>> for Literal {
            fn walk(&self, visitor: &mut T) -> Result<(), E> {
                match self {
                    Literal::String(s) => visitor.visit_string_literal(s),
                    Literal::Char(c) => visitor.visit_char_literal(c),
                    Literal::Bool(b) => visitor.visit_bool_literal(b),
                    Literal::Float(f) => visitor.visit_float_literal(f),
                    Literal::Int(i) => visitor.visit_int_literal(i),
                }
            }
        }
        impl<T: Visitor<Result<(), E>> + ?Sized, E> Walk<T, Result<(), E>> for Flow {
            fn walk(&self, visitor: &mut T) -> Result<(), E> {
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

    /// A Visitor traverses every kind of node in the [Abstract Syntax Tree](super)
    pub trait Visitor<R> {
        /// Visit the start of an AST
        fn visit(&mut self, start: &Start) -> R {
            self.visit_expr(&start.0)
        }

        /// Visit an [Expression](Expr)
        fn visit_expr(&mut self, expr: &Expr) -> R {
            self.visit_operation(&expr.ignore)
        }
        // Block expression
        /// Visit a [Block] expression
        fn visit_block(&mut self, expr: &Block) -> R {
            self.visit_expr(&expr.expr)
        }
        /// Visit a [Group] expression
        fn visit_group(&mut self, group: &Group) -> R {
            match group {
                Group::Expr(expr) => self.visit_expr(expr),
                Group::Empty => self.visit_empty(),
            }
        }

        // Math expression
        /// Visit an [Operation]
        fn visit_operation(&mut self, expr: &Operation) -> R;
        /// Visit a [Binary](Operation::Binary) [operator](operator::Binary)
        // Math operators
        fn visit_binary_op(&mut self, op: &operator::Binary) -> R;
        /// Visit a [Unary](Operation::Unary) [operator](operator::Unary)
        fn visit_unary_op(&mut self, op: &operator::Unary) -> R;

        /// Visit a [Primary] expression
        ///
        /// [Primary] := [Identifier] | [Literal] | [Block] | [Flow]
        fn visit_primary(&mut self, expr: &Primary) -> R {
            match expr {
                Primary::Identifier(v) => self.visit_identifier(v),
                Primary::Literal(v) => self.visit_literal(v),
                Primary::Block(v) => self.visit_block(v),
                Primary::Group(v) => self.visit_group(v),
                Primary::Branch(v) => self.visit_branch_expr(v),
            }
        }

        /// Visit a [Flow] expression.
        ///
        /// [Flow] := [While] | [If] | [For]
        fn visit_branch_expr(&mut self, expr: &Flow) -> R {
            match expr {
                Flow::While(e) => self.visit_while(e),
                Flow::If(e) => self.visit_if(e),
                Flow::For(e) => self.visit_for(e),
                Flow::Continue(e) => self.visit_continue(e),
                Flow::Return(e) => self.visit_return(e),
                Flow::Break(e) => self.visit_break(e),
            }
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
            match literal {
                Literal::String(l) => self.visit_string_literal(l),
                Literal::Char(l) => self.visit_char_literal(l),
                Literal::Bool(l) => self.visit_bool_literal(l),
                Literal::Float(l) => self.visit_float_literal(l),
                Literal::Int(l) => self.visit_int_literal(l),
            }
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

        /// Visit an Empty
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
/// [`Identifier`] := [`IDENTIFIER`](crate::token::token_type::Type::Identifier)
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
        /// [`Literal::String`] := [`STRING`](crate::token::token_type::Type::String)
        String(String),
        /// Represents a literal [char] value
        /// # Syntax
        /// [`Literal::Char`] := [`CHARACTER`](crate::token::token_type::Type::Character)
        Char(char),
        /// Represents a literal [bool] value
        /// # Syntax
        /// [`Literal::Bool`] :=
        ///     [`TRUE`](crate::token::token_type::Keyword::True)
        ///   | [`FALSE`](crate::token::token_type::Keyword::False)
        Bool(bool),
        /// Represents a literal float value
        /// # Syntax
        /// [`Float`] := [`FLOAT`](crate::token::token_type::Type::Float)
        Float(Float),
        /// Represents a literal integer value
        /// # Syntax
        /// [`u128`] := [`INTEGER`](crate::token::token_type::Type::Integer)
        Int(u128),
    }

    /// Represents a literal float value
    /// # Syntax
    /// [`Float`] := [`FLOAT`](crate::token::token_type::Type::Float)
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
    //! |  1 |  [`Ignore`](math) | Ignores the preceding sub-expression's result
    //! |  2 |  [`Assign`](math) | Assignment
    //! |  3 | [`Compare`](math) | Value Comparison
    //! |  4 |   [`Logic`](math) | Boolean And, Or, Xor
    //! |  5 | [`Bitwise`](math) | Bitwise And, Or, Xor
    //! |  6 |   [`Shift`](math) | Shift Left/Right
    //! |  7 |    [`Term`](math) | Add, Subtract
    //! |  8 |  [`Factor`](math) | Multiply, Divide, Remainder
    //! |  9 |   [`Unary`](math) | Unary Dereference, Reference, Negate, Not
    //! | 10 | [`control::Flow`] | Branch expressions (`if`, `while`, `for`, `return`, `break`, `continue`), `else`
    //! | 10 |         [`Group`] | Group expressions `(` [Expr]? `)` /* Can evaluate to Empty! */
    //! | 10 |         [`Block`] | Block expressions `{` [Expr] `}`
    //! | 10 |       [`Primary`] | Contains an [Identifier], [Literal](literal::Literal), [Block], [Group], or [Flow](control::Flow)
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
    /// [`Expr`] := [`math::Operation`]
    #[derive(Clone, Debug)]
    pub struct Expr {
        pub ignore: math::Operation,
    }

    /// A [Primary] Expression is the expression with the highest precedence (i.e. the deepest
    /// derivation)
    /// # Syntax
    /// [`Primary`] :=
    ///     [`IDENTIFIER`](Identifier)
    ///   | [`Literal`](literal::Literal)
    ///   | [`Block`]
    ///   | [`Group`]
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
        pub expr: Box<Expr>,
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
        //! | 1 |     Unary | `*` `&` `-` `!`                       | Right
        //! | 2 |    Factor | `*` `/` `%`                           | Left to Right
        //! | 3 |      Term | `+` `-`                               | Left to Right
        //! | 4 |     Shift | `<<` `>>`                             | Left to Right
        //! | 5 |   Bitwise | `&` <code>&#124;</code>               | Left to Right
        //! | 6 |     Logic | `&&` <code>&#124;&#124;</code> `^^`   | Left to Right
        //! | 7 |   Compare | `<` `<=` `==` `!=` `>=` `>`           | Left to Right
        #![doc = concat!( //|                                       |
        r"  | 8 |    Assign |", r"`*=`, `/=`, `%=`, `+=`, `-=`, ",//|
        /*  |   |           |*/ r"`&=`, <code>&#124;=</code>, ",  //|
        /*  |   |           |*/ r"`^=`, `<<=`, `>>=`",            r"| Right to Left")]
        //! | 9 |    Ignore | `;` |
        //!
        //! <!-- Note: '&#124;' == '|' /-->
        //!
        //! ## Syntax
        //! ```ignore
        //! /* All precedence levels other than Unary fold into Binary */
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

        /// An Operation is a tree of [operands](Primary) and [operators](operator).
        #[derive(Clone, Debug)]
        pub enum Operation {
            /// [`Binary`](Operation::Binary) :=
            /// [`Operation`] ([`operator::Binary`] [`Operation`])*
            Binary {
                first: Box<Self>,
                other: Vec<(operator::Binary, Self)>,
            },
            /// [`Unary`](Operation::Unary) := ([`operator::Unary`])* [`Primary`]
            Unary {
                operators: Vec<operator::Unary>,
                operand: Primary,
            },
        }
        impl Operation {
            pub fn binary(first: Self, other: Vec<(operator::Binary, Self)>) -> Self {
                Self::Binary { first: Box::new(first), other }
            }
        }

        pub mod operator {
            //! | # | [Operators](self)                     | Associativity
            //! |---|---------------------------------------|--------------
            //! | 0 |[`*`, `&`, `-`, `!`](Unary)            | Left to Right
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

            /// Operators which take a single argument
            ///
            /// (`*`, `&`, `-`, `!`, `@`, `#`, `~`)
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            pub enum Unary {
                /// `&&`: Take a reference, twice
                RefRef,
                /// `&`: Take a reference
                Ref,
                /// `*`: Dereference
                Deref,
                /// `-`: Arithmetic negation
                Neg,
                /// `!`: Binary/Boolean negation
                Not,
                /// `@`: Undefined
                At,
                /// `#`: Undefined
                Hash,
                /// `~`: Undefined
                Tilde,
            }
            /// Operators which take two arguments
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            pub enum Binary {
                // Term operators
                /// `*`: Multiplication
                Mul,
                /// `/`: Division
                Div,
                /// `%`: Remainder
                Rem,

                // Factor operators
                /// `+`: Addition
                Add,
                /// `-`: Subtraction
                Sub,

                // Shift operators
                /// `<<`: Left Shift
                Lsh,
                /// `>>`: Right Shift
                Rsh,

                // Bitwise operators
                /// `&`: Bitwise AND
                BitAnd,
                /// `|`: Bitwise OR
                BitOr,
                /// `^`: Bitwise XOR
                BitXor,

                // Logic operators
                /// `&&`: Short-circuiting logical AND
                LogAnd,
                /// `||`: Short-circuiting logical OR
                LogOr,
                /// `^^`: **Non-short-circuiting** logical XOR
                LogXor,

                // Range operators
                /// `..`: Exclusive range
                RangeExc,
                /// `..=`: Inclusive range
                RangeInc,

                // Comparison operators
                /// `<`: Less-than Comparison
                Less,
                /// `<=`: Less-than or Equal Comparison
                LessEq,
                /// `==`: Equal Comparison
                Equal,
                /// `!=`: Not Equal Comparison
                NotEq,
                /// `>=`: Greater-than or Equal Comparison
                GreaterEq,
                /// `>`: Greater-than Comparison
                Greater,

                // Assignment operators
                /// `=`: Assignment
                Assign,
                /// `+=`: Additive In-place Assignment
                AddAssign,
                /// `-=`: Subtractive In-place Assignment
                SubAssign,
                /// `*=`: Multiplicative In-place Assignment
                MulAssign,
                /// `/=`: Divisive In-place Assignment
                DivAssign,
                /// `%=`: Remainder In-place Assignment
                RemAssign,
                /// `&=`: Bitwise-AND In-place Assignment
                BitAndAssign,
                /// `|=`: Bitwise-OR In-place Assignment
                BitOrAssign,
                /// `^=`: Bitwise-XOR In-place Assignment
                BitXorAssign,
                /// `<<=`: Left Shift In-place Assignment
                ShlAssign,
                /// `>>=`: Right Shift In-place Assignment
                ShrAssign,
                // Ignorance operators
                /// `;`: Ignore
                Ignore,
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
