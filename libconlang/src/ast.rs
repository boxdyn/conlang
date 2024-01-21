//! # The Abstract Syntax Tree
//! Contains definitions of AST Nodes, to be derived by a [parser](super::parser).
//!
//! ## Syntax
//! [`Start`]`      := `[`Program`]  \
//! [`Program`]`    := `[`statement::Stmt`]`* EOI`  \
//! [`Identifier`]` := `[`IDENTIFIER`](crate::token::token_type::Type::Identifier)
//!
//! See [statement], [literal], and [expression] for more information.
#![deprecated]
pub mod preamble {
    #![allow(deprecated)]
    //! Common imports for working with the [ast](super)
    pub use super::{
        expression::{call::*, control::*, math::*, tuple::*, *},
        literal::*,
        path::*,
        statement::*,
        types::*,
        *,
    };
}

/// Marks the root of a tree
/// # Syntax
/// [`Start`]` := `[`Program`]
#[derive(Clone, Debug)]
pub struct Start(pub Program);

/// Contains an entire Conlang program
/// # Syntax
/// [`Program`]` := `[`statement::Stmt`]`* EOI`
#[derive(Clone, Debug)]
pub struct Program(pub Vec<statement::Stmt>);

/// An Identifier stores the name of an item
/// # Syntax
/// [`Identifier`]` := `[`IDENTIFIER`](crate::token::token_type::Type::Identifier)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub name: String,
    pub index: Option<usize>,
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
    /// [`Literal`]` := `[`String`]` | `[`char`]` | `[`bool`]` | `[`Float`]` | `[`u128`]
    #[derive(Clone, Debug)]
    pub enum Literal {
        /// Represents a literal string value
        /// # Syntax
        /// [`Literal::String`]` := `[`STRING`](crate::token::token_type::Type::String)
        String(String),
        /// Represents a literal [char] value
        /// # Syntax
        /// [`Literal::Char`]` := `[`CHARACTER`](crate::token::token_type::Type::Character)
        Char(char),
        /// Represents a literal [bool] value
        /// # Syntax
        /// [`Literal::Bool`] :=
        ///     [`TRUE`](crate::token::token_type::Keyword::True)
        ///   | [`FALSE`](crate::token::token_type::Keyword::False)
        Bool(bool),
        /// Represents a literal float value
        /// # Syntax
        /// [`Float`]` := `[`FLOAT`](crate::token::token_type::Type::Float)
        Float(Float),
        /// Represents a literal integer value
        /// # Syntax
        /// [`u128`]` := `[`INTEGER`](crate::token::token_type::Type::Integer)
        Int(u128),
    }

    /// Represents a literal float value
    /// # Syntax
    /// [`Float`]` := `[`FLOAT`](crate::token::token_type::Type::Float)
    #[derive(Clone, Debug)]
    pub struct Float {
        pub sign: bool,
        pub exponent: i32,
        pub mantissa: u64,
    }
}

pub mod statement {
    //! # Statements
    //! A statement is an expression which evaluates to Empty
    //!
    //! # Syntax
    //! [`Stmt`]` := `[`Let`](Stmt::Let)` | `[`Expr`](Stmt::Expr)
    //! [`Let`](Stmt::Let)` := "let"` [`Identifier`] (`:` `Type`)? (`=` [`Expr`])? `;`
    //! [`Expr`](Stmt::Expr)` := `[`Expr`] `;`

    use super::{
        expression::{Block, Expr},
        types::TypeExpr,
        Identifier,
    };

