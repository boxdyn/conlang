//! Parses [tokens](super::token) into an [AST](super::ast)
//! 
//! For the full grammar, see [grammar.ebnf][1]
//! 
//! [1]: https://github.com/boxdyn/conlang/src/branch/main/grammar.ebnf

use self::error::{
    Error,
    ErrorKind::{self, *},
    PResult, WhileParsing,
};
use crate::{
    ast::*,
    lexer::{error::Error as LexError, Lexer},
    token::{
        token_data::Data,
        token_type::{Keyword, Type},
        Token,
    },
};

pub mod error {
    use std::fmt::Display;

    use super::*;
    pub type PResult<T> = Result<T, Error>;

    /// Contains information about [Parser] errors
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Error {
        pub reason: ErrorKind,
        pub while_parsing: WhileParsing,
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
        Unexpected(Type),
        Expected {
            want: Type,
            got: Type,
        },
        /// No rules matched
        Nothing,
        /// Indicates unfinished code
        Todo,
    }
    impl From<LexError> for ErrorKind {
        fn from(value: LexError) -> Self {
            use crate::lexer::error::Reason;
            match value.reason() {
                Reason::EndOfFile => Self::EndOfInput,
                _ => Self::Lexical(value),
            }
        }
    }

    /// Compactly represents the stage of parsing an [Error] originated in
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum WhileParsing {
        File,

        Item,
        Visibility,
        Mutability,
        ItemKind,
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

        Ty,
        TyKind,
        TyTuple,
        TyRef,
        TyFn,

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
        Index,
        Call,
        Member,
        PathExpr,
        PathPart,
        Identifier,
        Literal,
        Array,
        ArrayRep,
        AddrOf,
        Block,
        Group,
        Tuple,
        While,
        If,
        For,
        Else,
        Break,
        Return,
        Continue,
    }

    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { reason, while_parsing, loc } = self;
            match reason {
                // TODO entries are debug-printed
                Todo => write!(f, "{loc} {reason} {while_parsing:?}"),
                // lexical errors print their own higher-resolution loc info
                Lexical(e) => write!(f, "{e} (while parsing {while_parsing})"),
                _ => write!(f, "{loc} {reason} while parsing {while_parsing}"),
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
                ErrorKind::Expected { want: e, got: g } => {
                    write!(f, "Expected {e}, but got {g}")
                }
                ErrorKind::Nothing => write!(f, "Nothing found"),
                ErrorKind::Todo => write!(f, "TODO:"),
            }
        }
    }
    impl Display for WhileParsing {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                WhileParsing::File => "a file",
                WhileParsing::Item => "an item",
                WhileParsing::Visibility => "a visibility qualifier",
                WhileParsing::Mutability => "a mutability qualifier",
                WhileParsing::ItemKind => "an item",
                WhileParsing::Const => "a const item",
                WhileParsing::Static => "a static variable",
                WhileParsing::Module => "a module",
                WhileParsing::ModuleKind => "a module",
                WhileParsing::Function => "a function",
                WhileParsing::Param => "a function parameter",
                WhileParsing::Struct => "a struct",
                WhileParsing::StructKind => "a struct",
                WhileParsing::StructMember => "a struct member",
                WhileParsing::Enum => "an enum",
                WhileParsing::EnumKind => "an enum",
                WhileParsing::Variant => "an enum variant",
                WhileParsing::VariantKind => "an enum variant",
                WhileParsing::Impl => "an impl block",

                WhileParsing::Ty => "a type",
                WhileParsing::TyKind => "a type",
                WhileParsing::TyTuple => "a tuple of types",
                WhileParsing::TyRef => "a reference type",
                WhileParsing::TyFn => "a function pointer type",

                WhileParsing::Stmt => "a statement",
                WhileParsing::StmtKind => "a statement",
                WhileParsing::Let => "a local variable declaration",

                WhileParsing::Expr => "an expression",
                WhileParsing::ExprKind => "an expression",
                WhileParsing::Assign => "an assignment",
                WhileParsing::AssignKind => "an assignment",
                WhileParsing::Binary => "a binary expression",
                WhileParsing::BinaryKind => "a binary expression",
                WhileParsing::Unary => "a unary expression",
                WhileParsing::UnaryKind => "a unary expression",
                WhileParsing::Index => "an indexing expression",
                WhileParsing::Call => "a call expression",
                WhileParsing::Member => "a member access expression",
                WhileParsing::PathExpr => "a path",
                WhileParsing::PathPart => "a path component",
                WhileParsing::Identifier => "an identifier",
                WhileParsing::Literal => "a literal",
                WhileParsing::Array => "an array",
                WhileParsing::ArrayRep => "an array of form [k;N]",
                WhileParsing::AddrOf => "a borrow op",
                WhileParsing::Block => "a block",
                WhileParsing::Group => "a grouped expression",
                WhileParsing::Tuple => "a tuple",
                WhileParsing::While => "a while expression",
                WhileParsing::If => "an if expression",
                WhileParsing::For => "a for expression",
                WhileParsing::Else => "an else block",
                WhileParsing::Break => "a break expression",
                WhileParsing::Return => "a return expression",
                WhileParsing::Continue => "a continue expression",
            }
            .fmt(f)
        }
    }
}

