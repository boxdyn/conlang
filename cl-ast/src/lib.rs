//! # The Abstract Syntax Tree
//! Contains definitions of Conlang AST Nodes.
//!
//! # Notable nodes
//! - [Item] and [ItemKind]: Top-level constructs
//! - [Stmt] and [StmtKind]: Statements
//! - [Expr] and [ExprKind]: Expressions
//!   - [Assign], [Binary], and [Unary] expressions
//!   - [AssignKind], [BinaryKind], and [UnaryKind] operators
//! - [Ty] and [TyKind]: Type qualifiers
//! - [Path]: Path expressions
#![warn(clippy::all)]
#![feature(decl_macro)]

use cl_structures::span::*;

pub mod ast_impl;
pub mod format;

/// Whether a binding ([Static] or [Let]) or reference is mutable or not
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Mutability {
    #[default]
    Not,
    Mut,
}

/// Whether an [Item] is visible outside of the current [Module]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Visibility {
    #[default]
    Private,
    Public,
}

/// A list of [Item]s
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct File {
    pub items: Vec<Item>,
}

// Metadata decorators
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Attrs {
    pub meta: Vec<Meta>,
}

/// A metadata decorator
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Meta {
    pub name: Identifier,
    pub kind: MetaKind,
}

/// Information attached to [Meta]data
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MetaKind {
    Plain,
    Equals(Literal),
    Func(Vec<Literal>),
}

// Items
/// Anything that can appear at the top level of a [File]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Item {
    pub extents: Span,
    pub attrs: Attrs,
    pub vis: Visibility,
    pub kind: ItemKind,
}

/// What kind of [Item] is this?
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ItemKind {
    // TODO: Import declaration ("use") item
    // TODO: Trait declaration ("trait") item?
    /// A [module](Module)
    Module(Module),
    /// A [type alias](Alias)
    Alias(Alias),
    /// An [enumerated type](Enum), with a discriminant and optional data
    Enum(Enum),
    /// A [structure](Struct)
    Struct(Struct),
    /// A [constant](Const)
    Const(Const),
    /// A [static](Static) variable
    Static(Static),
    /// A [function definition](Function)
    Function(Function),
    /// An [implementation](Impl)
    Impl(Impl),
}

/// An alias to another [Ty]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Alias {
    pub to: Identifier,
    pub from: Option<Box<Ty>>,
}

/// A compile-time constant
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Const {
    pub name: Identifier,
    pub ty: Box<Ty>,
    pub init: Box<Expr>,
}

/// A `static` variable
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Static {
    pub mutable: Mutability,
    pub name: Identifier,
    pub ty: Box<Ty>,
    pub init: Box<Expr>,
}

/// An ordered collection of [Items](Item)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Module {
    pub name: Identifier,
    pub kind: ModuleKind,
}

/// The contents of a [Module], if they're in the same file
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ModuleKind {
    Inline(File),
    Outline,
}

/// Code, and the interface to that code
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Function {
    pub name: Identifier,
    pub sign: TyFn,
    pub bind: Vec<Param>,
    pub body: Option<Block>,
}

/// A single parameter for a [Function]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Param {
    pub mutability: Mutability,
    pub name: Identifier,
}

/// A user-defined product type
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Struct {
    pub name: Identifier,
    pub kind: StructKind,
}

/// Either a [Struct]'s [StructMember]s or tuple [Ty]pes, if present.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StructKind {
    Empty,
    Tuple(Vec<Ty>),
    Struct(Vec<StructMember>),
}

/// The [Visibility], [Identifier], and [Ty]pe of a single [Struct] member
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StructMember {
    pub vis: Visibility,
    pub name: Identifier,
    pub ty: Ty,
}

/// A user-defined sum type
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Enum {
    pub name: Identifier,
    pub kind: EnumKind,
}

/// An [Enum]'s [Variant]s, if it has a variant block
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EnumKind {
    /// Represents an enum with no variants
    NoVariants,
    Variants(Vec<Variant>),
}

/// A single [Enum] variant
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Variant {
    pub name: Identifier,
    pub kind: VariantKind,
}

