//! The Abstract Syntax Tree defines an interface between the parser and type checker

use std::hash::Hash;

mod display;

pub mod fold;
pub mod macro_matcher;
pub mod types;
pub mod visit;

pub use types::DefaultTypes;

/// An annotation: bounds on AST parameters
pub trait Annotation: Clone + std::fmt::Display + std::fmt::Debug + PartialEq + Eq {}

impl<T: Clone + std::fmt::Debug + std::fmt::Display + PartialEq + Eq> Annotation for T {}

pub trait AstTypes: Annotation {
    /// An annotation on an arbitrary [Expr]
    type Annotation: Annotation;

    /// A literal value
    type Literal: Annotation;

    /// A (possibly interned) symbol or index which implements [`AsRef<str>`]
    type MacroId: Annotation + Hash + AsRef<str>;

    /// A (possibly interned) symbol or index
    type Symbol: Annotation + Copy + Hash;

    /// A (possibly compound) symbol or index
    type Path: Annotation;
}

/// A value with an annotation.
#[derive(Clone, PartialEq, Eq)]
pub struct At<T: Annotation, A: AstTypes = DefaultTypes>(pub T, pub A::Annotation);

/// Expressions: The beating heart of Conlang.
///
/// A program in Conlang is a single expression which, at compile time,
/// sets up the state in which a program will run. This expression binds types,
/// functions, and values to names which are exposed at runtime.
///
/// Whereas in the body of a function, `do` sequences are ordered, in the global
/// scope (or subsequent module scopes, which are children of the global module,)
/// `do` sequences are considered unordered, and subexpressions may be reordered
/// in whichever way the compiler sees fit. This is especially important when
/// performing import resolution, as imports typically depend on the order
/// in which names are bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr<A: AstTypes = DefaultTypes> {
    /// Omitted by semicolon insertion-elision rules
    Omitted,
    /// An identifier
    Id(A::Path),
    /// An escaped token for macro binding
    MetId(A::MacroId),
    /// A literal bool, string, char, or int
    Lit(A::Literal),
    /// use Use
    Use(Use<A>),
    /// `let Pat::NoTopAlt (= expr (else expr)?)?` |
    /// `(fn | mod | impl) Pat::Fn Expr`
    Bind(Box<Bind<A>>),
    /// Expr { (Ident (: Expr)?),* }
    Make(Box<Make<A>>),
    /// Op Expr | Expr Op | Expr (Op Expr)+ | Op Expr Expr else Expr
    Op(Op, Vec<At<Self, A>>),
}

