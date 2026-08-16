//! The Abstract Syntax Tree defines an interface between the parser and type checker

use std::hash::Hash;

mod display;

pub mod fold;
pub mod macro_matcher;
pub mod types;
pub mod visit;

pub use types::DefaultTypes;

/// Common bounds on AST nodes
pub trait AstNode: Clone + std::fmt::Display + std::fmt::Debug + PartialEq + Eq + Hash {}

impl<T: Clone + std::fmt::Debug + std::fmt::Display + PartialEq + Eq + Hash> AstNode for T {}

/// The replaceable types within major AST nodes
pub trait AstTypes: Clone + std::fmt::Debug + PartialEq + Eq + Hash {
    /// An annotation on an arbitrary [Expr] or [Pat]
    type Annotation: AstNode + Copy;

    /// A literal value
    type Literal: AstNode;

    /// A (possibly interned) symbol or index which implements [`AsRef<str>`]
    type MacroId: AstNode + Hash + AsRef<str>;

    /// A (possibly interned) symbol or index
    type Symbol: AstNode + Copy + Hash;

    /// A (possibly compound) symbol or index
    type Path: AstNode;
}

/// A value with an annotation.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct At<T: AstNode, A: AstTypes = DefaultTypes>(pub T, pub A::Annotation);

impl<T: AstNode, A: AstTypes> At<T, A> {
    pub const fn value(&self) -> &T {
        &self.0
    }
    pub const fn a(&self) -> &A::Annotation {
        &self.1
    }
    pub fn map<U: AstNode>(self, f: impl FnOnce(T) -> U) -> At<U, A> {
        At(f(self.0), self.1)
    }
    pub fn map_ref<U: AstNode>(&self, f: impl FnOnce(&T) -> U) -> At<U, A> {
        At(f(&self.0), self.1)
    }
    pub fn map_a<B: AstTypes>(self, f: impl FnOnce(A::Annotation) -> B::Annotation) -> At<T, B> {
        At(self.0, f(self.1))
    }
}

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
#[derive(Clone, PartialEq, Eq, Hash)]
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
    /// `match Expr { (Pat => Expr),* }`
    Match(Box<Match<A>>),
    /// `'label Expr`
    Label(Box<Label<A>>),
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    /// <code>`Expr`</code>
    Quote,
    /// `loop Expr`
    Loop,
    /// `if Expr Expr (else Expr)?`
    If,
    /// `while Expr Expr (else Expr)?`
    While,
    /// `defer Expr`
    Defer,
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
    /// Attaches this [Expr] to an [At] node with the provided [AstTypes::Annotation].
    pub const fn at(self, annotation: A::Annotation) -> At<Expr<A>, A> {
        At(self, annotation)
    }

    /// Attaches another expression to this one, continuing a [`do`](Op::Do) chain if possible.
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

    /// Removes omitted expressions from positions where expressions are allowed to be omitted.
    pub fn deomit(self) -> Self {
        let Self::Op(op @ (Op::Do | Op::Tuple | Op::Array), mut exprs) = self else {
            return self;
        };
        exprs.retain(|expr| !matches!(expr.0, Self::Omitted));
        Self::Op(op, exprs)
    }

    /// Turns this expression into a [`tuple`](Op::Tuple) if it isn't already one.
    pub fn to_tuple(self, annotation: A::Annotation) -> Self {
        match self {
            Self::Op(Op::Tuple, _) => self,
            _ => Self::Op(Op::Tuple, vec![self.at(annotation)]),
        }
    }

    /// Returns whether `self` is a "place projection" expression (identifier, index, dot, or deref)
    pub const fn is_place(&self) -> bool {
        matches!(
            self,
            Self::Id(_) | Self::Op(Op::Index | Op::Dot | Op::Deref, _)
        )
    }

    /// Returns whether `self` is NOT a "place projection" expression
    pub const fn is_value(&self) -> bool {
        !self.is_place()
    }

    /// If `self` is an [Expr::Op], returns the [Op] and a slice of its arguments.
    #[allow(clippy::type_complexity)]
    pub const fn as_slice(&self) -> Option<(Op, &[At<Expr<A>, A>])> {
        match self {
            Expr::Op(op, args) => Some((*op, args.as_slice())),
            _ => None,
        }
    }
}

/// A labeled expression
///
/// This creates a jump target which `break` can break to:
/// ```ignore
/// let seven = 'label
///     for x in 1..100 {
///         if x % 10 == 7
///             break 'label x;
///     } else 7
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Label<A: AstTypes = DefaultTypes>(pub A::Symbol, pub At<Expr<A>, A>);

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
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Bind<A: AstTypes = DefaultTypes>(pub BindOp, pub At<Pat<A>, A>, pub Vec<At<Expr<A>, A>>);