pub struct Parser<'t> {
    /// Lazy tokenizer
    lexer: Lexer<'t>,
    /// Look-ahead buffer
    next: Option<Token>,
    /// The location of the current token
    loc: Loc,
}

impl<'t> Parser<'t> {
    pub fn new(lexer: Lexer<'t>) -> Self {
        Self { loc: Loc::from(&lexer), lexer, next: None }
    }
    /// Gets the location of the last consumed [Token]
    pub fn loc(&self) -> Loc {
        self.loc
    }

    /// Constructs an [Error]
    fn error(&self, reason: ErrorKind, while_parsing: WhileParsing) -> Error {
        Error { reason, while_parsing, loc: self.loc }
    }
    /// Internal impl of peek and consume
    fn consume_from_lexer(&mut self, while_parsing: WhileParsing) -> PResult<Token> {
        loop {
            match self
                .lexer
                .scan()
                .map_err(|e| self.error(e.into(), while_parsing))?
            {
                t if t.ty() == Type::Comment => continue,
                t => break Ok(t),
            }
        }
    }
    /// Looks ahead one token
    ///
    /// Stores the token in an internal lookahead buffer
    pub fn peek(&mut self, while_parsing: WhileParsing) -> PResult<&Token> {
        if self.next.is_none() {
            self.next = Some(self.consume_from_lexer(while_parsing)?);
        }
        self.next.as_ref().ok_or_else(|| unreachable!())
    }
    /// Consumes a previously peeked [Token], returning it.
    /// Returns [None] when there is no peeked token.
    ///
    /// This avoids the overhead of constructing an [Error]
    pub fn consume_peeked(&mut self) -> Option<Token> {
        // location must be updated whenever a token is pulled from the lexer
        self.loc = Loc::from(&self.lexer);
        self.next.take()
    }
    /// Looks ahead at the next [Token]'s [Type]
    pub fn peek_type(&mut self, while_parsing: WhileParsing) -> PResult<Type> {
        self.peek(while_parsing).map(|t| t.ty())
    }
    /// Consumes one [Token]
    pub fn consume(&mut self, while_parsing: WhileParsing) -> PResult<Token> {
        self.loc = Loc::from(&self.lexer);
        match self.next.take() {
            Some(token) => Ok(token),
            None => self.consume_from_lexer(while_parsing),
        }
    }
    /// Consumes the next [Token] if it matches the pattern [Type]
    pub fn match_type(&mut self, want: Type, while_parsing: WhileParsing) -> PResult<Token> {
        let got = self.peek_type(while_parsing)?;
        if got == want {
            Ok(self.consume_peeked().expect("should not fail after peek"))
        } else {
            Err(self.error(Expected { want, got }, while_parsing))
        }
    }
    /// Consumes the next token if it matches the pattern [Keyword]
    pub fn match_kw(&mut self, pat: Keyword, while_parsing: WhileParsing) -> PResult<Token> {
        self.match_type(Type::Keyword(pat), while_parsing)
    }
}

/// Generic parse functions
impl<'t> Parser<'t> {
    /// Parses constructions of the form `Open F Close`
    fn delimited<T, F>(
        &mut self,
        open: Type,
        f: F,
        close: Type,
        while_parsing: WhileParsing,
    ) -> PResult<T>
    where
        F: Fn(&mut Self) -> PResult<T>,
    {
        self.match_type(open, while_parsing)?;
        let out = f(self)?;
        self.match_type(close, while_parsing)?;
        Ok(out)
    }
    /// Parses constructions of the form `(F Separator ~Terminator)*`
    ///
    /// where `~Terminator` is a negative lookahead assertion
    fn separated<T, F>(
        &mut self,
        separator: Type,
        f: F,
        terminator: Type,
        while_parsing: WhileParsing,
    ) -> PResult<Vec<T>>
    where
        F: Fn(&mut Self) -> PResult<T>,
    {
        let mut args = vec![];
        while terminator != self.peek_type(while_parsing)? {
            args.push(f(self)?);
            if separator != self.peek_type(while_parsing)? {
                break;
            }
            self.consume_peeked();
        }
        Ok(args)
    }
    /// Parses constructions of the form `(F ~Terminator)*`
    ///
    /// where `~Terminator` is a negative lookahead assertion
    fn repeated<T, F>(
        &mut self,
        f: F,
        terminator: Type,
        while_parsing: WhileParsing,
    ) -> PResult<Vec<T>>
    where
        F: Fn(&mut Self) -> PResult<T>,
    {
        let mut out = vec![];
        while terminator != self.peek_type(while_parsing)? {
            out.push(f(self)?);
        }
        Ok(out)
    }
}

/// Expands to a pattern which matches item-like [Token] [Type]s
macro item_like() {
    Type::Keyword(
        Keyword::Pub
            | Keyword::Const
            | Keyword::Static
            | Keyword::Mod
            | Keyword::Fn
            | Keyword::Struct
            | Keyword::Enum
            | Keyword::Impl,
    )
}

/// Top level parsing
impl<'t> Parser<'t> {
    /// Parses a [File]
    pub fn file(&mut self) -> PResult<File> {
        let mut items = vec![];
        while match self.peek_type(WhileParsing::File) {
            Ok(Type::RCurly) | Err(Error { reason: EndOfInput, .. }) => false,
            Ok(_) => true,
            Err(e) => Err(e)?,
        } {
            items.push(self.item()?)
        }
        Ok(File { items })
    }