    /// Contains a statement
    /// # Syntax
    /// [`Stmt`]` := `[`Let`](Stmt::Let)` | `[`Expr`](Stmt::Expr)
    #[derive(Clone, Debug)]
    pub enum Stmt {
        /// Contains a variable declaration
        /// # Syntax
        /// [`Let`](Stmt::Let) := `"let"` [`Identifier`] (`:` `Type`)? (`=` [`Expr`])? `;`
        Let(Let),
        /// Contains a function declaration
        /// # Syntax
        /// [`Fn`](Stmt::Fn) := `"fn"` [`Identifier`] `'('` `Args...` `')'` [`Block`]
        Fn(FnDecl),
        /// Contains a module declaration
        /// # Syntax
        /// [`Mod`](Stmt::Mod) := `"mod"` [`Identifier`] `'{'`
        ///
        /// `'}'`
        /// Contains an expression statement
        /// # Syntax
        /// [`Expr`](Stmt::Expr) := [`Expr`] `;`
        Expr(Expr),
    }

    /// Contains the declarations allowed in a module
    ///
    /// # Syntax
    /// [Mod](Module::Mod) := "mod" [Identifier] '{' [Module] '}'
    /// [`Let`](Module::Let) := `"let"` [`Identifier`] (`:` `Type`)? (`=` [`Expr`])? `;`
    #[derive(Clone, Debug)]
    pub enum Module {
        Struct(StructDecl),
        Mod(ModuleDecl),
        Let(Let),
        Fn(FnDecl),
    }

    /// Contains a variable declaration
    /// # Syntax
    /// [`Let`] := `let` [`Identifier`] (`:`) `Type`)? (`=` [`Expr`])? `;`
    #[derive(Clone, Debug)]
    pub struct Let {
        pub name: Name,
        pub init: Option<Expr>,
    }

    /// Contains a function declaration
    /// # Syntax
    /// [`FnDecl`] := `"fn"` [`Identifier`] `'('` `Args...` `')'`
    #[derive(Clone, Debug)]
    pub struct FnDecl {
        pub name: Name,
        pub args: Vec<Name>,
        pub body: Block,
        // TODO: Store type information
    }

    /// Contains the name, mutability, and type information for a [Let] or [FnDecl]
    /// # Syntax
    #[derive(Clone, Debug)]
    pub struct Name {
        pub symbol: Identifier,
        /// The mutability of the [Name]. Functions are never mutable.
        pub mutable: bool,
        /// The [type](TypeExpr)
        pub ty: Option<TypeExpr>,
    }

    /// Contains the name and declaration
    #[derive(Clone, Debug)]
    pub struct ModuleDecl {}

    // TODO: Create closure, transmute fndecl into a name and closure
    /// Contains the name and field information for a struct
    ///
    /// # Syntax
    /// [`StructDecl`]` := "struct" `[`Identifier`]` '{'
    /// (`[`Identifier`]` ':' `[`TypeExpr`]`),*
    /// '}'`
    #[derive(Clone, Debug)]
    pub struct StructDecl {
        pub name: Identifier,
        pub data: Vec<(Identifier, TypeExpr)>,
    }
}

pub mod path {
    //! Paths
    //!
    //! A Path Expression refers to an item, either local or module-scoped.

    use super::Identifier;

    /// A path to an item in a module
    /// # Syntax
    /// [`Path`]` := "::"? `[`PathPart`]` ("::" `[`PathPart`]`)*`
    #[derive(Clone, Debug)]
    pub struct Path {
        pub absolute: bool,
        pub parts: Vec<PathPart>,
    }

    /// A component of a [`Path`]
    /// # Syntax
    /// [`PathPart`]` := "super" | `[`Identifier`]
    #[derive(Clone, Debug)]
    pub enum PathPart {
        PathSuper,
        PathSelf,
        PathIdent(Identifier),
    }
}

pub mod types {
    //! # Types
    //!
    //! The [Type Expresson](TypeExpr) powers Conlang's type checker.
    //!
    //! # Syntax
    //! [`TypeExpr`]` := `[`TupleType`]` | `[`TypePath`]` | `[`Never`]

    pub use super::path::Path as TypePath;