/// The binding operation used by a [Bind].
///
/// See [Bind] for their syntactic representations
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
}

/// Binding patterns for each kind of matchable value.
///
/// This covers both bindings and type annotations in [Bind] expressions.
#[derive(Clone, PartialEq, Eq, Hash)]
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
    Op(PatOp, Vec<At<Pat<A>, A>>),
}

/// Operators on lists of patterns
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PatOp {
    /// `#![ .* ] Pat`
    MetaInner,
    /// `#[ .* ] Pat`
    MetaOuter,
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
    /// Matches a struct member or types a value: `x: T`
    Typed,
    /// Types a prefix-annotated structure `T(..)`, `R{..}`
    TypePrefixed,
    /// Types a prefix generic annotation `<T>U`
    PrefixGeneric,
    /// Types a postfix generic annotation `T<U>`
    PostfixGeneric,
    /// Types a function signature
    Fn,
    /// Matches a guard pattern (`Pat if Expr`)
    Guard,
    /// Matches one of a list of alternatives
    Alt,
}

impl<A: AstTypes> Pat<A> {
    /// Attaches this [Pat] to an [At] node with the provided [AstTypes::Annotation].
    pub const fn at(self, annotation: A::Annotation) -> At<Pat<A>, A> {
        At(self, annotation)
    }
    /// Turns this pattern into a [`tuple`](PatOp::Tuple) if it isn't already one.
    pub fn to_tuple(self, annotation: A::Annotation) -> Self {
        match self {
            Self::Op(PatOp::Tuple, _) => self,
            _ => Self::Op(PatOp::Tuple, vec![self.at(annotation)]),
        }
    }

    /// Gets the closest non-[PostfixGeneric], [PrefixGeneric], or [TypePrefixed] [Pat]
    ///
    /// [TypePrefixed]: PatOp::TypePrefixed
    /// [PrefixGeneric]: PatOp::PrefixGeneric
    /// [PostfixGeneric]: PatOp::PostfixGeneric
    pub fn inner(&self) -> &Self {
        match self {
            Pat::Op(PatOp::PostfixGeneric | PatOp::PrefixGeneric | PatOp::TypePrefixed, ats)
                if let [.., last] = &ats[..] =>
            {
                last.value().inner()
            }
            _ => self,
        }
    }

    /// Gets the "generics" pattern for a [PostfixGeneric], [PrefixGeneric], or [TypePrefixed] [Pat]
    ///
    /// [TypePrefixed]: PatOp::TypePrefixed
    /// [PrefixGeneric]: PatOp::PrefixGeneric
    /// [PostfixGeneric]: PatOp::PostfixGeneric
    pub fn generics(&self) -> Option<&Self> {
        Some(match self {
            Pat::Op(PatOp::PrefixGeneric, ats) if let [first, ..] = &ats[..] => first.value(),
            Pat::Op(PatOp::PostfixGeneric, ats) if let [.., last] = &ats[..] => last.value(),
            Pat::Op(PatOp::TypePrefixed, ats) if let [.., last] = &ats[..] => {
                last.value().generics()?
            }
            _ => None?,
        })
    }

    /// Returns the single, unambiguous name bound by this pattern, if there is one.
    ///
    /// Else, returns [None].
    pub fn name(&self) -> Option<A::Symbol> {
        match self {
            Self::Name(name) => Some(*name),
            Self::Op(
                PatOp::TypePrefixed
                | PatOp::Typed
                | PatOp::Pub
                | PatOp::Mut
                | PatOp::PostfixGeneric
                | PatOp::Guard,
                pats,
            ) if let [At(pat, _), ..] = &pats[..] => pat.name(),
            _ => None,
        }
    }
}

/// A compound import declaration
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Make<A: AstTypes = DefaultTypes>(pub At<Expr<A>, A>, pub Vec<MakeArm<A>>);

/// A single "arm" of a make expression
/// ```text
/// Identifier (':' Expr)?
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MakeArm<A: AstTypes = DefaultTypes>(pub A::Symbol, pub Option<At<Expr<A>, A>>);

/// A match expression has a scrutinee and zero or more arms
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Match<A: AstTypes = DefaultTypes>(pub At<Expr<A>, A>, pub Vec<MatchArm<A>>);

/// A single arm of a `match` expression
/// ```text
/// Pat => Expr
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MatchArm<A: AstTypes = DefaultTypes>(pub At<Pat<A>, A>, pub At<Expr<A>, A>);