    /// Parses an [Item]
    ///
    /// See also: [Parser::itemkind]
    pub fn item(&mut self) -> PResult<Item> {
        let start = self.loc();
        Ok(Item {
            vis: self.visibility()?,
            kind: self.itemkind()?,
            extents: Span(start, self.loc()),
        })
    }

    /// Parses a [Ty]
    ///
    /// See also: [Parser::tykind]
    pub fn ty(&mut self) -> PResult<Ty> {
        let start = self.loc();
        Ok(Ty { kind: self.tykind()?, extents: Span(start, self.loc()) })
    }

    /// Parses a [Path]
    ///
    /// See also: [Parser::path_part], [Parser::identifier]
    pub fn path(&mut self) -> PResult<Path> {
        const PARSING: WhileParsing = WhileParsing::PathExpr;
        let absolute = matches!(self.peek_type(PARSING)?, Type::ColonColon);
        if absolute {
            self.consume_peeked();
        }

        let mut parts = vec![self.path_part()?];
        while let Ok(Type::ColonColon) = self.peek_type(PARSING) {
            self.consume_peeked();
            parts.push(self.path_part()?);
        }
        Ok(Path { absolute, parts })
    }

    /// Parses a [Stmt]
    ///
    /// See also: [Parser::stmtkind]
    pub fn stmt(&mut self) -> PResult<Stmt> {
        const PARSING: WhileParsing = WhileParsing::Stmt;
        let start = self.loc();
        Ok(Stmt {
            kind: match self.peek_type(PARSING)? {
                Type::Semi => Ok(StmtKind::Empty),
                Type::Keyword(Keyword::Let) => self.stmtkind_local(),
                item_like!() => self.stmtkind_item(),
                _ => self.stmtkind_expr(),
            }?,
            semi: match self.peek_type(PARSING) {
                Ok(Type::Semi) => {
                    self.consume_peeked();
                    Semi::Terminated
                }
                _ => Semi::Unterminated,
            },
            extents: Span(start, self.loc()),
        })
    }

    /// Parses an [Expr]
    ///
    /// See also: [Parser::exprkind]
    pub fn expr(&mut self) -> PResult<Expr> {
        self.expr_from(Self::exprkind)
    }
}

/// Item parsing
impl<'t> Parser<'t> {
    /// Parses an [ItemKind]
    ///
    /// See also: [Parser::item]
    pub fn itemkind(&mut self) -> PResult<ItemKind> {
        Ok(match self.peek_type(WhileParsing::Item)? {
            Type::Keyword(Keyword::Const) => self.parse_const()?.into(),
            Type::Keyword(Keyword::Static) => self.parse_static()?.into(),
            Type::Keyword(Keyword::Mod) => self.parse_module()?.into(),
            Type::Keyword(Keyword::Fn) => self.parse_function()?.into(),
            Type::Keyword(Keyword::Struct) => self.parse_struct()?.into(),
            Type::Keyword(Keyword::Enum) => self.parse_enum()?.into(),
            Type::Keyword(Keyword::Impl) => self.parse_impl()?.into(),
            t => Err(self.error(Unexpected(t), WhileParsing::Item))?,
        })
    }