    /// Contains a [Type Expression](self)
    ///
    /// # Syntax
    /// [`TypeExpr`]` := `[`TupleType`]` | `[`TypePath`]` | `[`Empty`]` | `[`Never`]
    #[derive(Clone, Debug)]
    pub enum TypeExpr {
        TupleType(TupleType),
        TypePath(TypePath),
        Empty(Empty),
        Never(Never),
    }

    /// A [TupleType] represents the [TypeExpr] of a Tuple value
    #[derive(Clone, Debug)]
    pub struct TupleType {
        pub types: Vec<TypeExpr>,
    }

    /// The empty type. You get nothing! You lose!
    /// # Syntax
    /// [`Empty`]` := '(' ')'`
    #[derive(Clone, Copy, Debug, Default)]
    pub struct Empty;

    /// The never type. This type can never be constructed, and can only appear if a block of code
    /// doesn't terminate
    /// # Syntax
    /// [`Never`]` := '!'`
    #[derive(Clone, Copy, Debug, Default)]
    pub struct Never;
}

pub mod expression {
    //! # Expressions
    //!
    //! The [expression] is the backbone of Conlang: almost everything is an expression.
    //!
    //! ## Grammar
    //! Higher number = higher precedence.
    //!
    //! | # |              Node | Function                                      
    //! |---|------------------:|:----------------------------------------------
    //! | 0 |          [`Expr`] | Contains an expression
    //! | 1 |  [`Assign`](math) | Assignment
    //! | 2 | [`Compare`](math) | Value Comparison
    //! | 3 |   [`Logic`](math) | Boolean And, Or, Xor
    //! | 4 | [`Bitwise`](math) | Bitwise And, Or, Xor
    //! | 5 |   [`Shift`](math) | Shift Left/Right
    //! | 6 |    [`Term`](math) | Add, Subtract
    //! | 7 |  [`Factor`](math) | Multiply, Divide, Remainder
    //! | 8 |   [`Unary`](math) | Unary Dereference, Reference, Negate, Not
    //! | 9 | [`control::Flow`] | Branch expressions (`if`, `while`, `for`, `return`, `break`, `continue`)
    //! | 9 |         [`Group`] | Group expressions `(` [Expr]? `)` /* Can evaluate to Empty! */
    //! | 9 |         [`Block`] | Block expressions `{` [Expr] `}`
    //! | 9 |       [`Primary`] | Contains an [Identifier], [Literal], [Block], [Group], or [Flow]
    //!
    //! ## Syntax
    //! [`Expr`]`  := `[`math::Operation`]  \
    //! [`Block`]` := '{' `[`Expr`]` '}'`   \
    //! [`Group`]` := '(' `[`Expr`]`? ')'`  \
    //! [`Primary`]` := `[`Identifier`]` | `[`Literal`]` | `[`Block`]` |
    //! `[`Group`]` | `[`Flow`]
    //!
    //! See [control] and [math] for their respective production rules.
    use super::{literal::Literal, statement::Stmt, *};
    use control::Flow;
    use tuple::Group;

    /// Contains an expression
    ///
    /// # Syntax
    /// [`Expr`]` := `[`math::Operation`]
    #[derive(Clone, Debug)]
    pub struct Expr(pub math::Operation);

    /// A [Primary] Expression is the expression with the highest precedence (i.e. the deepest
    /// derivation)
    /// # Syntax
    /// [`Primary`]` := `[`Identifier`]`
    /// | `[`Literal`]`
    /// | `[`Block`]`
    /// | `[`Group`]`
    /// | `[`Branch`](Flow)
    #[derive(Clone, Debug)]
    pub enum Primary {
        Identifier(Identifier),
        Literal(Literal),
        Block(Block),
        Group(Group),
        Branch(Flow),
    }

    /// Contains a Block Expression
    /// # Syntax
    /// [`Block`] := `'{'` [`Expr`] `'}'`
    #[derive(Clone, Debug)]
    pub struct Block {
        pub let_count: Option<usize>,
        pub statements: Vec<Stmt>,
        pub expr: Option<Box<Expr>>,
    }

