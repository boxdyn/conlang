//! # The Abstract Syntax Tree
//! Contains definitions of Conlang AST Nodes.
//!
//! # Notable nodes
//! - [Item] and [ItemKind]: Top-level constructs
//! - [Stmt] and [StmtKind]: Statements
//! - [Expr] and [ExprKind]: Expressions
//!   - [Assign], [Modify], [Binary], and [Unary] expressions
//!   - [ModifyKind], [BinaryKind], and [UnaryKind] operators
//! - [Ty] and [TyKind]: Type qualifiers
//! - [Path]: Path expressions
use cl_structures::{intern::interned::Interned, span::*};

/// An [Interned] static [str], used in place of an identifier
pub type Sym = Interned<'static, str>;

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

/// A [Literal]: 0x42, 1e123, 2.4, "Hello"
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Literal {
    Bool(bool),
    Char(char),
    Int(u128),
    String(String),
}

/// A list of [Item]s
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct File {
    pub items: Vec<Item>,
}

/// A list of [Meta] decorators
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Attrs {
    pub meta: Vec<Meta>,
}

/// A metadata decorator
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Meta {
    pub name: Sym,
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
    /// An [import](Use)
    Use(Use),
}

/// An alias to another [Ty]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Alias {
    pub to: Sym,
    pub from: Option<Box<Ty>>,
}

/// A compile-time constant
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Const {
    pub name: Sym,
    pub ty: Box<Ty>,
    pub init: Box<Expr>,
}

/// A `static` variable
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Static {
    pub mutable: Mutability,
    pub name: Sym,
    pub ty: Box<Ty>,
    pub init: Box<Expr>,
}

/// An ordered collection of [Items](Item)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Module {
    pub name: Sym,
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
    pub name: Sym,
    pub sign: TyFn,
    pub bind: Vec<Param>,
    pub body: Option<Block>,
}

/// A single parameter for a [Function]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Param {
    pub mutability: Mutability,
    pub name: Sym,
}

/// A user-defined product type
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Struct {
    pub name: Sym,
    pub kind: StructKind,
}

/// Either a [Struct]'s [StructMember]s or tuple [Ty]pes, if present.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StructKind {
    Empty,
    Tuple(Vec<Ty>),
    Struct(Vec<StructMember>),
}

/// The [Visibility], [Sym], and [Ty]pe of a single [Struct] member
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StructMember {
    pub vis: Visibility,
    pub name: Sym,
    pub ty: Ty,
}

/// A user-defined sum type
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Enum {
    pub name: Sym,
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
    pub name: Sym,
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

/// An import of nonlocal [Item]s
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Use {
    pub absolute: bool,
    pub tree: UseTree,
}

/// A tree of [Item] imports
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum UseTree {
    Tree(Vec<UseTree>),
    Path(PathPart, Box<UseTree>),
    Alias(Sym, Sym),
    Name(Sym),
    Glob,
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
    Path(Path),
    Array(TyArray),
    Slice(TySlice),
    Tuple(TyTuple),
    Ref(TyRef),
    Fn(TyFn),
    // TODO: slice, array types
}

/// An array of [`T`](Ty)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TyArray {
    pub ty: Box<TyKind>,
    pub count: usize,
}

/// A [Ty]pe slice expression: `[T]`
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TySlice {
    pub ty: Box<TyKind>,
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
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Path {
    pub absolute: bool,
    pub parts: Vec<PathPart>,
}

/// A single component of a [Path]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PathPart {
    SuperKw,
    SelfKw,
    SelfTy,
    Ident(Sym),
}

/// An abstract statement, and associated metadata
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Stmt {
    pub extents: Span,
    pub kind: StmtKind,
    pub semi: Semi,
}

/// Whether the [Stmt] is a [Let], [Item], or [Expr] statement
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StmtKind {
    Empty,
    Local(Let),
    Item(Box<Item>),
    Expr(Box<Expr>),
}

/// Whether or not a [Stmt] is followed by a semicolon
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Semi {
    Terminated,
    Unterminated,
}

/// A local variable declaration [Stmt]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Let {
    pub mutable: Mutability,
    pub name: Sym,
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
#[derive(Clone, Default, Debug, PartialEq, Eq, Hash)]
pub enum ExprKind {
    /// An empty expression: `(` `)`
    #[default]
    Empty,
    /// An [Assign]ment expression: [`Expr`] (`=` [`Expr`])\+
    Assign(Assign),
    /// A [Modify]-assignment expression: [`Expr`] ([`ModifyKind`] [`Expr`])\+
    Modify(Modify),
    /// A [Binary] expression: [`Expr`] ([`BinaryKind`] [`Expr`])\+
    Binary(Binary),
    /// A [Unary] expression: [`UnaryKind`]\* [`Expr`]
    Unary(Unary),
    /// A [Cast] expression: [`Expr`] `as` [`Ty`]
    Cast(Cast),
    /// A [Member] access expression: [`Expr`] [`MemberKind`]\*
    Member(Member),
    /// An Array [Index] expression: a[10, 20, 30]
    Index(Index),
    /// A [Struct creation](Structor) expression: [Path] `{` ([Fielder] `,`)* [Fielder]? `}`
    Structor(Structor),
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

/// An [Assign]ment expression: [`Expr`] ([`ModifyKind`] [`Expr`])\+
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Assign {
    pub parts: Box<(ExprKind, ExprKind)>,
}

/// A [Modify]-assignment expression: [`Expr`] ([`ModifyKind`] [`Expr`])\+
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Modify {
    pub kind: ModifyKind,
    pub parts: Box<(ExprKind, ExprKind)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModifyKind {
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Cast {
    pub head: Box<ExprKind>,
    pub ty: Ty,
}

/// A [Member] access expression: [`Expr`] [`MemberKind`]\*
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Member {
    pub head: Box<ExprKind>,
    pub kind: MemberKind,
}

/// The kind of [Member] access
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MemberKind {
    Call(Sym, Tuple),
    Struct(Sym),
    Tuple(Literal),
}

/// A repeated [Index] expression: a[10, 20, 30][40, 50, 60]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Index {
    pub head: Box<ExprKind>,
    pub indices: Vec<Expr>,
}

/// A [Struct creation](Structor) expression: [Path] `{` ([Fielder] `,`)* [Fielder]? `}`
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Structor {
    pub to: Path,
    pub init: Vec<Fielder>,
}

/// A [Struct field initializer] expression: [Sym] (`=` [Expr])?
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Fielder {
    pub name: Sym,
    pub init: Option<Box<Expr>>,
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
    pub bind: Sym, // TODO: Patterns?
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
