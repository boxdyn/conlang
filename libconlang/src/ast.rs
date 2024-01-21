//! # The Abstract Syntax Tree
//! Contains definitions of AST Nodes, to be derived by a [parser](super::parser).
//!
//! # Notable nodes
//! - [Item] and [ItemKind]: Top-level constructs
//! - [Stmt] and [StmtKind]: Statements
//! - [Expr] and [ExprKind]: Expressions
//!   - [Assign], [Binary], and [Unary] expressions
//!   - [AssignKind], [BinaryKind], and [UnaryKind] operators
//! - [Ty] and [TyKind]: Type qualifiers
//! - [Path]: Path expressions
//!
//! # Notable structures
//! - [struct@Span]: Stores the start and end [struct@Loc] of a notable AST node
//! - [struct@Loc]: Stores the line/column of a notable AST node
#![allow(non_snake_case)]

pub mod ast_impl;

// Universal data types

/// Stores the start and end [locations](struct@Loc) within the token stream
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    pub head: Loc,
    pub tail: Loc,
}
pub fn Span(head: Loc, tail: Loc) -> Span {
    Span { head, tail }
}

/// Stores a read-only (line, column) location in a token stream
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Loc {
    line: u32,
    col: u32,
}
pub fn Loc(line: u32, col: u32) -> Loc {
    Loc { line, col }
}
impl Loc {
    pub fn line(self) -> u32 {
        self.line
    }
    pub fn col(self) -> u32 {
        self.col
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mutability {
    #[default]
    Not,
    Mut,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Visibility {
    #[default]
    Private,
    Public,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct File {
    pub items: Vec<Item>,
}

// Items
/// Holds an abstract Item and associated metadata
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Item {
    pub extents: Span,
    pub vis: Visibility,
    pub kind: ItemKind,
}

/// Stores a concrete Item
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ItemKind {
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

/// Stores a `const` value
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Const {
    pub name: Identifier,
    pub ty: Box<Ty>,
    pub init: Box<Expr>,
    // TODO: Type path in const
}

/// Stores a `static` variable
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Static {
    pub mutable: Mutability,
    pub name: Identifier,
    pub ty: Box<Ty>,
    pub init: Box<Expr>,
    // TODO: Type path in static
}

/// Stores a collection of [Items](Item)
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Module {
    pub name: Identifier,
    pub kind: ModuleKind,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModuleKind {
    Inline(File),
    Outline,
}

/// Contains code, and the interface to that code
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Function {
    pub name: Identifier,
    pub args: Vec<Param>,
    pub body: Option<Block>,
    pub rety: Option<Box<Ty>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Param {
    pub mutability: Mutability,
    pub name: Identifier,
    pub ty: Box<Ty>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Struct {
    pub name: Identifier,
    pub kind: StructKind,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StructKind {
    Empty,
    Tuple(Vec<Ty>),
    Struct(Vec<StructMember>),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StructMember {
    pub vis: Visibility,
    pub name: Identifier,
    pub ty: Ty,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Enum {
    pub name: Identifier,
    pub kind: EnumKind,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnumKind {
    /// Represents an enum with no variants
    NoVariants,
    Variants(Vec<Variant>),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Variant {
    pub name: Identifier,
    pub kind: VariantKind,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum VariantKind {
    Named(Identifier),
    Tuple(Vec<Ty>),
    Struct(Vec<StructMember>),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Impl {
    pub target: Ty,
    pub body: Vec<Item>,
    // TODO: `impl` <Target> { }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImplKind {
    Type(Box<Ty>),
    Trait { impl_trait: Path, for_type: Box<Ty> },
}

/// # Static Type Information
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ty {
    pub extents: Span,
    pub kind: TyKind,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TyKind {
    Never,
    Empty,
    Path(Path),
    Tuple(TyTuple),
    Ref(TyRef),
    Fn(TyFn),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TyTuple {
    pub types: Vec<Ty>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TyRef {
    pub count: usize,
    pub to: Path,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TyFn {
    pub args: TyTuple,
    pub rety: Option<Box<Ty>>,
}

// Path
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Path {
    pub absolute: bool,
    pub parts: Vec<PathPart>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PathPart {
    SuperKw,
    SelfKw,
    Ident(Identifier),
}

// TODO: Capture token?
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Identifier(pub String);

/// Stores an abstract statement, and associated metadata
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Stmt {
    pub extents: Span,
    pub kind: StmtKind,
    pub semi: Semi,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Semi {
    Terminated,
    Unterminated,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StmtKind {
    Empty,
    Local(Let),
    Item(Box<Item>),
    Expr(Box<Expr>),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Let {
    pub mutable: Mutability,
    pub name: Identifier,
    pub init: Option<Box<Expr>>,
}

/// Stores an abstract expression, and associated metadata
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Expr {
    pub extents: Span,
    pub kind: ExprKind,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExprKind {
    Assign(Box<Assign>),
    /// A [Binary] expression, with a leading and trailing side
    Binary(Binary),
    /// A [Unary] expression, with a trailing side
    Unary(Unary),
    /// A [Member] access expression
    Member(Member),
    /// A [Call] expression, with arguments
    Call(Call),
    /// An Array [Index] expression
    Index(Index),
    /// A [path expression](Path): `::`? [PathPart] (`::` [PathPart])*
    Path(Path),
    /// A [Literal]: 0x42, 1e123, 2.4, "Hello"
    Literal(Literal),
    /// An [Array] literal: `[` Expr (`,` Expr)* `]`
    Array(Array),
    /// An Array literal constructed with [repeat syntax](ArrayRep)
    /// `[` [Expr] `;` [Literal] `]`
    ArrayRep(ArrayRep),
    /// An address-of expression: `&`foo
    AddrOf(AddrOf),
    /// A [Block] expression: `{` Stmt* Expr? `}`
    Block(Block),
    /// An empty expression: `(` `)`
    Empty,
    /// A [Grouping](Group) expression `(` Expr `)`
    Group(Group),
    /// A [Tuple] expression: `(` Expr (`,` Expr)+ `)`
    Tuple(Tuple),
    /// A [While] expression: `while` Expr Block Else?
    While(While),
    /// An [If] expression: `if` Expr Block Else?
    If(If),
    /// A [For] expression: `for` Pattern in Expr Block Else?
    For(For),
    /// A [Break] expression: `break` Expr?
    Break(Break),
    /// A [Return] expression `return` Expr?
    Return(Return),
    /// A continue expression: `continue`
    Continue(Continue),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Assign {
    pub head: Expr,
    pub op: AssignKind,
    pub tail: Box<Expr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Binary {
    pub head: Box<Expr>,
    pub tail: Vec<(BinaryKind, Expr)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Unary {
    pub ops: Vec<UnaryKind>,
    pub tail: Box<Expr>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnaryKind {
    Deref,
    Neg,
    Not,
    /// Unused
    At,
    /// Unused
    Hash,
    /// Unused
    Tilde,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Member {
    pub head: Box<Expr>,
    pub tail: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemberKind {
    Dot,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Call {
    pub callee: Box<Expr>,
    pub args: Vec<Tuple>,
}

/// Index operator: Member (`[` Expr `]`)*
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Index {
    pub head: Box<Expr>,
    pub indices: Vec<Indices>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Indices {
    pub exprs: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Literal {
    Bool(bool),
    Char(char),
    Int(u128),
    String(String),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Array {
    pub values: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArrayRep {
    pub value: Box<Expr>,
    pub repeat: Box<Expr>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AddrOf {
    pub count: usize,
    pub mutable: Mutability,
    pub expr: Box<Expr>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Group {
    pub expr: Box<Expr>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tuple {
    pub exprs: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct While {
    pub cond: Box<Expr>,
    pub pass: Box<Block>,
    pub fail: Else,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct If {
    pub cond: Box<Expr>,
    pub pass: Box<Block>,
    pub fail: Else,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct For {
    pub bind: Identifier, // TODO: Patterns?
    pub cond: Box<Expr>,
    pub pass: Box<Block>,
    pub fail: Else,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Else {
    pub body: Option<Box<Expr>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Break {
    pub body: Option<Box<Expr>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Return {
    pub body: Option<Box<Expr>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Continue;