    pub mod call {
        //! [Function](FnCall) and [Method](todo) Call Expressions
        //!
        //! # Syntax
        //! [`Call`]` := `[`FnCall`]` | `[`Primary`]
        //!
        //! [`FnCall`]` := `[`Primary`]` (`[`Tuple`]`)*`
        use super::{tuple::Tuple, Primary};
        #[derive(Clone, Debug)]
        pub enum Call {
            /// Contains a [Function Call Expression](FnCall)
            FnCall(FnCall),
            /// Contains only a [Primary] expression
            Primary(Primary),
        }

        /// Contains a Function Call Expression
        #[derive(Clone, Debug)]
        pub struct FnCall {
            pub callee: Box<Primary>,
            pub args: Vec<Tuple>,
        }
        #[allow(non_snake_case)]
        pub fn FnCall(callee: Box<Primary>, args: Vec<Tuple>) -> FnCall {
            FnCall { callee, args }
        }
    }

    pub mod tuple {
        //! A [Tuple] expression contains an arbitrary number of sub-expressions
        use super::Expr;
        /// Contains a [Tuple], [`(Expr)`](Group::Single),
        /// or [Empty](Group::Empty)
        /// # Syntax
        /// [`Group`]`:= '('(`[`Expr`]` | `[`Tuple`]`)?')'`
        #[derive(Clone, Debug)]
        pub enum Group {
            /// Contains a variety of elements
            /// # Syntax
            /// [`Group::Tuple`]`:= '('`[`Tuple`]`')'`
            Tuple(Tuple),
            /// Contains a single element
            /// # Syntax
            /// [`Group::Single`]`:= '('`[`Expr`]`')'`
            Single(Box<Expr>),
            /// Contains no elements
            /// # Syntax
            /// [`Group::Empty`]`:= '(' ')'`
            Empty,
        }

        /// Contains a heterogeneous collection of sub-expressions
        /// # Syntax
        #[derive(Clone, Debug)]
        pub struct Tuple {
            pub elements: Vec<Expr>,
        }
    }