/// Whether the [Variant] has a C-like constant value, a tuple, or [StructMember]s
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum VariantKind {
    Plain,
    CLike(u128),
    Tuple(Ty),
    Struct(Vec<StructMember>),
}

/// Sub-[items](Item) (associated functions, etc.) for a [Ty]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Impl {
    pub target: ImplKind,
    pub body: File,
}

// TODO: `impl` Trait for <Target> { }
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ImplKind {
    Type(Ty),
    Trait { impl_trait: Path, for_type: Box<Ty> },
}

/// A type expression
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ty {
    pub extents: Span,
    pub kind: TyKind,
}

/// Information about a [Ty]pe expression
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TyKind {
    Never,
    Empty,
    SelfTy,
    Path(Path),
    Tuple(TyTuple),
    Ref(TyRef),
    Fn(TyFn),
    // TODO: slice, array types
}

/// A tuple of [Ty]pes
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TyTuple {
    pub types: Vec<TyKind>,
}

/// A [Ty]pe-reference expression as (number of `&`, [Path])
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TyRef {
    pub mutable: Mutability,
    pub count: u16,
    pub to: Path,
}

/// The args and return value for a function pointer [Ty]pe
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TyFn {
    pub args: Box<TyKind>,
    pub rety: Option<Box<Ty>>,
}

/// A path to an [Item] in the [Module] tree
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Path {
    pub absolute: bool,
    pub parts: Vec<PathPart>,
}

/// A single component of a [Path]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PathPart {
    SuperKw,
    SelfKw,
    Ident(Identifier),
}

// TODO: Capture token?
/// A name
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Identifier(pub String);

/// An abstract statement, and associated metadata
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Stmt {
    pub extents: Span,
    pub kind: StmtKind,
    pub semi: Semi,
}

/// Whether or not a [Stmt] is followed by a semicolon
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Semi {
    Terminated,
    Unterminated,
}

/// Whether the [Stmt] is a [Let], [Item], or [Expr] statement
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StmtKind {
    Empty,
    Local(Let),
    Item(Box<Item>),
    Expr(Box<Expr>),
}

/// A local variable declaration [Stmt]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Let {
    pub mutable: Mutability,
    pub name: Identifier,
    pub ty: Option<Box<Ty>>,
    pub init: Option<Box<Expr>>,
}

/// An expression, the beating heart of the language
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Expr {
    pub extents: Span,
    pub kind: ExprKind,
}

/// Any of the different [Expr]essions
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExprKind {
    /// An [Assign]ment expression: [`Expr`] ([`AssignKind`] [`Expr`])\+
    Assign(Assign),
    /// A [Binary] expression: [`Expr`] ([`BinaryKind`] [`Expr`])\+
    Binary(Binary),
    /// A [Unary] expression: [`UnaryKind`]\* [`Expr`]
    Unary(Unary),
    /// An Array [Index] expression: a[10, 20, 30]
    Index(Index),
    /// A [path expression](Path): `::`? [PathPart] (`::` [PathPart])*
    Path(Path),
    /// A [Literal]: 0x42, 1e123, 2.4, "Hello"
    Literal(Literal),
    /// An [Array] literal: `[` [`Expr`] (`,` [`Expr`])\* `]`
    Array(Array),
    /// An Array literal constructed with [repeat syntax](ArrayRep)
    /// `[` [Expr] `;` [Literal] `]`
    ArrayRep(ArrayRep),
    /// An address-of expression: `&` `mut`? [`Expr`]
    AddrOf(AddrOf),
    /// A [Block] expression: `{` [`Stmt`]\* [`Expr`]? `}`
    Block(Block),
    /// An empty expression: `(` `)`
    Empty,
    /// A [Grouping](Group) expression `(` [`Expr`] `)`
    Group(Group),
    /// A [Tuple] expression: `(` [`Expr`] (`,` [`Expr`])+ `)`
    Tuple(Tuple),
    /// A [Loop] expression: `loop` [`Block`]
    Loop(Loop),
    /// A [While] expression: `while` [`Expr`] [`Block`] [`Else`]?
    While(While),
    /// An [If] expression: `if` [`Expr`] [`Block`] [`Else`]?
    If(If),
    /// A [For] expression: `for` Pattern `in` [`Expr`] [`Block`] [`Else`]?
    For(For),
    /// A [Break] expression: `break` [`Expr`]?
    Break(Break),
    /// A [Return] expression `return` [`Expr`]?
    Return(Return),
    /// A continue expression: `continue`
    Continue(Continue),
}