    pub fn parse_const(&mut self) -> PResult<Const> {
        const PARSING: WhileParsing = WhileParsing::Const;
        self.match_kw(Keyword::Const, PARSING)?;
        let out = Ok(Const {
            name: self.identifier()?,
            ty: {
                self.match_type(Type::Colon, PARSING)?;
                self.ty()?.into()
            },
            init: {
                self.match_type(Type::Eq, PARSING)?;
                self.expr()?.into()
            },
        });
        self.match_type(Type::Semi, PARSING)?;
        out
    }
    pub fn parse_static(&mut self) -> PResult<Static> {
        const PARSING: WhileParsing = WhileParsing::Static;
        self.match_kw(Keyword::Static, PARSING)?;
        let out = Ok(Static {
            mutable: self.mutability()?,
            name: self.identifier()?,
            ty: {
                self.match_type(Type::Colon, PARSING)?;
                self.ty()?.into()
            },
            init: {
                self.match_type(Type::Eq, PARSING)?;
                self.expr()?.into()
            },
        });
        self.match_type(Type::Semi, PARSING)?;
        out
    }
    pub fn parse_module(&mut self) -> PResult<Module> {
        const PARSING: WhileParsing = WhileParsing::Module;
        self.match_kw(Keyword::Mod, PARSING)?;
        Ok(Module { name: self.identifier()?, kind: self.modulekind()? })
    }
    pub fn modulekind(&mut self) -> PResult<ModuleKind> {
        const PARSING: WhileParsing = WhileParsing::ModuleKind;
        match self.peek_type(PARSING)? {
            Type::LCurly => Ok(ModuleKind::Inline(self.delimited(
                Type::LCurly,
                Self::file,
                Type::RCurly,
                PARSING,
            )?)),
            Type::Semi => {
                self.consume_peeked();
                Ok(ModuleKind::Outline)
            }
            got => Err(self.error(Expected { want: Type::Semi, got }, PARSING)),
        }
    }
    pub fn parse_function(&mut self) -> PResult<Function> {
        const PARSING: WhileParsing = WhileParsing::Function;
        self.match_kw(Keyword::Fn, PARSING)?;
        Ok(Function {
            name: self.identifier()?,
            args: self.parse_params()?,
            rety: match self.peek_type(PARSING)? {
                Type::LCurly | Type::Semi => None,
                Type::Arrow => {
                    self.consume_peeked();
                    Some(self.ty()?.into())
                }
                got => Err(self.error(Expected { want: Type::Arrow, got }, PARSING))?,
            },
            body: match self.peek_type(PARSING)? {
                Type::LCurly => Some(self.block()?),
                Type::Semi => {
                    self.consume_peeked();
                    None
                }
                t => Err(self.error(Unexpected(t), PARSING))?,
            },
        })
    }
    pub fn parse_params(&mut self) -> PResult<Vec<Param>> {
        const PARSING: WhileParsing = WhileParsing::Function;
        self.delimited(
            Type::LParen,
            |this| this.separated(Type::Comma, Self::parse_param, Type::RParen, PARSING),
            Type::RParen,
            PARSING,
        )
    }
    pub fn parse_param(&mut self) -> PResult<Param> {
        Ok(Param {
            mutability: self.mutability()?,
            name: self.identifier()?,
            ty: {
                self.match_type(Type::Colon, WhileParsing::Param)?;
                self.ty()?.into()
            },
        })
    }
    pub fn parse_struct(&mut self) -> PResult<Struct> {
        const PARSING: WhileParsing = WhileParsing::Struct;
        self.match_kw(Keyword::Struct, PARSING)?;
        Ok(Struct {
            name: self.identifier()?,
            kind: match self.peek_type(PARSING)? {
                Type::LParen => self.structkind_tuple()?,
                Type::LCurly => self.structkind_struct()?,
                Type::Semi => {
                    self.consume_peeked();
                    StructKind::Empty
                }
                got => Err(self.error(Expected { want: Type::Semi, got }, PARSING))?,
            },
        })
    }
    pub fn structkind_tuple(&mut self) -> PResult<StructKind> {
        const PARSING: WhileParsing = WhileParsing::StructKind;

        Ok(StructKind::Tuple(self.delimited(
            Type::LParen,
            |s| s.separated(Type::Comma, Self::ty, Type::RParen, PARSING),
            Type::RParen,
            PARSING,
        )?))
    }
    pub fn structkind_struct(&mut self) -> PResult<StructKind> {
        const PARSING: WhileParsing = WhileParsing::StructKind;

        Ok(StructKind::Struct(self.delimited(
            Type::LCurly,
            |s| s.separated(Type::Comma, Self::struct_member, Type::RCurly, PARSING),
            Type::RCurly,
            PARSING,
        )?))
    }
    pub fn struct_member(&mut self) -> PResult<StructMember> {
        const PARSING: WhileParsing = WhileParsing::StructMember;
        Ok(StructMember {
            vis: self.visibility()?,
            name: self.identifier()?,
            ty: {
                self.match_type(Type::Colon, PARSING)?;
                self.ty()?
            },
        })
    }
    pub fn parse_enum(&mut self) -> PResult<Enum> {
        const PARSING: WhileParsing = WhileParsing::Enum;
        self.match_kw(Keyword::Enum, PARSING)?;
        Err(self.error(Todo, PARSING))
    }
    pub fn parse_impl(&mut self) -> PResult<Impl> {
        const PARSING: WhileParsing = WhileParsing::Impl;
        self.match_kw(Keyword::Impl, PARSING)?;
        Err(self.error(Todo, PARSING))
    }

    pub fn visibility(&mut self) -> PResult<Visibility> {
        if let Type::Keyword(Keyword::Pub) = self.peek_type(WhileParsing::Visibility)? {
            self.consume_peeked();
            return Ok(Visibility::Public);
        };
        Ok(Visibility::Private)
    }
    pub fn mutability(&mut self) -> PResult<Mutability> {
        if let Type::Keyword(Keyword::Mut) = self.peek_type(WhileParsing::Mutability)? {
            self.consume_peeked();
            return Ok(Mutability::Mut);
        };
        Ok(Mutability::Not)
    }
}

/// # Type parsing
impl<'t> Parser<'t> {
    /// Parses a [TyKind]
    ///
    /// See also: [Parser::ty]
    pub fn tykind(&mut self) -> PResult<TyKind> {
        const PARSING: WhileParsing = WhileParsing::TyKind;
        let out = match self.peek_type(PARSING)? {
            Type::Bang => {
                self.consume_peeked();
                TyKind::Never
            }
            Type::Amp | Type::AmpAmp => self.tyref()?.into(),
            Type::LParen => self.tytuple()?.into(),
            Type::Keyword(Keyword::Fn) => self.tyfn()?.into(),
            path_like!() => self.path()?.into(),
            t => Err(self.error(Unexpected(t), PARSING))?,
        };

        Ok(out)
    }
    /// [TyTuple] = `(` ([Ty] `,`)* [Ty]? `)`
    pub fn tytuple(&mut self) -> PResult<TyTuple> {
        const PARSING: WhileParsing = WhileParsing::TyTuple;
        Ok(TyTuple {
            types: self.delimited(
                Type::LParen,
                |s| s.separated(Type::Comma, Self::ty, Type::RParen, PARSING),
                Type::RParen,
                PARSING,
            )?,
        })
    }
    /// [TyRef] = (`&`|`&&`)* [Path]
    pub fn tyref(&mut self) -> PResult<TyRef> {
        const PARSING: WhileParsing = WhileParsing::TyRef;
        let mut count = 0;
        loop {
            match self.peek_type(PARSING)? {
                Type::Amp => count += 1,
                Type::AmpAmp => count += 2,
                _ => break,
            }
            self.consume_peeked();
        }
        Ok(TyRef { count, to: self.path()? })
    }
    /// [TyFn] = `fn` [TyTuple] (-> [Ty])?
    pub fn tyfn(&mut self) -> PResult<TyFn> {
        const PARSING: WhileParsing = WhileParsing::TyFn;
        self.match_type(Type::Keyword(Keyword::Fn), PARSING)?;
        Ok(TyFn {
            args: self.tytuple()?,
            rety: {
                match self.peek_type(PARSING)? {
                    Type::Arrow => {
                        self.consume_peeked();
                        Some(self.ty()?.into())
                    }
                    _ => None,
                }
            },
        })
    }
}

