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
#![feature(decl_macro)]

use cl_structures::span::*;

pub mod ast_impl;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mutability {
    #[default]
    Not,
    Mut,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Visibility {
    #[default]
    Private,
    Public,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct File {
    pub items: Vec<Item>,
}

// Metadata decorators
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attrs {
    pub meta: Vec<Meta>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Meta {
    pub name: Identifier,
    pub kind: MetaKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetaKind {
    Plain,
    Equals(Literal),
    Func(Vec<Literal>),
}

// Items
/// Stores an [ItemKind] and associated metadata
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub extents: Span,
    pub attrs: Attrs,
    pub vis: Visibility,
    pub kind: ItemKind,
}

/// Stores a concrete Item
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemKind {
    // TODO: Import declaration ("use") item
    // TODO: Trait declaration ("trait") item?
    /// A [type alias](Alias)
    Alias(Alias),
    /// A [constant](Const)
    Const(Const),
    /// A [static](Static) variable
    Static(Static),
    /// A [module](Module)
    Module(Module),
    /// A [function definition](Function)
    Function(Function),
    /// A [structure](Struct)
    Struct(Struct),
    /// An [enumerated type](Enum)
    Enum(Enum),
    /// An [implementation](Impl)
    Impl(Impl),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alias {
    pub to: Box<Ty>,
    pub from: Option<Box<Ty>>,
}

/// Stores a `const` value
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Const {
    pub name: Identifier,
    pub ty: Box<Ty>,
    pub init: Box<Expr>,
}

/// Stores a `static` variable
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Static {
    pub mutable: Mutability,
    pub name: Identifier,
    pub ty: Box<Ty>,
    pub init: Box<Expr>,
}

/// Stores a collection of [Items](Item)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    pub name: Identifier,
    pub kind: ModuleKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModuleKind {
    Inline(File),
    Outline,
}

/// Contains code, and the interface to that code
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    pub name: Identifier,
    pub args: Vec<Param>,
    pub body: Option<Block>,
    pub rety: Option<Box<Ty>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    pub mutability: Mutability,
    pub name: Identifier,
    pub ty: Box<Ty>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Struct {
    pub name: Identifier,
    pub kind: StructKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StructKind {
    Empty,
    Tuple(Vec<Ty>),
    Struct(Vec<StructMember>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructMember {
    pub vis: Visibility,
    pub name: Identifier,
    pub ty: Ty,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Enum {
    pub name: Identifier,
    pub kind: EnumKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnumKind {
    /// Represents an enum with no variants
    NoVariants,
    Variants(Vec<Variant>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    pub name: Identifier,
    pub kind: VariantKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariantKind {
    Plain,
    CLike(u128),
    Tuple(Vec<Ty>),
    Struct(Vec<StructMember>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Impl {
    pub target: Ty,
    pub body: Vec<Item>,
}

// TODO: `impl` Trait for <Target> { }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImplKind {
    Type(Box<Ty>),
    Trait { impl_trait: Path, for_type: Box<Ty> },
}

/// # Static Type Information
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ty {
    pub extents: Span,
    pub kind: TyKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TyKind {
    Never,
    Empty,
    SelfTy,
    Path(Path),
    Tuple(TyTuple),
    Ref(TyRef),
    Fn(TyFn),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TyTuple {
    pub types: Vec<Ty>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TyRef {
    pub count: u16,
    pub to: Path,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TyFn {
    pub args: TyTuple,
    pub rety: Option<Box<Ty>>,
}

// Path
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path {
    pub absolute: bool,
    pub parts: Vec<PathPart>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathPart {
    SuperKw,
    SelfKw,
    Ident(Identifier),
}

// TODO: Capture token?
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identifier(pub String);

/// Stores an abstract statement, and associated metadata
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stmt {
    pub extents: Span,
    pub kind: StmtKind,
    pub semi: Semi,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Semi {
    Terminated,
    Unterminated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StmtKind {
    Empty,
    Local(Let),
    Item(Box<Item>),
    Expr(Box<Expr>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Let {
    pub mutable: Mutability,
    pub name: Identifier,
    pub ty: Option<Box<Ty>>,
    pub init: Option<Box<Expr>>,
}

/// Stores an abstract expression, and associated metadata
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expr {
    pub extents: Span,
    pub kind: ExprKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExprKind {
    /// An [Assign]ment expression: [`Expr`] ([`AssignKind`] [`Expr`])\+
    Assign(Box<Assign>),
    /// A [Binary] expression: [`Expr`] ([`BinaryKind`] [`Expr`])\+
    Binary(Binary),
    /// A [Unary] expression: [`UnaryKind`]\* [`Expr`]
    Unary(Unary),
    /// A [Member] access expression: [`Expr`] (`.` [`Expr`])+
    Member(Member),
    /// A [Call] expression, with arguments: a(foo, bar)
    Call(Call),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assign {
    pub head: Expr,
    pub op: AssignKind,
    pub tail: Box<Expr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binary {
    pub head: Box<Expr>,
    pub tail: Vec<(BinaryKind, Expr)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unary {
    pub ops: Vec<UnaryKind>,
    pub tail: Box<Expr>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnaryKind {
    Deref,
    Neg,
    Not,
    /// Unused
    At,
    /// Unused
    Tilde,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    pub head: Box<Expr>,
    pub tail: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemberKind {
    Dot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    pub callee: Box<Expr>,
    pub args: Vec<Tuple>,
}

/// Index operator: Member (`[` Expr `]`)*
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Index {
    pub head: Box<Expr>,
    pub indices: Vec<Indices>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Indices {
    pub exprs: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Literal {
    Bool(bool),
    Char(char),
    Int(u128),
    String(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Array {
    pub values: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArrayRep {
    pub value: Box<Expr>,
    pub repeat: Box<Expr>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddrOf {
    pub count: usize,
    pub mutable: Mutability,
    pub expr: Box<Expr>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub expr: Box<Expr>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tuple {
    pub exprs: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct While {
    pub cond: Box<Expr>,
    pub pass: Box<Block>,
    pub fail: Else,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct If {
    pub cond: Box<Expr>,
    pub pass: Box<Block>,
    pub fail: Else,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct For {
    pub bind: Identifier, // TODO: Patterns?
    pub cond: Box<Expr>,
    pub pass: Box<Block>,
    pub fail: Else,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Else {
    pub body: Option<Box<Expr>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Break {
    pub body: Option<Box<Expr>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Return {
    pub body: Option<Box<Expr>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Continue;
