use super::*;
use crate::error::{
    Error,
    ErrorKind::{self, *},
    PResult, Parsing,
};
use cl_ast::*;
use cl_lexer::Lexer;
use cl_token::token_type::Op;

/// Parses a sequence of [Tokens](Token) into an [AST](cl_ast)
pub struct Parser<'t> {
    /// Lazy tokenizer
    lexer: Lexer<'t>,
    /// Look-ahead buffer
    next: Option<Token>,
    /// The location of the current token
    loc: Loc,
}

/// Basic parser functionality
impl<'t> Parser<'t> {
    pub fn new(lexer: Lexer<'t>) -> Self {
        Self { loc: Loc::from(&lexer), lexer, next: None }
    }
    /// Gets the location of the last consumed [Token]
    pub fn loc(&self) -> Loc {
        self.loc
    }

    /// Constructs an [Error]
    fn error(&self, reason: ErrorKind, while_parsing: Parsing) -> Error {
        Error { reason, while_parsing, loc: self.loc }
    }
    /// Internal impl of peek and consume
    fn consume_from_lexer(&mut self, while_parsing: Parsing) -> PResult<Token> {
        loop {
            match self
                .lexer
                .scan()
                .map_err(|e| self.error(e.into(), while_parsing))?
            {
                t if t.ty() == TokenKind::Invalid => continue,
                t if t.ty() == TokenKind::Comment => continue,
                t => break Ok(t),
            }
        }
    }
    /// Looks ahead one token
    ///
    /// Stores the token in an internal lookahead buffer
    pub fn peek(&mut self, while_parsing: Parsing) -> PResult<&Token> {
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
    /// Looks ahead at the next [Token]'s [TokenKind]
    pub fn peek_kind(&mut self, while_parsing: Parsing) -> PResult<TokenKind> {
        self.peek(while_parsing).map(|t| t.ty())
    }
    /// Consumes one [Token]
    pub fn consume(&mut self, while_parsing: Parsing) -> PResult<Token> {
        self.loc = Loc::from(&self.lexer);
        match self.next.take() {
            Some(token) => Ok(token),
            None => self.consume_from_lexer(while_parsing),
        }
    }
    /// Consumes the next [Token] if it matches the pattern [TokenKind]
    pub fn match_type(&mut self, want: TokenKind, while_parsing: Parsing) -> PResult<Token> {
        let got = self.peek_kind(while_parsing)?;
        if got == want {
            Ok(self.consume_peeked().expect("should not fail after peek"))
        } else {
            Err(self.error(Expected { want, got }, while_parsing))
        }
    }
    #[inline]
    pub fn match_op(&mut self, want: Op, while_parsing: Parsing) -> PResult<Token> {
        self.match_type(TokenKind::Op(want), while_parsing)
    }
}

// the three matched delimiter pairs
/// Square brackets: `[` `]`
const BRACKETS: (TokenKind, TokenKind) = (TokenKind::Op(Op::LBrack), TokenKind::Op(Op::RBrack));
/// Curly braces: `{` `}`
const CURLIES: (TokenKind, TokenKind) = (TokenKind::Op(Op::LCurly), TokenKind::Op(Op::RCurly));
/// Parentheses: `(` `)`
const PARENS: (TokenKind, TokenKind) = (TokenKind::Op(Op::LParen), TokenKind::Op(Op::RParen));

/// Parses constructions of the form `delim.0 f delim.1` (i.e. `(` `foobar` `)`)
const fn delim<'t, T>(
    f: impl Fn(&mut Parser<'t>) -> PResult<T>,
    delim: (TokenKind, TokenKind),
    while_parsing: Parsing,
) -> impl Fn(&mut Parser<'t>) -> PResult<T> {
    move |parser| {
        parser.match_type(delim.0, while_parsing)?;
        let out = f(parser)?;
        parser.match_type(delim.1, while_parsing)?;
        Ok(out)
    }
}

/// Parses constructions of the form `(f sep ~until)*`
///
/// where `~until` is a negative lookahead assertion
const fn sep<'t, T>(
    f: impl Fn(&mut Parser<'t>) -> PResult<T>,
    sep: TokenKind,
    until: TokenKind,
    while_parsing: Parsing,
) -> impl Fn(&mut Parser<'t>) -> PResult<Vec<T>> {
    move |parser| {
        let mut args = vec![];
        while until != parser.peek_kind(while_parsing)? {
            args.push(f(parser)?);
            if sep != parser.peek_kind(while_parsing)? {
                break;
            }
            parser.consume_peeked();
        }
        Ok(args)
    }
}

/// Parses constructions of the form `(f ~until)*`
///
/// where `~until` is a negative lookahead assertion
#[allow(dead_code)]
const fn rep<'t, T>(
    f: impl Fn(&mut Parser<'t>) -> PResult<T>,
    until: TokenKind,
    while_parsing: Parsing,
) -> impl Fn(&mut Parser<'t>) -> PResult<Vec<T>> {
    move |parser| {
        let mut out = vec![];
        while until != parser.peek_kind(while_parsing)? {
            out.push(f(parser)?)
        }
        Ok(out)
    }
}

/// Expands to a pattern which matches item-like [Token] [TokenKind]s
macro item_like() {
    TokenKind::Op(Op::Hash)
        | TokenKind::Pub
        | TokenKind::Type
        | TokenKind::Const
        | TokenKind::Static
        | TokenKind::Mod
        | TokenKind::Fn
        | TokenKind::Struct
        | TokenKind::Enum
        | TokenKind::Impl
}

/// Top level parsing
impl<'t> Parser<'t> {
    /// Parses a [File]
    pub fn file(&mut self) -> PResult<File> {
        let mut items = vec![];
        while match self.peek_kind(Parsing::File) {
            Ok(TokenKind::Op(Op::RCurly)) | Err(Error { reason: EndOfInput, .. }) => false,
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
            attrs: self.attributes()?,
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
        const PARSING: Parsing = Parsing::PathExpr;
        let absolute = matches!(self.peek_kind(PARSING)?, TokenKind::Op(Op::ColonColon));
        if absolute {
            self.consume_peeked();
        }

        let mut parts = vec![self.path_part()?];
        while let Ok(TokenKind::Op(Op::ColonColon)) = self.peek_kind(PARSING) {
            self.consume_peeked();
            parts.push(self.path_part()?);
        }
        Ok(Path { absolute, parts })
    }

    /// Parses a [Stmt]
    ///
    /// See also: [Parser::stmtkind]
    pub fn stmt(&mut self) -> PResult<Stmt> {
        const PARSING: Parsing = Parsing::Stmt;
        let start = self.loc();
        Ok(Stmt {
            kind: self.stmtkind()?,
            semi: match self.peek_kind(PARSING) {
                Ok(TokenKind::Op(Op::Semi)) => {
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

/// Attribute parsing
impl<'t> Parser<'t> {
    /// Parses an [attribute set](Attrs)
    pub fn attributes(&mut self) -> PResult<Attrs> {
        if self.match_op(Op::Hash, Parsing::Attrs).is_err() {
            return Ok(Attrs { meta: vec![] });
        }
        let meta = delim(
            sep(
                Self::meta,
                TokenKind::Op(Op::Comma),
                BRACKETS.1,
                Parsing::Attrs,
            ),
            BRACKETS,
            Parsing::Attrs,
        );
        Ok(Attrs { meta: meta(self)? })
    }
    pub fn meta(&mut self) -> PResult<Meta> {
        Ok(Meta { name: self.identifier()?, kind: self.meta_kind()? })
    }
    pub fn meta_kind(&mut self) -> PResult<MetaKind> {
        const PARSING: Parsing = Parsing::Meta;
        let lit_tuple = delim(
            sep(Self::literal, TokenKind::Op(Op::Comma), PARENS.1, PARSING),
            PARENS,
            PARSING,
        );
        Ok(match self.peek_kind(PARSING) {
            Ok(TokenKind::Op(Op::Eq)) => {
                self.consume_peeked();
                MetaKind::Equals(self.literal()?)
            }
            Ok(TokenKind::Op(Op::LParen)) => MetaKind::Func(lit_tuple(self)?),
            _ => MetaKind::Plain,
        })
    }
}

/// Item parsing
impl<'t> Parser<'t> {
    /// Parses an [ItemKind]
    ///
    /// See also: [Parser::item]
    pub fn itemkind(&mut self) -> PResult<ItemKind> {
        Ok(match self.peek_kind(Parsing::Item)? {
            TokenKind::Type => self.parse_alias()?.into(),
            TokenKind::Const => self.parse_const()?.into(),
            TokenKind::Static => self.parse_static()?.into(),
            TokenKind::Mod => self.parse_module()?.into(),
            TokenKind::Fn => self.parse_function()?.into(),
            TokenKind::Struct => self.parse_struct()?.into(),
            TokenKind::Enum => self.parse_enum()?.into(),
            TokenKind::Impl => self.parse_impl()?.into(),
            t => Err(self.error(Unexpected(t), Parsing::Item))?,
        })
    }

    pub fn parse_alias(&mut self) -> PResult<Alias> {
        const PARSING: Parsing = Parsing::Alias;
        self.match_type(TokenKind::Type, PARSING)?;
        let out = Ok(Alias {
            to: self.identifier()?,
            from: if self.match_op(Op::Eq, PARSING).is_ok() {
                Some(self.ty()?.into())
            } else {
                None
            },
        });
        self.match_op(Op::Semi, PARSING)?;
        out
    }

    pub fn parse_const(&mut self) -> PResult<Const> {
        const PARSING: Parsing = Parsing::Const;
        self.match_type(TokenKind::Const, PARSING)?;
        let out = Ok(Const {
            name: self.identifier()?,
            ty: {
                self.match_op(Op::Colon, PARSING)?;
                self.ty()?.into()
            },
            init: {
                self.match_op(Op::Eq, PARSING)?;
                self.expr()?.into()
            },
        });
        self.match_op(Op::Semi, PARSING)?;
        out
    }
    pub fn parse_static(&mut self) -> PResult<Static> {
        const PARSING: Parsing = Parsing::Static;
        self.match_type(TokenKind::Static, PARSING)?;
        let out = Ok(Static {
            mutable: self.mutability()?,
            name: self.identifier()?,
            ty: {
                self.match_op(Op::Colon, PARSING)?;
                self.ty()?.into()
            },
            init: {
                self.match_op(Op::Eq, PARSING)?;
                self.expr()?.into()
            },
        });
        self.match_op(Op::Semi, PARSING)?;
        out
    }
    pub fn parse_module(&mut self) -> PResult<Module> {
        const PARSING: Parsing = Parsing::Module;
        self.match_type(TokenKind::Mod, PARSING)?;
        Ok(Module { name: self.identifier()?, kind: self.modulekind()? })
    }
    pub fn modulekind(&mut self) -> PResult<ModuleKind> {
        const PARSING: Parsing = Parsing::ModuleKind;
        let inline = delim(Self::file, CURLIES, PARSING);

        match self.peek_kind(PARSING)? {
            TokenKind::Op(Op::LCurly) => Ok(ModuleKind::Inline(inline(self)?)),
            TokenKind::Op(Op::Semi) => {
                self.consume_peeked();
                Ok(ModuleKind::Outline)
            }
            got => Err(self.error(Expected { want: TokenKind::Op(Op::Semi), got }, PARSING)),
        }
    }
    pub fn parse_function(&mut self) -> PResult<Function> {
        const PARSING: Parsing = Parsing::Function;
        self.match_type(TokenKind::Fn, PARSING)?;
        Ok(Function {
            name: self.identifier()?,
            args: self.parse_params()?,
            rety: match self.peek_kind(PARSING)? {
                TokenKind::Op(Op::LCurly) | TokenKind::Op(Op::Semi) => None,
                TokenKind::Op(Op::Arrow) => {
                    self.consume_peeked();
                    Some(self.ty()?.into())
                }
                got => Err(self.error(Expected { want: TokenKind::Op(Op::Arrow), got }, PARSING))?,
            },
            body: match self.peek_kind(PARSING)? {
                TokenKind::Op(Op::LCurly) => Some(self.block()?),
                TokenKind::Op(Op::Semi) => {
                    self.consume_peeked();
                    None
                }
                t => Err(self.error(Unexpected(t), PARSING))?,
            },
        })
    }
    pub fn parse_params(&mut self) -> PResult<Vec<Param>> {
        const PARSING: Parsing = Parsing::Function;
        delim(
            sep(
                Self::parse_param,
                TokenKind::Op(Op::Comma),
                PARENS.1,
                PARSING,
            ),
            PARENS,
            PARSING,
        )(self)
    }
    pub fn parse_param(&mut self) -> PResult<Param> {
        Ok(Param {
            mutability: self.mutability()?,
            name: self.identifier()?,
            ty: {
                self.match_op(Op::Colon, Parsing::Param)?;
                self.ty()?.into()
            },
        })
    }
    pub fn parse_struct(&mut self) -> PResult<Struct> {
        const PARSING: Parsing = Parsing::Struct;
        self.match_type(TokenKind::Struct, PARSING)?;
        Ok(Struct {
            name: self.identifier()?,
            kind: match self.peek_kind(PARSING)? {
                TokenKind::Op(Op::LParen) => self.structkind_tuple()?,
                TokenKind::Op(Op::LCurly) => self.structkind_struct()?,
                TokenKind::Op(Op::Semi) => {
                    self.consume_peeked();
                    StructKind::Empty
                }
                got => Err(self.error(Expected { want: TokenKind::Op(Op::Semi), got }, PARSING))?,
            },
        })
    }
    pub fn structkind_tuple(&mut self) -> PResult<StructKind> {
        const PARSING: Parsing = Parsing::StructKind;

        Ok(StructKind::Tuple(delim(
            sep(Self::ty, TokenKind::Op(Op::Comma), PARENS.1, PARSING),
            PARENS,
            PARSING,
        )(self)?))
    }
    pub fn structkind_struct(&mut self) -> PResult<StructKind> {
        const PARSING: Parsing = Parsing::StructKind;
        Ok(StructKind::Struct(delim(
            sep(
                Self::struct_member,
                TokenKind::Op(Op::Comma),
                CURLIES.1,
                PARSING,
            ),
            CURLIES,
            PARSING,
        )(self)?))
    }
    pub fn struct_member(&mut self) -> PResult<StructMember> {
        const PARSING: Parsing = Parsing::StructMember;
        Ok(StructMember {
            vis: self.visibility()?,
            name: self.identifier()?,
            ty: {
                self.match_op(Op::Colon, PARSING)?;
                self.ty()?
            },
        })
    }
    pub fn parse_enum(&mut self) -> PResult<Enum> {
        // Enum        = "enum" Identifier '{' (Variant ',')* Variant? '}' ;
        const PARSING: Parsing = Parsing::Enum;
        self.match_type(TokenKind::Enum, PARSING)?;
        Ok(Enum {
            name: self.identifier()?,
            kind: match self.peek_kind(PARSING)? {
                TokenKind::Op(Op::LCurly) => EnumKind::Variants(delim(
                    sep(
                        Self::enum_variant,
                        TokenKind::Op(Op::Comma),
                        TokenKind::Op(Op::RCurly),
                        PARSING,
                    ),
                    CURLIES,
                    PARSING,
                )(self)?),
                TokenKind::Op(Op::Semi) => {
                    self.consume_peeked();
                    EnumKind::NoVariants
                }
                t => Err(self.error(Unexpected(t), PARSING))?,
            },
        })
    }

    pub fn enum_variant(&mut self) -> PResult<Variant> {
        const PARSING: Parsing = Parsing::Variant;
        Ok(Variant {
            name: self.identifier()?,
            kind: match self.peek_kind(PARSING)? {
                TokenKind::Op(Op::Eq) => self.variantkind_clike()?,
                TokenKind::Op(Op::LCurly) => self.variantkind_struct()?,
                TokenKind::Op(Op::LParen) => self.variantkind_tuple()?,
                _ => VariantKind::Plain,
            },
        })
    }
    pub fn variantkind_clike(&mut self) -> PResult<VariantKind> {
        const PARSING: Parsing = Parsing::VariantKind;
        self.match_op(Op::Eq, PARSING)?;
        let tok = self.match_type(TokenKind::Integer, PARSING)?;
        Ok(VariantKind::CLike(match tok.data() {
            TokenData::Integer(i) => *i,
            _ => panic!("Expected token data for {tok:?} while parsing {PARSING}"),
        }))
    }
    pub fn variantkind_struct(&mut self) -> PResult<VariantKind> {
        const PARSING: Parsing = Parsing::VariantKind;
        Ok(VariantKind::Struct(delim(
            sep(
                Self::struct_member,
                TokenKind::Op(Op::Comma),
                TokenKind::Op(Op::RCurly),
                PARSING,
            ),
            CURLIES,
            PARSING,
        )(self)?))
    }
    pub fn variantkind_tuple(&mut self) -> PResult<VariantKind> {
        const PARSING: Parsing = Parsing::VariantKind;
        Ok(VariantKind::Tuple(delim(
            sep(
                Self::ty,
                TokenKind::Op(Op::Comma),
                TokenKind::Op(Op::RParen),
                PARSING,
            ),
            PARENS,
            PARSING,
        )(self)?))
    }

    pub fn parse_impl(&mut self) -> PResult<Impl> {
        const PARSING: Parsing = Parsing::Impl;
        self.match_type(TokenKind::Impl, PARSING)?;
        Err(self.error(Todo, PARSING))
    }

    pub fn visibility(&mut self) -> PResult<Visibility> {
        if let TokenKind::Pub = self.peek_kind(Parsing::Visibility)? {
            self.consume_peeked();
            return Ok(Visibility::Public);
        };
        Ok(Visibility::Private)
    }
    pub fn mutability(&mut self) -> PResult<Mutability> {
        if let TokenKind::Mut = self.peek_kind(Parsing::Mutability)? {
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
        const PARSING: Parsing = Parsing::TyKind;
        let out = match self.peek_kind(PARSING)? {
            TokenKind::Op(Op::Bang) => {
                self.consume_peeked();
                TyKind::Never
            }
            TokenKind::SelfTy => {
                self.consume_peeked();
                TyKind::SelfTy
            }
            TokenKind::Op(Op::Amp) | TokenKind::Op(Op::AmpAmp) => self.tyref()?.into(),
            TokenKind::Op(Op::LParen) => self.tytuple()?.into(),
            TokenKind::Fn => self.tyfn()?.into(),
            path_like!() => self.path()?.into(),
            t => Err(self.error(Unexpected(t), PARSING))?,
        };

        Ok(out)
    }
    /// [TyTuple] = `(` ([Ty] `,`)* [Ty]? `)`
    pub fn tytuple(&mut self) -> PResult<TyTuple> {
        const PARSING: Parsing = Parsing::TyTuple;
        Ok(TyTuple {
            types: delim(
                sep(Self::ty, TokenKind::Op(Op::Comma), PARENS.1, PARSING),
                PARENS,
                PARSING,
            )(self)?,
        })
    }
    /// [TyRef] = (`&`|`&&`)* [Path]
    pub fn tyref(&mut self) -> PResult<TyRef> {
        const PARSING: Parsing = Parsing::TyRef;
        let mut count = 0;
        loop {
            match self.peek_kind(PARSING)? {
                TokenKind::Op(Op::Amp) => count += 1,
                TokenKind::Op(Op::AmpAmp) => count += 2,
                _ => break,
            }
            self.consume_peeked();
        }
        Ok(TyRef { count, to: self.path()? })
    }
    /// [TyFn] = `fn` [TyTuple] (-> [Ty])?
    pub fn tyfn(&mut self) -> PResult<TyFn> {
        const PARSING: Parsing = Parsing::TyFn;
        self.match_type(TokenKind::Fn, PARSING)?;
        Ok(TyFn {
            args: self.tytuple()?,
            rety: {
                match self.peek_kind(PARSING)? {
                    TokenKind::Op(Op::Arrow) => {
                        self.consume_peeked();
                        Some(self.ty()?.into())
                    }
                    _ => None,
                }
            },
        })
    }
}

/// Expands to a pattern which matches literal-like [TokenKind]s
macro literal_like() {
    TokenKind::True
        | TokenKind::False
        | TokenKind::String
        | TokenKind::Character
        | TokenKind::Integer
        | TokenKind::Float
}
/// Expands to a pattern which matches path-like [token Types](Type)
macro path_like() {
    TokenKind::Super | TokenKind::SelfKw | TokenKind::Identifier | TokenKind::Op(Op::ColonColon)
}
/// # Path parsing
impl<'t> Parser<'t> {
    /// [PathPart] = `super` | `self` | [Identifier]
    pub fn path_part(&mut self) -> PResult<PathPart> {
        const PARSING: Parsing = Parsing::PathPart;
        let out = match self.peek_kind(PARSING)? {
            TokenKind::Super => PathPart::SuperKw,
            TokenKind::SelfKw => PathPart::SelfKw,
            TokenKind::Identifier => PathPart::Ident(self.identifier()?),
            t => return Err(self.error(Unexpected(t), PARSING)),
        };
        self.consume_peeked();
        Ok(out)
    }
    /// [Identifier] = [`Identifier`](TokenKind::Identifier)
    pub fn identifier(&mut self) -> PResult<Identifier> {
        let tok = self.match_type(TokenKind::Identifier, Parsing::Identifier)?;
        match tok.data() {
            TokenData::Identifier(ident) => Ok(ident.into()),
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
        Ok(match self.peek_kind(Parsing::StmtKind)? {
            TokenKind::Op(Op::Semi) => StmtKind::Empty,
            TokenKind::Let => self.parse_let()?.into(),
            item_like!() => self.item()?.into(),
            _ => self.expr()?.into(),
        })
    }

    pub fn parse_let(&mut self) -> PResult<Let> {
        self.match_type(TokenKind::Let, Parsing::Let)?;
        Ok(Let {
            mutable: self.mutability()?,
            name: self.identifier()?,
            ty: if Ok(TokenKind::Op(Op::Colon)) == self.peek_kind(Parsing::Let) {
                self.consume_peeked();
                Some(self.ty()?.into())
            } else {
                None
            },
            init: if Ok(TokenKind::Op(Op::Eq)) == self.peek_kind(Parsing::Let) {
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
            Err(Error { reason: Unexpected(_) | EndOfInput, ..}) => break,
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
        // TODO: Formalize the concept of a "place expression"
        if !matches!(
            head.kind,
            ExprKind::Path(_) | ExprKind::Call(_) | ExprKind::Member(_) | ExprKind::Index(_)
        ) {
            return Ok(head.kind);
        }
        let Ok(op) = self.assign_op() else {
            return Ok(head.kind);
        };
        Ok(
            Assign {
                head: Box::new(head),
                op,
                tail: self.expr_from(Self::exprkind_assign)?.into(),
            }
            .into(),
        )
    }
    // TODO: use a pratt parser for binary expressions, to simplify this
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
        const PARSING: Parsing = Parsing::Call;
        let callee = self.expr_from(Self::exprkind_index)?;
        let mut args = vec![];
        while Ok(TokenKind::Op(Op::LParen)) == self.peek_kind(PARSING) {
            self.consume_peeked();
            args.push(self.tuple()?);
            self.match_op(Op::RParen, PARSING)?;
        }
        if args.is_empty() {
            Ok(callee.kind)
        } else {
            Ok(Call { callee: callee.into(), args }.into())
        }
    }
    /// [Index] = [Primary](Parser::exprkind_primary) (`[` [Indices] `]`)*
    pub fn exprkind_index(&mut self) -> PResult<ExprKind> {
        const PARSING: Parsing = Parsing::Index;
        let head = self.expr_from(Self::exprkind_primary)?;
        if Ok(TokenKind::Op(Op::LBrack)) != self.peek_kind(PARSING) {
            return Ok(head.kind);
        }

        let mut indices = vec![];
        while Ok(TokenKind::Op(Op::LBrack)) == self.peek_kind(PARSING) {
            indices.push(delim(Self::tuple, BRACKETS, PARSING)(self)?.into());
        }
        Ok(Index { head: head.into(), indices }.into())
    }
    /// Delegates to the set of highest-priority rules based on unambiguous pattern matching
    pub fn exprkind_primary(&mut self) -> PResult<ExprKind> {
        match self.peek_kind(Parsing::Expr)? {
            TokenKind::Op(Op::Amp) | TokenKind::Op(Op::AmpAmp) => self.exprkind_addrof(),
            TokenKind::Op(Op::LCurly) => self.exprkind_block(),
            TokenKind::Op(Op::LBrack) => self.exprkind_array(),
            TokenKind::Op(Op::LParen) => self.exprkind_empty_group_or_tuple(),
            literal_like!() => Ok(self.literal()?.into()),
            path_like!() => Ok(self.path()?.into()),
            TokenKind::If => Ok(self.parse_if()?.into()),
            TokenKind::For => Ok(self.parse_for()?.into()),
            TokenKind::While => Ok(self.parse_while()?.into()),
            TokenKind::Break => Ok(self.parse_break()?.into()),
            TokenKind::Return => Ok(self.parse_return()?.into()),
            TokenKind::Continue => Ok(self.parse_continue()?.into()),
            _ => Err(self.error(Nothing, Parsing::Expr)),
        }
    }
    /// [Array] = '[' ([Expr] ',')* [Expr]? ']'
    ///
    /// Array and ArrayRef are ambiguous until the second token,
    /// so they can't be independent subexpressions
    pub fn exprkind_array(&mut self) -> PResult<ExprKind> {
        const PARSING: Parsing = Parsing::Array;
        const START: TokenKind = TokenKind::Op(Op::LBrack);
        const END: TokenKind = TokenKind::Op(Op::RBrack);
        self.match_type(START, PARSING)?;
        match self.peek_kind(PARSING)? {
            END => {
                self.consume_peeked();
                Ok(Array { values: vec![] }.into())
            }
            _ => self.exprkind_array_rep(),
        }
    }
    /// [ArrayRep] = `[` [Expr] `;` [Expr] `]`
    pub fn exprkind_array_rep(&mut self) -> PResult<ExprKind> {
        const PARSING: Parsing = Parsing::Array;
        const END: TokenKind = TokenKind::Op(Op::RBrack);
        let first = self.expr()?;
        let out: ExprKind = match self.peek_kind(PARSING)? {
            TokenKind::Op(Op::Semi) => ArrayRep {
                value: first.into(),
                repeat: {
                    self.consume_peeked();
                    Box::new(self.expr()?)
                },
            }
            .into(),
            TokenKind::Op(Op::RBrack) => Array { values: vec![first] }.into(),
            TokenKind::Op(Op::Comma) => Array {
                values: {
                    self.consume_peeked();
                    let mut out = vec![first];
                    out.extend(sep(Self::expr, TokenKind::Op(Op::Comma), END, PARSING)(
                        self,
                    )?);
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
        const PARSING: Parsing = Parsing::AddrOf;
        let mut count = 0;
        loop {
            match self.peek_kind(PARSING)? {
                TokenKind::Op(Op::Amp) => count += 1,
                TokenKind::Op(Op::AmpAmp) => count += 2,
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
        self.match_op(Op::LParen, Parsing::Group)?;
        let out = match self.peek_kind(Parsing::Group)? {
            TokenKind::Op(Op::RParen) => Ok(ExprKind::Empty),
            _ => self.exprkind_group(),
        };
        match self.peek_kind(Parsing::Group) {
            Ok(TokenKind::Op(Op::RParen)) => self.consume_peeked(),
            _ => Err(self.error(UnmatchedParentheses, Parsing::Group))?,
        };
        out
    }
    /// [Group] = `(`([Empty](ExprKind::Empty)|[Expr]|[Tuple])`)`
    pub fn exprkind_group(&mut self) -> PResult<ExprKind> {
        let first = self.expr()?;
        match self.peek_kind(Parsing::Group)? {
            TokenKind::Op(Op::Comma) => {
                let mut exprs = vec![first];
                self.consume_peeked();
                while TokenKind::Op(Op::RParen) != self.peek_kind(Parsing::Tuple)? {
                    exprs.push(self.expr()?);
                    match self.peek_kind(Parsing::Tuple)? {
                        TokenKind::Op(Op::Comma) => self.consume_peeked(),
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
    /// [Literal] = [String](TokenKind::String) | [Character](TokenKind::Character)
    /// | [Float](TokenKind::Float) (TODO) | [Integer](TokenKind::Integer) | `true` | `false`
    pub fn literal(&mut self) -> PResult<Literal> {
        let tok = self.consume(Parsing::Literal)?;
        // keyword literals true and false
        match tok.ty() {
            TokenKind::True => return Ok(Literal::Bool(true)),
            TokenKind::False => return Ok(Literal::Bool(false)),
            TokenKind::String | TokenKind::Character | TokenKind::Integer | TokenKind::Float => (),
            t => return Err(self.error(Unexpected(t), Parsing::Literal)),
        }
        Ok(match tok.data() {
            TokenData::String(v) => Literal::from(v.as_str()),
            TokenData::Character(v) => Literal::from(*v),
            TokenData::Integer(v) => Literal::from(*v),
            TokenData::Float(v) => todo!("Literal::Float({v})"),
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
            match self.peek_kind(Parsing::Tuple)? {
                TokenKind::Op(Op::Comma) => self.consume_peeked(),
                _ => break,
            };
        }
        Ok(Tuple { exprs })
    }
    /// [Block] = `{` [Stmt]* `}`
    pub fn block(&mut self) -> PResult<Block> {
        const PARSING: Parsing = Parsing::Block;
        Ok(Block { stmts: delim(rep(Self::stmt, CURLIES.1, PARSING), CURLIES, PARSING)(self)? })
    }
}
/// ## Control flow subexpressions
impl<'t> Parser<'t> {
    /// [Break] = `break` [Expr]?
    pub fn parse_break(&mut self) -> PResult<Break> {
        self.match_type(TokenKind::Break, Parsing::Break)?;
        Ok(Break { body: self.optional_expr()?.map(Into::into) })
    }
    /// [Return] = `return` [Expr]?
    pub fn parse_return(&mut self) -> PResult<Return> {
        self.match_type(TokenKind::Return, Parsing::Return)?;
        Ok(Return { body: self.optional_expr()?.map(Into::into) })
    }
    /// [Continue] = `continue`
    pub fn parse_continue(&mut self) -> PResult<Continue> {
        self.match_type(TokenKind::Continue, Parsing::Continue)?;
        Ok(Continue)
    }
    /// [While] = `while` [Expr] [Block] [Else]?
    pub fn parse_while(&mut self) -> PResult<While> {
        self.match_type(TokenKind::While, Parsing::While)?;
        Ok(While {
            cond: self.expr()?.into(),
            pass: self.block()?.into(),
            fail: self.parse_else()?,
        })
    }
    /// [If] = <code>`if` [Expr] [Block] [Else]?</code>
    #[rustfmt::skip] // second line is barely not long enough
    pub fn parse_if(&mut self) -> PResult<If> {
    self.match_type(TokenKind::If, Parsing::If)?;
    Ok(If {
        cond: self.expr()?.into(),
        pass: self.block()?.into(),
        fail: self.parse_else()?,
    })
}
    /// [For]: `for` Pattern (TODO) `in` [Expr] [Block] [Else]?
    pub fn parse_for(&mut self) -> PResult<For> {
        self.match_type(TokenKind::For, Parsing::For)?;
        let bind = self.identifier()?;
        self.match_type(TokenKind::In, Parsing::For)?;
        Ok(For {
            bind,
            cond: self.expr()?.into(),
            pass: self.block()?.into(),
            fail: self.parse_else()?,
        })
    }
    /// [Else]: (`else` [Block])?
    pub fn parse_else(&mut self) -> PResult<Else> {
        match self.peek_kind(Parsing::Else) {
            Ok(TokenKind::Else) => {
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
    const PARSING: Parsing = Parsing::$returns;
    let out = Ok(match self.peek_kind(PARSING) {
        $(Ok(TokenKind::Op(Op::$t)) => $returns::$p,)*
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
            Tilde => Tilde, // ~
        };
    }
    pub fn member_op(&mut self) -> PResult<()> {
        const PARSING: Parsing = Parsing::Member;
        match self.peek(PARSING)?.ty() {
            TokenKind::Op(Op::Dot) => {}
            t => Err(self.error(Unexpected(t), PARSING))?,
        }
        self.consume_peeked();
        Ok(())
    }
}