/// An [Assign]ment expression: [`Expr`] ([`AssignKind`] [`Expr`])\+
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Assign {
    pub kind: AssignKind,
    pub parts: Box<(ExprKind, ExprKind)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AssignKind {
    /// Standard Assignment with no read-back
    Plain,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

/// A [Binary] expression: [`Expr`] ([`BinaryKind`] [`Expr`])\+
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Binary {
    pub kind: BinaryKind,
    pub parts: Box<(ExprKind, ExprKind)>,
}

/// A [Binary] operator
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinaryKind {
    Lt,
    LtEq,
    Equal,
    NotEq,
    GtEq,
    Gt,
    RangeExc,
    RangeInc,
    LogAnd,
    LogOr,
    LogXor,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Dot,
    Call,
}

/// A [Unary] expression: [`UnaryKind`]\* [`Expr`]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Unary {
    pub kind: UnaryKind,
    pub tail: Box<ExprKind>,
}

/// A [Unary] operator
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnaryKind {
    Deref,
    Neg,
    Not,
    /// Unused
    At,
    /// Unused
    Tilde,
}
/// A repeated [Index] expression: a[10, 20, 30][40, 50, 60]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Index {
    pub head: Box<ExprKind>,
    pub indices: Vec<Expr>,
}

/// A [Literal]: 0x42, 1e123, 2.4, "Hello"
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Literal {
    Bool(bool),
    Char(char),
    Int(u128),
    String(String),
}

/// An [Array] literal: `[` [`Expr`] (`,` [`Expr`])\* `]`
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Array {
    pub values: Vec<Expr>,
}

/// An Array literal constructed with [repeat syntax](ArrayRep)
/// `[` [Expr] `;` [Literal] `]`
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArrayRep {
    pub value: Box<ExprKind>,
    pub repeat: Box<ExprKind>,
}

/// An address-of expression: `&` `mut`? [`Expr`]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AddrOf {
    pub count: usize,
    pub mutable: Mutability,
    pub expr: Box<ExprKind>,
}

/// A [Block] expression: `{` [`Stmt`]\* [`Expr`]? `}`
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

/// A [Grouping](Group) expression `(` [`Expr`] `)`
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Group {
    pub expr: Box<ExprKind>,
}

/// A [Tuple] expression: `(` [`Expr`] (`,` [`Expr`])+ `)`
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Tuple {
    pub exprs: Vec<Expr>,
}

/// A [Loop] expression: `loop` [`Block`]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Loop {
    pub body: Box<Expr>,
}

/// A [While] expression: `while` [`Expr`] [`Block`] [`Else`]?
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct While {
    pub cond: Box<Expr>,
    pub pass: Box<Block>,
    pub fail: Else,
}

/// An [If] expression: `if` [`Expr`] [`Block`] [`Else`]?
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct If {
    pub cond: Box<Expr>,
    pub pass: Box<Block>,
    pub fail: Else,
}

/// A [For] expression: `for` Pattern `in` [`Expr`] [`Block`] [`Else`]?
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct For {
    pub bind: Identifier, // TODO: Patterns?
    pub cond: Box<Expr>,
    pub pass: Box<Block>,
    pub fail: Else,
}

/// The (optional) `else` clause of a [While], [If], or [For] expression
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Else {
    pub body: Option<Box<Expr>>,
}

/// A [Break] expression: `break` [`Expr`]?
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Break {
    pub body: Option<Box<Expr>>,
}

/// A [Return] expression `return` [`Expr`]?
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Return {
    pub body: Option<Box<Expr>>,
}

/// A continue expression: `continue`
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Continue;