/// Expands to a pattern which matches literal-like [Type]s
macro literal_like() {
    Type::Keyword(Keyword::True | Keyword::False)
        | Type::String
        | Type::Character
        | Type::Integer
        | Type::Float
}
/// Expands to a pattern which matches path-like [token Types](Type)
macro path_like() {
    Type::Keyword(Keyword::Super | Keyword::SelfKw) | Type::Identifier | Type::ColonColon
}
/// # Path parsing
impl<'t> Parser<'t> {
    /// [PathPart] = `super` | `self` | [Identifier]
    pub fn path_part(&mut self) -> PResult<PathPart> {
        const PARSING: WhileParsing = WhileParsing::PathPart;
        let out = match self.peek_type(PARSING)? {
            Type::Keyword(Keyword::Super) => PathPart::SuperKw,
            Type::Keyword(Keyword::SelfKw) => PathPart::SelfKw,
            Type::Identifier => PathPart::Ident(self.identifier()?),
            t => return Err(self.error(Unexpected(t), PARSING)),
        };
        self.consume_peeked();
        Ok(out)
    }
    /// [Identifier] = [`Identifier`](Type::Identifier)
    pub fn identifier(&mut self) -> PResult<Identifier> {
        let tok = self.match_type(Type::Identifier, WhileParsing::Identifier)?;
        match tok.data() {
            Data::Identifier(ident) => Ok(ident.into()),
            _ => panic!("Expected token data for {tok:?}"),
        }
    }
}

/// # Statement parsing
impl<'t> Parser<'t> {
    /// Parses a [StmtKind]
    ///
    /// See also: [Parser::stmt]
    pub fn stmtkind(&mut self) -> PResult<StmtKind> {
        match self.peek_type(WhileParsing::StmtKind)? {
            Type::Semi => Ok(StmtKind::Empty),
            Type::Keyword(Keyword::Let) => self.stmtkind_local(),
            item_like!() => self.stmtkind_item(),
            _ => self.stmtkind_expr(),
        }
    }
    pub fn stmtkind_local(&mut self) -> PResult<StmtKind> {
        Ok(StmtKind::Local(self.parse_let()?))
    }
    pub fn stmtkind_item(&mut self) -> PResult<StmtKind> {
        Ok(StmtKind::Item(Box::new(self.item()?)))
    }
    pub fn stmtkind_expr(&mut self) -> PResult<StmtKind> {
        Ok(StmtKind::Expr(self.expr()?.into()))
    }

    pub fn parse_let(&mut self) -> PResult<Let> {
        self.match_kw(Keyword::Let, WhileParsing::Let)?;
        Ok(Let {
            mutable: self.mutability()?,
            name: self.identifier()?,
            init: if Type::Eq == self.peek_type(WhileParsing::Let)? {
                self.consume_peeked();
                Some(self.expr()?.into())
            } else {
                None
            },
        })
    }
}

