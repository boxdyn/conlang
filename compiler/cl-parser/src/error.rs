use super::*;

use cl_ast::Expr;
use cl_lexer::error::{Error as LexError, Reason};
use std::fmt::Display;
pub type PResult<T> = Result<T, Error>;

/// Contains information about [Parser] errors
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub reason: ErrorKind,
    pub while_parsing: Parsing,
    pub loc: Loc,
}
impl std::error::Error for Error {}

/// Represents the reason for parse failure
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Lexical(LexError),
    EndOfInput,
    UnmatchedParentheses,
    UnmatchedCurlyBraces,
    UnmatchedSquareBrackets,
    Unexpected(TokenKind),
    ExpectedToken {
        want: TokenKind,
        got: TokenKind,
    },
    ExpectedParsing {
        want: Parsing,
    },
    InvalidPattern(Box<Expr>),
    /// Indicates unfinished code
    Todo(&'static str),
}
impl From<LexError> for ErrorKind {
    fn from(value: LexError) -> Self {
        match value.reason() {
            Reason::EndOfFile => Self::EndOfInput,
            _ => Self::Lexical(value),
        }
    }
}

/// Compactly represents the stage of parsing an [Error] originated in
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parsing {
    Mutability,
    Visibility,
    Identifier,
    Literal,

    File,

    Attrs,
    Meta,
    MetaKind,

    Item,
    ItemKind,
    Alias,
    Const,
    Static,
    Module,
    ModuleKind,
    Function,
    Param,
    Struct,
    StructKind,
    StructMember,
    Enum,
    EnumKind,
    Variant,
    VariantKind,
    Impl,
    ImplKind,
    Use,
    UseTree,

    Ty,
    TyKind,
    TySlice,
    TyArray,
    TyTuple,
    TyRef,
    TyFn,

    Path,
    PathPart,

    Stmt,
    StmtKind,
    Let,

    Expr,
    ExprKind,
    Assign,
    AssignKind,
    Binary,
    BinaryKind,
    Unary,
    UnaryKind,
    Cast,
    Index,
    Structor,
    Fielder,
    Call,
    Member,
    Array,
    ArrayRep,
    AddrOf,
    Block,
    Group,
    Tuple,
    Loop,
    While,
    If,
    For,
    Else,
    Break,
    Return,
    Continue,

    Pattern,
    Match,
    MatchArm,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { reason, while_parsing, loc } = self;
        match reason {
            // TODO entries are debug-printed
            ErrorKind::Todo(_) => write!(f, "{loc} {reason} {while_parsing:?}"),
            // lexical errors print their own higher-resolution loc info
            ErrorKind::Lexical(e) => write!(f, "{e} (while parsing {while_parsing})"),
            _ => write!(f, "{loc}: {reason} while parsing {while_parsing}"),
        }
    }
}
impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::Lexical(e) => e.fmt(f),
            ErrorKind::EndOfInput => write!(f, "End of input"),
            ErrorKind::UnmatchedParentheses => write!(f, "Unmatched parentheses"),
            ErrorKind::UnmatchedCurlyBraces => write!(f, "Unmatched curly braces"),
            ErrorKind::UnmatchedSquareBrackets => write!(f, "Unmatched square brackets"),
            ErrorKind::Unexpected(t) => write!(f, "Encountered unexpected token `{t}`"),
            ErrorKind::ExpectedToken { want: e, got: g } => write!(f, "Expected `{e}`, got `{g}`"),
            ErrorKind::ExpectedParsing { want } => write!(f, "Expected {want}"),
            ErrorKind::InvalidPattern(got) => write!(f, "Got invalid `{got}`"),
            ErrorKind::Todo(unfinished) => write!(f, "TODO: {unfinished}"),
        }
    }
}
impl Display for Parsing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Parsing::Visibility => "a visibility qualifier",
            Parsing::Mutability => "a mutability qualifier",
            Parsing::Identifier => "an identifier",
            Parsing::Literal => "a literal",

            Parsing::File => "a file",

            Parsing::Attrs => "an attribute-set",
            Parsing::Meta => "an attribute",
            Parsing::MetaKind => "an attribute's arguments",
            Parsing::Item => "an item",
            Parsing::ItemKind => "an item",
            Parsing::Alias => "a type alias",
            Parsing::Const => "a const item",
            Parsing::Static => "a static variable",
            Parsing::Module => "a module",
            Parsing::ModuleKind => "a module",
            Parsing::Function => "a function",
            Parsing::Param => "a function parameter",
            Parsing::Struct => "a struct",
            Parsing::StructKind => "a struct",
            Parsing::StructMember => "a struct member",
            Parsing::Enum => "an enum",
            Parsing::EnumKind => "an enum",
            Parsing::Variant => "an enum variant",
            Parsing::VariantKind => "an enum variant",
            Parsing::Impl => "an impl block",
            Parsing::ImplKind => "the target of an impl block",
            Parsing::Use => "a use item",
            Parsing::UseTree => "a use-tree",

            Parsing::Ty => "a type",
            Parsing::TyKind => "a type",
            Parsing::TySlice => "a slice type",
            Parsing::TyArray => "an array type",
            Parsing::TyTuple => "a tuple of types",
            Parsing::TyRef => "a reference type",
            Parsing::TyFn => "a function pointer type",

            Parsing::Path => "a path",
            Parsing::PathPart => "a path component",

            Parsing::Stmt => "a statement",
            Parsing::StmtKind => "a statement",
            Parsing::Let => "a local variable declaration",

            Parsing::Expr => "an expression",
            Parsing::ExprKind => "an expression",
            Parsing::Assign => "an assignment",
            Parsing::AssignKind => "an assignment operator",
            Parsing::Binary => "a binary expression",
            Parsing::BinaryKind => "a binary operator",
            Parsing::Unary => "a unary expression",
            Parsing::UnaryKind => "a unary operator",
            Parsing::Cast => "an `as`-casting expression",
            Parsing::Index => "an indexing expression",
            Parsing::Structor => "a struct constructor expression",
            Parsing::Fielder => "a struct field expression",
            Parsing::Call => "a call expression",
            Parsing::Member => "a member access expression",
            Parsing::Array => "an array",
            Parsing::ArrayRep => "an array of form [k;N]",
            Parsing::AddrOf => "a borrow op",
            Parsing::Block => "a block",
            Parsing::Group => "a grouped expression",
            Parsing::Tuple => "a tuple",
            Parsing::Loop => "an unconditional loop expression",
            Parsing::While => "a while expression",
            Parsing::If => "an if expression",
            Parsing::For => "a for expression",
            Parsing::Else => "an else block",
            Parsing::Break => "a break expression",
            Parsing::Return => "a return expression",
            Parsing::Continue => "a continue expression",

            Parsing::Pattern => "a pattern",
            Parsing::Match => "a match expression",
            Parsing::MatchArm => "a match arm",
        }
        .fmt(f)
    }
}