    pub mod math {
        //! # Arithmetic and Logical Expressions
        //!
        //! ## Precedence Order
        //! Operator associativity is always left-to-right among members of the same group
        //!
        //! | # |       Name | Operators                               | Associativity
        //! |---|-----------:|:----------------------------------------|---------------
        //  |   |  TODO: Try | `?`                                     |
        //! | 1 |  [Unary][1]| [`*` `&` `-` `!`][4]                    | Right
        //! | 2 | [Factor][2]| [`*` `/` `%`][5]                        | Left to Right
        //! | 3 |   [Term][2]| [`+` `-`][5]                            | Left to Right
        //! | 4 |  [Shift][2]| [`<<` `>>`][5]                          | Left to Right
        //! | 5 |[Bitwise][2]| [`&` <code>&#124;</code>][4]            | Left to Right
        //! | 6 |  [Logic][2]| [`&&` <code>&#124;&#124;</code> `^^`][5]| Left to Right
        //! | 7 |[Compare][2]| [`<` `<=` `==` `!=` `>=` `>`][5]        | Left to Right
        #![doc = concat!(  //|                                         |
        r"  | 8 | [Assign][3]| [`*=`, `/=`, `%=`, `+=`, `-=`, ",     //|
        /*  |   |            |*/ r"`&=`, <code>&#124;=</code>, ",    //|
        /*  |   |            |*/ r"`^=`, `<<=`, `>>=`][6]",          r"| Left to Right")]
        //!
        //! <!-- Note: '&#124;' == '|' /-->
        //!
        //! ## Syntax
        //! All precedence levels other than [Unary][1] fold into [Binary][2]
        //!
        //! [`Assign`][3]`  := `[`Compare`][2]` (`[`AssignOp`][6]`  `[`Compare`][2]`)*`  \
        //! [`Compare`][2]` := `[`Logic`][2]`   (`[`CompareOp`][5]` `[`Logic`][2]`  )*`  \
        //! [`Logic`][2]`   := `[`Bitwise`][2]` (`[`LogicOp`][5]`   `[`Bitwise`][2]`)*`  \
        //! [`Bitwise`][2]` := `[`Shift`][2]`   (`[`BitwiseOp`][5]` `[`Shift`][2]`  )*`  \
        //! [`Shift`][2]`   := `[`Term`][2]`    (`[`ShiftOp`][5]`   `[`Term`][2]`   )*`  \
        //! [`Term`][2]`    := `[`Factor`][2]`  (`[`TermOp`][5]`    `[`Factor`][2]` )*`  \
        //! [`Factor`][2]`  := `[`Unary`][1]`   (`[`FactorOp`][5]`  `[`Unary`][1]`  )*`  \
        //! [`Unary`][1]`   := (`[`UnaryOp`][4]`)* `[`Call`]
        //!
        //! [1]: Operation::Unary
        //! [2]: Operation::Binary
        //! [3]: Operation::Assign
        //! [4]: operator::Unary
        //! [5]: operator::Binary
        //! [6]: operator::Assign
        use super::{call::Call, *};

        /// An Operation is a tree of [operands](Primary) and [operators](operator).
        #[derive(Clone, Debug)]
        pub enum Operation {
            /// [`Assign`](Operation::Assign) :=
            /// [`Identifier`] [`operator::Assign`] [`Operation`] | [`Operation`]
            Assign(Assign),
            /// [`Binary`](Operation::Binary) :=
            /// [`Operation`] ([`operator::Binary`] [`Operation`])*
            Binary(Binary),
            /// [`Unary`](Operation::Unary) := ([`operator::Unary`])*
            /// [`Call`](Operation::Call)
            Unary(Unary),
            /// [`Call`](Operation::Call) := [`expression::call::Call`]
            Call(Call),
        }

        /// [`Assign`] := [`Identifier`] [`operator::Assign`] [`Operation`] | [`Operation`]
        #[derive(Clone, Debug)]
        pub struct Assign {
            pub target: Identifier,
            pub operator: operator::Assign,
            pub init: Box<Operation>,
        }

        /// [`Binary`] := [`Operation`] ([`operator::Binary`] [`Operation`])*
        #[derive(Clone, Debug)]
        pub struct Binary {
            pub first: Box<Operation>,
            pub other: Vec<(operator::Binary, Operation)>,
        }

        /// [`Unary`] := ([`operator::Unary`])* [`Call`](Operation::Call)
        #[derive(Clone, Debug)]
        pub struct Unary {
            pub operators: Vec<operator::Unary>,
            pub operand: Box<Operation>,
        }

        pub mod operator {
            //! # [Unary], [Binary], and [Assign] operators
            //!
            //! An Operator represents the action taken during an [operation](super::Operation)

            /// # Operators which take a single argument
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
            /// # Operators which take two arguments
            /// ## Term operators:
            /// [`*`](Binary::Mul), [`/`](Binary::Div), [`%`](Binary::Rem)
            /// ## Factor operators
            /// [`+`](Binary::Add), [`-`](Binary::Sub)
            /// ## Shift operators
            /// [`<<`](Binary::Lsh), [`>>`](Binary::Rsh)
            /// ## Bitwise operators
            /// [`&`](Binary::BitAnd), [`|`](Binary::BitOr), [`^`](Binary::BitXor)
            /// ## Logic operators
            /// [`&&`](Binary::LogAnd), [`||`](Binary::LogOr), [`^^`](Binary::LogXor)
            /// ## Range operators
            /// [`..`](Binary::RangeExc), [`..=`](Binary::RangeInc)
            /// ## Comparison operators
            /// [`<`](Binary::Less), [`<=`](Binary::LessEq), [`==`](Binary::Equal),
            /// [`!=`](Binary::NotEq), [`>=`](Binary::GreaterEq), [`>`](Binary::Greater),
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            pub enum Binary {
                /// `*`: Multiplication
                Mul,
                /// `/`: Division
                Div,
                /// `%`: Remainder
                Rem,
                /// `+`: Addition
                Add,
                /// `-`: Subtraction
                Sub,
                /// `<<`: Left Shift
                Lsh,
                /// `>>`: Right Shift
                Rsh,
                /// `&`: Bitwise AND
                BitAnd,
                /// `|`: Bitwise OR
                BitOr,
                /// `^`: Bitwise XOR
                BitXor,
                /// `&&`: Short-circuiting logical AND
                LogAnd,
                /// `||`: Short-circuiting logical OR
                LogOr,
                /// `^^`: **Non-short-circuiting** logical XOR
                LogXor,
                /// `..`: Exclusive range
                RangeExc,
                /// `..=`: Inclusive range
                RangeInc,
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
            }

            /// # Assignment operators
            /// [`=`](Assign::Assign), [`+=`](Assign::AddAssign), [`-=`](Assign::SubAssign),
            /// [`*=`](Assign::MulAssign), [`/=`](Assign::DivAssign), [`%=`](Assign::RemAssign),
            /// [`&=`](Assign::BitAndAssign), [`|=`](Assign::BitOrAssign),
            /// [`^=`](Assign::BitXorAssign) [`<<=`](Assign::ShlAssign),
            /// [`>>=`](Assign::ShrAssign)
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            pub enum Assign {
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
        //! [`Flow`]`  := `[`While`]` | `[`If`]` | `[`For`]` |
        //! `[`Break`]` | `[`Return`]` | `[`Continue`]  \
        //!
        //! [`While`]` := "while" `[`Expr`]` `[`Block`]` `[`Else`]`?`  \
        //! [`If`]`    := "if"    `[`Expr`]` `[`Block`]` `[`Else`]`?`  \
        //! [`For`]`   := "for"   `[`Identifier`]` "in" `[`Expr`]` `[`Block`]` `[`Else`]`?`  \
        //! [`Else`]`  := "else"  `[`Expr`]  \
        //!
        //! [`Break`]`    := "break" `[`Expr`]  \
        //! [`Return`]`   := "return" `[`Expr`]  \
        //! [`Continue`]` := "continue"`
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
        /// An [`else` expression](Else) contains instructions to be executed if
        /// the corresponding body refused to produce a value. In the case of
        /// [`if` expressions](If), this happens if the condition fails.
        /// In the case of loop ([`while`](While), [`for`](For))expressions,
        /// this executes when the loop does *not* [`break`](Break).
        ///
        /// If one of the aforementioned control flow expressions evaluates
        /// to something other than the Empty type, this block is mandatory.
        ///
        /// # Syntax
        /// [`Else`] := `"else"` [`Expr`]
        #[derive(Clone, Debug)]
        pub struct Else {
            pub expr: Box<Expr>,
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

pub mod todo {
    //! temporary storage for pending expression work.  \
    //! when an item is in progress, remove it from todo.
    //!
    //! # General TODOs:
    //! - [x] REMOVE VISITOR TRAIT
    //! - [x] Implement support for storing items in the AST
    //! - [ ] Keep track of the source location of each node
    //! - [ ] Implement paths
    //! - [x] Implement functions
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

    pub mod module {
        //! Module support
        //! - [ ] Add Module Declaration type : ModDecl = "mod" Identifier '{' Module '}' ;
        //! - [ ] Change Program to Module : Module = (ModDecl | FnDecl | Let)*
        //!   - [ ] Implementer's note: Modules must be traversed breadth-first, with no
        //!     alpha-renaming
        //!   - [ ] Blocks should probably also be traversed breadth-first, and Let declarations
        //!     hoisted up, leaving initialization assignments in-place
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