macro binary($($name:ident {$lower:ident, $op:ident})*) {
    $(pub fn $name(&mut self) -> PResult<ExprKind> {
        let head = self.expr_from(Self::$lower)?;
        let mut tail = vec![];
        loop {
            match self.$op() {
                Ok(op) => tail.push((op, self.expr_from(Self::$lower)?)),
                Err(Error { reason: Unexpected(_), ..}) => break,
                Err(e) => Err(e)?,
            }
        }
        if tail.is_empty() {
            return Ok(head.kind);
        }
        Ok(Binary { head: head.into(), tail }.into())
    })*
}
/// # Expression parsing
impl<'t> Parser<'t> {
    /// Parses an [ExprKind]
    ///
    /// See also: [Parser::expr], [Parser::exprkind_primary]
    pub fn exprkind(&mut self) -> PResult<ExprKind> {
        self.exprkind_assign()
    }
    /// Creates an [Expr] with the given [ExprKind]-parser
    pub fn expr_from(&mut self, f: impl Fn(&mut Self) -> PResult<ExprKind>) -> PResult<Expr> {
        let start = self.loc();
        Ok(Expr { kind: f(self)?, extents: Span(start, self.loc()) })
    }
    pub fn optional_expr(&mut self) -> PResult<Option<Expr>> {
        match self.expr() {
            Ok(v) => Ok(Some(v)),
            Err(Error { reason: Nothing, .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// [Assign] = [Path] ([AssignKind] [Assign]) | [Compare](Binary)
    pub fn exprkind_assign(&mut self) -> PResult<ExprKind> {
        let head = self.expr_from(Self::exprkind_compare)?;
        if !matches!(head.kind, ExprKind::Path(_)) {
            return Ok(head.kind);
        }
        let Ok(op) = self.assign_op() else {
            return Ok(head.kind);
        };
        Ok(Assign { head, op, tail: self.expr_from(Self::exprkind_assign)?.into() }.into())
    }
    binary! {
        exprkind_compare {exprkind_range, compare_op}
        exprkind_range {exprkind_logic, range_op}
        exprkind_logic {exprkind_bitwise, logic_op}
        exprkind_bitwise {exprkind_shift, bitwise_op}
        exprkind_shift {exprkind_factor, shift_op}
        exprkind_factor {exprkind_term, factor_op}
        exprkind_term {exprkind_unary, term_op}
    }
    /// [Unary] = [UnaryKind]* [Member]
    pub fn exprkind_unary(&mut self) -> PResult<ExprKind> {
        let mut ops = vec![];
        loop {
            match self.unary_op() {
                Ok(v) => ops.push(v),
                Err(Error { reason: Unexpected(_), .. }) => break,
                Err(e) => Err(e)?,
            }
        }
        let tail = self.expr_from(Self::exprkind_member)?;
        if ops.is_empty() {
            return Ok(tail.kind);
        }
        Ok(Unary { ops, tail: Box::new(tail) }.into())
    }
    /// [Member] = [Call] `.` [Call]
    pub fn exprkind_member(&mut self) -> PResult<ExprKind> {
        let head = self.expr_from(Self::exprkind_call)?;
        let mut tail = vec![];
        while self.member_op().is_ok() {
            tail.push(self.expr_from(Self::exprkind_call)?)
        }
        if tail.is_empty() {
            Ok(head.kind)
        } else {
            Ok(Member { head: head.into(), tail }.into())
        }
    }
    /// Call = [Index] (`(` [Tuple]? `)`)*
    pub fn exprkind_call(&mut self) -> PResult<ExprKind> {
        const PARSING: WhileParsing = WhileParsing::Call;
        let callee = self.expr_from(Self::exprkind_index)?;
        let mut args = vec![];
        while Ok(Type::LParen) == self.peek_type(PARSING) {
            self.consume_peeked();
            args.push(self.tuple()?);
            self.match_type(Type::RParen, PARSING)?;
        }
        if args.is_empty() {
            Ok(callee.kind)
        } else {
            Ok(Call { callee: callee.into(), args }.into())
        }
    }
    /// [Index] = [Primary](Parser::exprkind_primary) (`[` [Indices] `]`)*
    pub fn exprkind_index(&mut self) -> PResult<ExprKind> {
        const PARSING: WhileParsing = WhileParsing::Index;
        let head = self.expr_from(Self::exprkind_primary)?;
        if Type::LBrack != self.peek_type(PARSING)? {
            return Ok(head.kind);
        }

        let mut indices = vec![];
        while Ok(Type::LBrack) == self.peek_type(PARSING) {
            self.consume_peeked();
            indices.push(self.tuple()?.into());
            self.match_type(Type::RBrack, PARSING)?;
        }
        Ok(Index { head: head.into(), indices }.into())
    }
    /// Delegates to the set of highest-priority rules based on unambiguous pattern matching
    pub fn exprkind_primary(&mut self) -> PResult<ExprKind> {
        match self.peek_type(WhileParsing::Expr)? {
            Type::Amp | Type::AmpAmp => self.exprkind_addrof(),
            Type::LCurly => self.exprkind_block(),
            Type::LBrack => self.exprkind_array(),
            Type::LParen => self.exprkind_empty_group_or_tuple(),
            literal_like!() => Ok(self.literal()?.into()),
            path_like!() => Ok(self.path()?.into()),
            Type::Keyword(Keyword::If) => Ok(self.parse_if()?.into()),
            Type::Keyword(Keyword::For) => Ok(self.parse_for()?.into()),
            Type::Keyword(Keyword::While) => Ok(self.parse_while()?.into()),
            Type::Keyword(Keyword::Break) => Ok(self.parse_break()?.into()),
            Type::Keyword(Keyword::Return) => Ok(self.parse_return()?.into()),
            Type::Keyword(Keyword::Continue) => Ok(self.parse_continue()?.into()),
            _ => Err(self.error(Nothing, WhileParsing::Expr)),
        }
    }
    /// [Array] = '[' ([Expr] ',')* [Expr]? ']'
    ///
    /// Array and ArrayRef are ambiguous until the second token,
    /// so they can't be independent subexpressions
    pub fn exprkind_array(&mut self) -> PResult<ExprKind> {
        const PARSING: WhileParsing = WhileParsing::Array;
        const START: Type = Type::LBrack;
        const END: Type = Type::RBrack;
        self.match_type(START, PARSING)?;
        match self.peek_type(PARSING)? {
            END => {
                self.consume_peeked();
                Ok(Array { values: vec![] }.into())
            }
            _ => self.exprkind_array_rep(),
        }
    }
    /// [ArrayRep] = `[` [Expr] `;` [Expr] `]`
    pub fn exprkind_array_rep(&mut self) -> PResult<ExprKind> {
        const PARSING: WhileParsing = WhileParsing::Array;
        const END: Type = Type::RBrack;
        let first = self.expr()?;
        let out: ExprKind = match self.peek_type(PARSING)? {
            Type::Semi => ArrayRep {
                value: first.into(),
                repeat: {
                    self.consume_peeked();
                    Box::new(self.expr()?)
                },
            }
            .into(),
            Type::RBrack => Array { values: vec![first] }.into(),
            Type::Comma => Array {
                values: {
                    self.consume_peeked();
                    let mut out = vec![first];
                    out.extend(self.separated(Type::Comma, Self::expr, Type::RBrack, PARSING)?);
                    out
                },
            }
            .into(),
            ty => Err(self.error(Unexpected(ty), PARSING))?,
        };
        self.match_type(END, PARSING)?;
        Ok(out)
    }

    /// [AddrOf] = (`&`|`&&`)* [Expr]
    pub fn exprkind_addrof(&mut self) -> PResult<ExprKind> {
        const PARSING: WhileParsing = WhileParsing::AddrOf;
        let mut count = 0;
        loop {
            match self.peek_type(PARSING)? {
                Type::Amp => count += 1,
                Type::AmpAmp => count += 2,
                _ => break,
            }
            self.consume_peeked();
        }
        Ok(AddrOf { count, mutable: self.mutability()?, expr: self.expr()?.into() }.into())
    }
    /// [Block] = `{` [Stmt]* `}`
    pub fn exprkind_block(&mut self) -> PResult<ExprKind> {
        self.block().map(Into::into)
    }
    /// [Group] = `(`([Empty](ExprKind::Empty)|[Expr]|[Tuple])`)`
    ///
    /// [ExprKind::Empty] and [Group] are special cases of [Tuple]
    pub fn exprkind_empty_group_or_tuple(&mut self) -> PResult<ExprKind> {
        self.match_type(Type::LParen, WhileParsing::Group)?;
        let out = match self.peek_type(WhileParsing::Group)? {
            Type::RParen => Ok(ExprKind::Empty),
            _ => self.exprkind_group(),
        };
        match self.peek_type(WhileParsing::Group) {
            Ok(Type::RParen) => self.consume_peeked(),
            _ => Err(self.error(UnmatchedParentheses, WhileParsing::Group))?,
        };
        out
    }
    /// [Group] = `(`([Empty](ExprKind::Empty)|[Expr]|[Tuple])`)`
    pub fn exprkind_group(&mut self) -> PResult<ExprKind> {
        let first = self.expr()?;
        match self.peek_type(WhileParsing::Group)? {
            Type::Comma => {
                let mut exprs = vec![first];
                self.consume_peeked();
                while Type::RParen != self.peek_type(WhileParsing::Tuple)? {
                    exprs.push(self.expr()?);
                    match self.peek_type(WhileParsing::Tuple)? {
                        Type::Comma => self.consume_peeked(),
                        _ => break,
                    };
                }
                Ok(Tuple { exprs }.into())
            }
            _ => Ok(Group { expr: first.into() }.into()),
        }
    }
}

/// ## Subexpressions
impl<'t> Parser<'t> {
    /// [Literal] = [String](Type::String) | [Character](Type::Character)
    /// | [Float](Type::Float) (TODO) | [Integer](Type::Integer) | `true` | `false`
    pub fn literal(&mut self) -> PResult<Literal> {
        let tok = self.consume(WhileParsing::Literal)?;
        // keyword literals true and false
        match tok.ty() {
            Type::Keyword(Keyword::True) => return Ok(Literal::Bool(true)),
            Type::Keyword(Keyword::False) => return Ok(Literal::Bool(false)),
            Type::String | Type::Character | Type::Integer | Type::Float => (),
            t => return Err(self.error(Unexpected(t), WhileParsing::Literal)),
        }
        Ok(match tok.data() {
            Data::String(v) => Literal::from(v.as_str()),
            Data::Character(v) => Literal::from(*v),
            Data::Integer(v) => Literal::from(*v),
            Data::Float(v) => todo!("Literal::Float({v})"),
            _ => panic!("Expected token data for {tok:?}"),
        })
    }
    /// [Tuple] = ([Expr] `,`)* [Expr]?
    pub fn tuple(&mut self) -> PResult<Tuple> {
        let mut exprs = vec![];
        while let Some(expr) = match self.expr() {
            Ok(v) => Some(v),
            Err(Error { reason: Nothing, .. }) => None,
            Err(e) => return Err(e),
        } {
            exprs.push(expr);
            match self.peek_type(WhileParsing::Tuple)? {
                Type::Comma => self.consume_peeked(),
                _ => break,
            };
        }
        Ok(Tuple { exprs })
    }
    /// [Block] = `{` [Stmt]* `}`
    pub fn block(&mut self) -> PResult<Block> {
        const PARSING: WhileParsing = WhileParsing::Block;
        const START: Type = Type::LCurly;
        const END: Type = Type::RCurly;
        Ok(Block {
            stmts: self.delimited(
                START,
                |this| this.repeated(Self::stmt, END, PARSING),
                END,
                PARSING,
            )?,
        })
    }
}
/// ## Control flow subexpressions
impl<'t> Parser<'t> {
    /// [Break] = `break` [Expr]?
    pub fn parse_break(&mut self) -> PResult<Break> {
        self.match_kw(Keyword::Break, WhileParsing::Break)?;
        Ok(Break { body: self.optional_expr()?.map(Into::into) })
    }
    /// [Return] = `return` [Expr]?
    pub fn parse_return(&mut self) -> PResult<Return> {
        self.match_kw(Keyword::Return, WhileParsing::Return)?;
        Ok(Return { body: self.optional_expr()?.map(Into::into) })
    }
    /// [Continue] = `continue`
    pub fn parse_continue(&mut self) -> PResult<Continue> {
        self.match_kw(Keyword::Continue, WhileParsing::Continue)?;
        Ok(Continue)
    }
    /// [While] = `while` [Expr] [Block] [Else]?
    pub fn parse_while(&mut self) -> PResult<While> {
        self.match_kw(Keyword::While, WhileParsing::While)?;
        Ok(While {
            cond: self.expr()?.into(),
            pass: self.block()?.into(),
            fail: self.parse_else()?,
        })
    }
    /// [If] = <code>`if` [Expr] [Block] [Else]?</code>
    #[rustfmt::skip] // second line is barely not long enough
    pub fn parse_if(&mut self) -> PResult<If> {
        self.match_kw(Keyword::If, WhileParsing::If)?;
        Ok(If {
            cond: self.expr()?.into(),
            pass: self.block()?.into(),
            fail: self.parse_else()?,
        })
    }
    /// [For]: `for` Pattern (TODO) `in` [Expr] [Block] [Else]?
    pub fn parse_for(&mut self) -> PResult<For> {
        self.match_kw(Keyword::For, WhileParsing::For)?;
        let bind = self.identifier()?;
        self.match_kw(Keyword::In, WhileParsing::For)?;
        Ok(For {
            bind,
            cond: self.expr()?.into(),
            pass: self.block()?.into(),
            fail: self.parse_else()?,
        })
    }
    /// [Else]: (`else` [Block])?
    pub fn parse_else(&mut self) -> PResult<Else> {
        match self.peek_type(WhileParsing::Else) {
            Ok(Type::Keyword(Keyword::Else)) => {
                self.consume_peeked();
                Ok(self.expr()?.into())
            }
            Ok(_) | Err(Error { reason: EndOfInput, .. }) => Ok(None.into()),
            Err(e) => Err(e),
        }
    }
}

macro operator($($name:ident ($returns:ident) {$($t:ident => $p:ident),*$(,)?};)*) {$(
    pub fn $name (&mut self) -> PResult<$returns> {
        const PARSING: WhileParsing = WhileParsing::$returns;
        let out = Ok(match self.peek_type(PARSING) {
            $(Ok(Type::$t) => $returns::$p,)*
            Err(e) => Err(e)?,
            Ok(t) => Err(self.error(Unexpected(t), PARSING))?,
        });
        self.consume_peeked();
        out
    }
)*}

/// ## Operator Kinds
impl<'t> Parser<'t> {
    operator! {
        assign_op (AssignKind) {
            Eq => Plain,    // =
            AmpEq => And,   // &=
            BarEq => Or,    // |=
            XorEq => Xor,   // ^=
            LtLtEq => Shl,  // <<=
            GtGtEq => Shr,  // >>=
            PlusEq => Add,  // +=
            MinusEq => Sub, // -=
            StarEq => Mul,  // *=
            SlashEq => Div, // /=
            RemEq => Rem,   // %=
        };
        compare_op (BinaryKind) {
            Lt => Lt,       // <
            LtEq => LtEq,   // <=
            EqEq => Equal,  // ==
            BangEq => NotEq,// !=
            GtEq => GtEq,   // >=
            Gt => Gt,       // >
        };
        range_op (BinaryKind) {
            DotDot => RangeExc,  // ..
            DotDotEq => RangeInc,// ..=
        };
        logic_op (BinaryKind) {
            AmpAmp => LogAnd,   // &&
            BarBar => LogOr,    // ||
            XorXor => LogXor,   // ^^
        };
        bitwise_op (BinaryKind) {
            Amp => BitAnd,  // &
            Bar => BitOr,   // |
            Xor => BitXor,  // ^
        };
        shift_op (BinaryKind) {
            LtLt => Shl,    // <<
            GtGt => Shr,    // >>
        };
        factor_op (BinaryKind) {
            Plus => Add,    // +
            Minus => Sub,   // -
        };
        term_op (BinaryKind) {
            Star => Mul,    // *
            Slash => Div,   // /
            Rem => Rem,     // %
        };
        unary_op (UnaryKind) {
            Star => Deref,  // *
            Minus => Neg,   // -
            Bang => Not,    // !
            At => At,       // @
            Hash => Hash,   // #
            Tilde => Tilde, // ~
        };
    }
    pub fn member_op(&mut self) -> PResult<()> {
        const PARSING: WhileParsing = WhileParsing::Member;
        match self.peek(PARSING)?.ty() {
            Type::Dot => {}
            t => Err(self.error(Unexpected(t), PARSING))?,
        }
        self.consume_peeked();
        Ok(())
    }
}