/// Conlang's AST is partitioned by data representation, so it
/// considers any expression which is composed solely of keywords,
/// symbols, and other expressions as operator expressions.
///
/// This includes:
/// - Do-sequence expressions: `Expr ; Expr `
/// - Type-cast expressions `Expr as Expr`
/// - Binding-modifier expressions: `pub Expr`, `#[Expr] Expr`
/// - Block and Group expressions: `{Expr?}`, `(Expr?)`
/// - Control flow: `if`, `while`, `loop`, `match`, `break`, `return`
/// - Function calls `Expr (Expr,*)`
/// - Traditional binary and unary operators (add, sub, neg, assign)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// `Expr (; Expr)*`
    Do,
    /// `Expr as Expr`
    As,
    /// `{ Expr }`
    Block,
    /// `[ Expr,* ]`
    Array,
    /// `[ Expr ; Expr ]`
    ArRep,
    /// `( Expr )`
    Group,
    /// `Expr (, Expr)*`
    Tuple,
    /// `#![ Expr ]`
    MetaInner,
    /// `#[ Expr ]`
    MetaOuter,

    /// `Expr '?'`
    Try,
    /// `Expr [ Expr ]`
    Index,
    /// `Expr ( Expr )`
    Call,

    /// `pub Expr`
    Pub,
    /// `const Expr`
    Const,
    /// `static Expr`
    Static,
    /// `macro Expr`
    Macro,
    /// `loop Expr`
    Loop,
    /// `match Expr { <Bind(Match, ..)>,* }`
    Match,
    /// `if Expr Expr (else Expr)?`
    If,
    /// `while Expr Expr (else Expr)?`
    While,
    /// `break Expr`
    Break,
    /// `return Expr`
    Return,
    /// `continue`
    Continue,

    /// `Expr . Expr`
    Dot,

    /// `Expr? ..Expr`
    RangeEx,
    /// `Expr? ..=Expr`
    RangeIn,
    /// `-Expr`
    Neg,
    /// `!Expr`
    Not,
    /// `!!Expr`
    Identity,
    /// `&Expr`
    Refer,
    /// `*Expr`
    Deref,

    /// `Expr * Expr`
    Mul,
    /// `Expr / Expr`
    Div,
    /// `Expr % Expr`
    Rem,

    /// `Expr + Expr`
    Add,
    /// `Expr - Expr`
    Sub,

    /// `Expr << Expr`
    Shl,
    /// `Expr >> Expr`
    Shr,

    /// `Expr & Expr`
    And,
    /// `Expr ^ Expr`
    Xor,
    /// `Expr | Expr`
    Or,

    /// `Expr < Expr`
    Lt,
    /// `Expr <= Expr`
    Leq,
    /// `Expr == Expr`
    Eq,
    /// `Expr != Expr`
    Neq,
    /// `Expr >= Expr`
    Geq,
    /// `Expr > Expr`
    Gt,

    /// `Expr && Expr`
    LogAnd,
    /// `Expr ^^ Expr`
    LogXor,
    /// `Expr || Expr`
    LogOr,

    /// `Expr = Expr`
    Set,
    /// `Expr *= Expr`
    MulSet,
    /// `Expr /= Expr`
    DivSet,
    /// `Expr %= Expr`
    RemSet,
    /// `Expr += Expr`
    AddSet,
    /// `Expr -= Expr`
    SubSet,
    /// `Expr <<= Expr`
    ShlSet,
    /// `Expr >>= Expr`
    ShrSet,
    /// `Expr &= Expr`
    AndSet,
    /// `Expr ^= Expr`
    XorSet,
    /// `Expr |= Expr`
    OrSet,
}

impl<A: AstTypes> Expr<A> {
    pub const fn at(self, annotation: A::Annotation) -> At<Expr<A>, A> {
        At(self, annotation)
    }

    pub fn and_do(self, annotation: A::Annotation, other: At<Expr<A>, A>) -> Self {
        let Self::Op(Op::Do, mut exprs) = self else {
            return Self::Op(Op::Do, vec![self.at(annotation), other]);
        };
        let At(Self::Op(Op::Do, mut other), _) = other else {
            exprs.push(other);
            return Self::Op(Op::Do, exprs);
        };
        exprs.append(&mut other);
        Self::Op(Op::Do, exprs)
    }

    pub fn to_tuple(self, annotation: A::Annotation) -> Self {
        match self {
            Self::Op(Op::Tuple, _) => self,
            _ => Self::Op(Op::Tuple, vec![self.at(annotation)]),
        }
    }

    pub const fn is_place(&self) -> bool {
        matches!(
            self,
            Self::Id(_) | Self::Op(Op::Index | Op::Dot | Op::Deref, _)
        )
    }

    pub const fn is_value(&self) -> bool {
        !self.is_place()
    }

    #[allow(clippy::type_complexity)]
    pub const fn as_slice(&self) -> Option<(Op, &[At<Expr<A>, A>])> {
        match self {
            Expr::Op(op, args) => Some((*op, args.as_slice())),
            _ => None,
        }
    }
}

/// A pattern binding
/// ```ignore
/// let    Pat (= Expr (else Expr)?)?
/// type   Pat (= Expr)?
/// fn     Pat =? Expr
/// mod    Pat =? Expr
/// impl   Pat =? Expr
/// struct Pat
/// enum   Pat
/// for    Pat in Expr Expr (else Expr)?
/// Pat => Expr // in match
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bind<A: AstTypes = DefaultTypes>(
    pub BindOp,
    pub Vec<A::Path>,
    pub Pat<A>,
    pub Vec<At<Expr<A>, A>>,
);

/// The binding operation used by a [Bind].
///
/// See [Bind] for their syntactic representations
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindOp {
    /// A `let Pat (= Expr (else Expr)?)?` binding
    Let,
    /// A type-alias binding
    Type,
    /// A `fn Pat Expr` binding
    Fn,
    /// A `mod Pat Expr` binding
    Mod,
    /// An `impl Pat Expr` binding
    Impl,
    /// A struct definition
    Struct,
    /// An enum definition
    Enum,
    /// A `for Pat in Expr Expr (else Expr)?` binding
    For,
    /// A `Pat => Expr` binding
    Match,
}

/// Binding patterns for each kind of matchable value.
///
/// This covers both bindings and type annotations in [Bind] expressions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pat<A: AstTypes = DefaultTypes> {
    /// `_`: Matches anything without binding
    Ignore,
    /// `!`: Matches nothing, ever
    Never,
    /// `$Token`: Matches nothing; used for macro substitution
    MetId(A::MacroId),
    /// `Identifier`: Matches anything, and binds it to a name
    Name(A::Symbol),
    /// `Expr`: Matches a value by equality comparison
    Value(Box<At<Expr<A>, A>>),
    /// Matches a compound pattern
    Op(PatOp, Vec<Pat<A>>),
}

/// Operators on lists of patterns
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatOp {
    /// Changes the visibility mode to "public"
    Pub,
    /// Changes the binding mode to "mutable"
    Mut,
    /// Matches the dereference of a pointer (`&pat`)
    Ref,
    /// Matches the dereference of a raw pointer (`*pat`)
    Ptr,
    /// Matches a partial decomposition (`..rest`) or upper-bounded range (`..100`)
    Rest,
    /// Matches an exclusive bounded range (`0..100`)
    RangeEx,
    /// Matches an inclusive bounded range (`0..=100`)
    RangeIn,
    /// Matches the elements of a record or struct { a, b, c }
    Record,
    /// Matches the elements of a tuple ( a, b, c )
    Tuple,
    /// Matches the elements of a slice or array [ a, b, c ]
    Slice,
    /// Matches a constant-size slice with repeating elements
    ArRep,
    /// Matches a type annotation or struct member
    Typed,
    /// Matches a prefix-type-annotated structure
    TypePrefixed,
    /// Matches a generic specialization annotation
    Generic,
    /// Changes the binding mode to "function-body"
    Fn,
    /// Matches one of a list of alternatives
    Alt,
}

impl<A: AstTypes> Pat<A> {
    pub fn to_tuple(self) -> Self {
        match self {
            Self::Op(PatOp::Tuple, _) => self,
            _ => Self::Op(PatOp::Tuple, vec![self]),
        }
    }
}

/// A compound import declaration
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Use<A: AstTypes = DefaultTypes> {
    /// "*"
    Glob,
    /// Identifier
    Name(A::Symbol),
    /// Identifier as Identifier
    Alias(A::Symbol, A::Symbol),
    /// Identifier :: Use
    Path(A::Symbol, Box<Use<A>>),
    /// { Use, * }
    Tree(Vec<Use<A>>),
}

/// A make (constructor) expression
/// ```ignore
/// Expr { (Ident (: Expr)?),* }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Make<A: AstTypes = DefaultTypes>(pub At<Expr<A>, A>, pub Vec<MakeArm<A>>);

/// A single "arm" of a make expression
/// ```text
/// Identifier (':' Expr)?
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MakeArm<A: AstTypes = DefaultTypes>(pub A::Symbol, pub Option<At<Expr<A>, A>>);
