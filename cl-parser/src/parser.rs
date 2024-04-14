use super::*;
use crate::error::{
    Error,
    ErrorKind::{self, *},
    PResult, Parsing,
};
use cl_ast::*;
use cl_lexer::Lexer;

/// Parses a sequence of [Tokens](Token) into an [AST](cl_ast)
#[derive(Debug)]
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
            let tok = self
                .lexer
                .scan()
                .map_err(|e| self.error(e.into(), while_parsing))?;
            match tok.ty {
                TokenKind::Comment | TokenKind::Invalid => continue,
                _ => break Ok(tok),
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
    /// Looks ahead at the next [Token]'s [TokenKind]
    pub fn peek_kind(&mut self, while_parsing: Parsing) -> PResult<TokenKind> {
        self.peek(while_parsing).map(|t| t.ty)
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
    /// Consumes one [Token]
    pub fn consume(&mut self, while_parsing: Parsing) -> PResult<Token> {
        match self.consume_peeked() {
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
            Err(self.error(ExpectedToken { want, got }, while_parsing))
        }
    }
    #[inline]
    pub fn match_op(&mut self, want: Punct, while_parsing: Parsing) -> PResult<Token> {
        self.match_type(TokenKind::Punct(want), while_parsing)
    }
}

// the three matched delimiter pairs
/// Square brackets: `[` `]`
const BRACKETS: (Punct, Punct) = (Punct::LBrack, Punct::RBrack);
/// Curly braces: `{` `}`
const CURLIES: (Punct, Punct) = (Punct::LCurly, Punct::RCurly);
/// Parentheses: `(` `)`
const PARENS: (Punct, Punct) = (Punct::LParen, Punct::RParen);

/// Parses constructions of the form `delim.0 f delim.1` (i.e. `(` `foobar` `)`)
const fn delim<'t, T>(
    f: impl Fn(&mut Parser<'t>) -> PResult<T>,
    delim: (Punct, Punct),
    while_parsing: Parsing,
) -> impl Fn(&mut Parser<'t>) -> PResult<T> {
    move |parser| {
        parser.match_op(delim.0, while_parsing)?;
        let out = f(parser)?;
        parser.match_op(delim.1, while_parsing)?;
        Ok(out)
    }
}

/// Parses constructions of the form `(f sep ~until)*`
///
/// where `~until` is a negative lookahead assertion
const fn sep<'t, T>(
    f: impl Fn(&mut Parser<'t>) -> PResult<T>,
    sep: Punct,
    until: Punct,
    while_parsing: Parsing,
) -> impl Fn(&mut Parser<'t>) -> PResult<Vec<T>> {
    move |parser| {
        let mut args = vec![];
        while TokenKind::Punct(until) != parser.peek_kind(while_parsing)? {
            args.push(f(parser)?);
            if TokenKind::Punct(sep) != parser.peek_kind(while_parsing)? {
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
    until: Punct,
    while_parsing: Parsing,
) -> impl Fn(&mut Parser<'t>) -> PResult<Vec<T>> {
    move |parser| {
        let mut out = vec![];
        while TokenKind::Punct(until) != parser.peek_kind(while_parsing)? {
            out.push(f(parser)?)
        }
        Ok(out)
    }
}

/// Expands to a pattern which matches item-like [Token] [TokenKind]s
macro item_like() {
    TokenKind::Punct(Punct::Hash)
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
            Ok(TokenKind::Punct(Punct::RCurly)) | Err(Error { reason: EndOfInput, .. }) => false,
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
        let absolute = matches!(
            self.peek_kind(PARSING)?,
            TokenKind::Punct(Punct::ColonColon)
        );
        if absolute {
            self.consume_peeked();
        }

        let mut parts = vec![self.path_part()?];
        while let Ok(TokenKind::Punct(Punct::ColonColon)) = self.peek_kind(PARSING) {
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
                Ok(TokenKind::Punct(Punct::Semi)) => {
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
        self.expr_from(|this| this.exprkind(0))
    }
}

/// Attribute parsing
impl<'t> Parser<'t> {
    /// Parses an [attribute set](Attrs)
    pub fn attributes(&mut self) -> PResult<Attrs> {
        if self.match_op(Punct::Hash, Parsing::Attrs).is_err() {
            return Ok(Attrs { meta: vec![] });
        }
        let meta = delim(
            sep(Self::meta, Punct::Comma, BRACKETS.1, Parsing::Attrs),
            BRACKETS,
            Parsing::Attrs,
        );
        Ok(Attrs { meta: meta(self)? })
    }
    /// Parses a single [attribute](Meta)
    pub fn meta(&mut self) -> PResult<Meta> {
        Ok(Meta { name: self.identifier()?, kind: self.meta_kind()? })
    }
    /// Parses data associated with a [Meta] attribute
    pub fn meta_kind(&mut self) -> PResult<MetaKind> {
        const PARSING: Parsing = Parsing::Meta;
        let lit_tuple = delim(
            sep(Self::literal, Punct::Comma, PARENS.1, PARSING),
            PARENS,
            PARSING,
        );
        Ok(match self.peek_kind(PARSING) {
            Ok(TokenKind::Punct(Punct::Eq)) => {
                self.consume_peeked();
                MetaKind::Equals(self.literal()?)
            }
            Ok(TokenKind::Punct(Punct::LParen)) => MetaKind::Func(lit_tuple(self)?),
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

    /// Parses a [`type` alias](Alias)
    pub fn parse_alias(&mut self) -> PResult<Alias> {
        const PARSING: Parsing = Parsing::Alias;
        self.match_type(TokenKind::Type, PARSING)?;

        let out = Ok(Alias {
            to: self.identifier()?,
            from: if self.match_op(Punct::Eq, PARSING).is_ok() {
                Some(self.ty()?.into())
            } else {
                None
            },
        });
        self.match_op(Punct::Semi, PARSING)?;
        out
    }

    /// Parses a [compile-time constant](Const)
    pub fn parse_const(&mut self) -> PResult<Const> {
        const PARSING: Parsing = Parsing::Const;
        self.match_type(TokenKind::Const, PARSING)?;

        let out = Ok(Const {
            name: self.identifier()?,
            ty: {
                self.match_op(Punct::Colon, PARSING)?;
                self.ty()?.into()
            },
            init: {
                self.match_op(Punct::Eq, PARSING)?;
                self.expr()?.into()
            },
        });
        self.match_op(Punct::Semi, PARSING)?;
        out
    }

    /// Parses a [`static` item](Static)
    pub fn parse_static(&mut self) -> PResult<Static> {
        const PARSING: Parsing = Parsing::Static;
        self.match_type(TokenKind::Static, PARSING)?;

        let out = Ok(Static {
            mutable: self.mutability()?,
            name: self.identifier()?,
            ty: {
                self.match_op(Punct::Colon, PARSING)?;
                self.ty()?.into()
            },
            init: {
                self.match_op(Punct::Eq, PARSING)?;
                self.expr()?.into()
            },
        });
        self.match_op(Punct::Semi, PARSING)?;
        out
    }

    /// Parses a [Module]
    pub fn parse_module(&mut self) -> PResult<Module> {
        const PARSING: Parsing = Parsing::Module;
        self.match_type(TokenKind::Mod, PARSING)?;

        Ok(Module { name: self.identifier()?, kind: self.modulekind()? })
    }

    /// Parses the item list associated with a [Module], if present
    pub fn modulekind(&mut self) -> PResult<ModuleKind> {
        const PARSING: Parsing = Parsing::ModuleKind;
        let inline = delim(Self::file, CURLIES, PARSING);

        match self.peek_kind(PARSING)? {
            TokenKind::Punct(Punct::LCurly) => Ok(ModuleKind::Inline(inline(self)?)),
            TokenKind::Punct(Punct::Semi) => {
                self.consume_peeked();
                Ok(ModuleKind::Outline)
            }
            got => Err(self.error(
                ExpectedToken { want: TokenKind::Punct(Punct::Semi), got },
                PARSING,
            )),
        }
    }

    /// Parses a [Function] definition
    pub fn parse_function(&mut self) -> PResult<Function> {
        const PARSING: Parsing = Parsing::Function;
        self.match_type(TokenKind::Fn, PARSING)?;

        Ok(Function {
            name: self.identifier()?,
            args: self.parse_params()?,
            rety: match self.peek_kind(PARSING)? {
                TokenKind::Punct(Punct::LCurly) | TokenKind::Punct(Punct::Semi) => None,
                TokenKind::Punct(Punct::Arrow) => {
                    self.consume_peeked();
                    Some(self.ty()?.into())
                }
                got => Err(self.error(
                    ExpectedToken { want: TokenKind::Punct(Punct::Arrow), got },
                    PARSING,
                ))?,
            },
            body: match self.peek_kind(PARSING)? {
                TokenKind::Punct(Punct::LCurly) => Some(self.block()?),
                TokenKind::Punct(Punct::Semi) => {
                    self.consume_peeked();
                    None
                }
                t => Err(self.error(Unexpected(t), PARSING))?,
            },
        })
    }

    /// Parses the [parameters](Param) associated with a Function
    pub fn parse_params(&mut self) -> PResult<Vec<Param>> {
        const PARSING: Parsing = Parsing::Function;
        delim(
            sep(Self::parse_param, Punct::Comma, PARENS.1, PARSING),
            PARENS,
            PARSING,
        )(self)
    }

    /// Parses a single function [parameter](Param)
    pub fn parse_param(&mut self) -> PResult<Param> {
        Ok(Param {
            mutability: self.mutability()?,
            name: self.identifier()?,
            ty: {
                self.match_op(Punct::Colon, Parsing::Param)?;
                self.ty()?.into()
            },
        })
    }

    /// Parses a [`struct` definition](Struct)
    pub fn parse_struct(&mut self) -> PResult<Struct> {
        const PARSING: Parsing = Parsing::Struct;
        self.match_type(TokenKind::Struct, PARSING)?;

        Ok(Struct {
            name: self.identifier()?,
            kind: match self.peek_kind(PARSING)? {
                TokenKind::Punct(Punct::LParen) => self.structkind_tuple()?,
                TokenKind::Punct(Punct::LCurly) => self.structkind_struct()?,
                TokenKind::Punct(Punct::Semi) => {
                    self.consume_peeked();
                    StructKind::Empty
                }
                got => Err(self.error(
                    ExpectedToken { want: TokenKind::Punct(Punct::Semi), got },
                    PARSING,
                ))?,
            },
        })
    }

    /// Parses a [tuple-`struct`](StructKind::Tuple)'s members
    pub fn structkind_tuple(&mut self) -> PResult<StructKind> {
        const PARSING: Parsing = Parsing::StructKind;

        Ok(StructKind::Tuple(delim(
            sep(Self::ty, Punct::Comma, PARENS.1, PARSING),
            PARENS,
            PARSING,
        )(self)?))
    }

    /// Parses a [`struct`](StructKind::Struct)s members
    pub fn structkind_struct(&mut self) -> PResult<StructKind> {
        const PARSING: Parsing = Parsing::StructKind;

        Ok(StructKind::Struct(delim(
            sep(Self::struct_member, Punct::Comma, CURLIES.1, PARSING),
            CURLIES,
            PARSING,
        )(self)?))
    }

    /// Parses a single [StructMember]
    pub fn struct_member(&mut self) -> PResult<StructMember> {
        const PARSING: Parsing = Parsing::StructMember;
        Ok(StructMember {
            vis: self.visibility()?,
            name: self.identifier()?,
            ty: {
                self.match_op(Punct::Colon, PARSING)?;
                self.ty()?
            },
        })
    }

    /// Parses an [`enum`](Enum) definition
    pub fn parse_enum(&mut self) -> PResult<Enum> {
        const PARSING: Parsing = Parsing::Enum;

        self.match_type(TokenKind::Enum, PARSING)?;

        Ok(Enum {
            name: self.identifier()?,
            kind: match self.peek_kind(PARSING)? {
                TokenKind::Punct(Punct::LCurly) => EnumKind::Variants(delim(
                    sep(Self::enum_variant, Punct::Comma, Punct::RCurly, PARSING),
                    CURLIES,
                    PARSING,
                )(self)?),
                TokenKind::Punct(Punct::Semi) => {
                    self.consume_peeked();
                    EnumKind::NoVariants
                }
                t => Err(self.error(Unexpected(t), PARSING))?,
            },
        })
    }

    /// Parses an [`enum`](Enum) [Variant]
    pub fn enum_variant(&mut self) -> PResult<Variant> {
        const PARSING: Parsing = Parsing::Variant;

        Ok(Variant {
            name: self.identifier()?,
            kind: match self.peek_kind(PARSING)? {
                TokenKind::Punct(Punct::Eq) => self.variantkind_clike()?,
                TokenKind::Punct(Punct::LCurly) => self.variantkind_struct()?,
                TokenKind::Punct(Punct::LParen) => self.variantkind_tuple()?,
                _ => VariantKind::Plain,
            },
        })
    }

    /// Parses a [C-like](VariantKind::CLike) [`enum`](Enum) [Variant]
    pub fn variantkind_clike(&mut self) -> PResult<VariantKind> {
        const PARSING: Parsing = Parsing::VariantKind;

        self.match_op(Punct::Eq, PARSING)?;
        let tok = self.match_type(TokenKind::Literal, PARSING)?;

        Ok(VariantKind::CLike(match tok.data() {
            TokenData::Integer(i) => *i,
            _ => panic!("Expected token data for {tok:?} while parsing {PARSING}"),
        }))
    }

    /// Parses a [struct-like](VariantKind::Struct) [`enum`](Enum) [Variant]
    pub fn variantkind_struct(&mut self) -> PResult<VariantKind> {
        const PARSING: Parsing = Parsing::VariantKind;
        Ok(VariantKind::Struct(delim(
            sep(Self::struct_member, Punct::Comma, Punct::RCurly, PARSING),
            CURLIES,
            PARSING,
        )(self)?))
    }

    /// Parses a [tuple-like](VariantKind::Tuple) [`enum`](Enum) [Variant]
    pub fn variantkind_tuple(&mut self) -> PResult<VariantKind> {
        const PARSING: Parsing = Parsing::VariantKind;
        Ok(VariantKind::Tuple(delim(
            sep(Self::ty, Punct::Comma, Punct::RParen, PARSING),
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
            TokenKind::Punct(Punct::Bang) => {
                self.consume_peeked();
                TyKind::Never
            }
            TokenKind::SelfTy => {
                self.consume_peeked();
                TyKind::SelfTy
            }
            TokenKind::Punct(Punct::Amp) | TokenKind::Punct(Punct::AmpAmp) => self.tyref()?.into(),
            TokenKind::Punct(Punct::LParen) => self.tytuple()?.into(),
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
                sep(Self::ty, Punct::Comma, PARENS.1, PARSING),
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
                TokenKind::Punct(Punct::Amp) => count += 1,
                TokenKind::Punct(Punct::AmpAmp) => count += 2,
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
                    TokenKind::Punct(Punct::Arrow) => {
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
    TokenKind::True | TokenKind::False | TokenKind::Literal
}
/// Expands to a pattern which matches path-like [token Types](Type)
macro path_like() {
    TokenKind::Super
        | TokenKind::SelfKw
        | TokenKind::Identifier
        | TokenKind::Punct(Punct::ColonColon)
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
            TokenData::String(ident) => Ok(ident.into()),
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
            TokenKind::Punct(Punct::Semi) => StmtKind::Empty,
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
            ty: if Ok(TokenKind::Punct(Punct::Colon)) == self.peek_kind(Parsing::Let) {
                self.consume_peeked();
                Some(self.ty()?.into())
            } else {
                None
            },
            init: if Ok(TokenKind::Punct(Punct::Eq)) == self.peek_kind(Parsing::Let) {
                self.consume_peeked();
                Some(self.expr()?.into())
            } else {
                None
            },
        })
    }
}

/// # Expression parsing
impl<'t> Parser<'t> {
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

    /// Parses an [ExprKind]
    ///
    /// See also: [Parser::expr]
    pub fn exprkind(&mut self, power: u8) -> PResult<ExprKind> {
        let parsing = Parsing::ExprKind;
        //
        let mut head = match self.peek_kind(Parsing::Unary)? {
            literal_like!() => self.literal()?.into(),
            path_like!() => self.path()?.into(),
            TokenKind::Punct(Punct::Amp | Punct::AmpAmp) => self.addrof()?.into(),
            TokenKind::Punct(Punct::LCurly) => self.block()?.into(),
            TokenKind::Punct(Punct::LBrack) => self.exprkind_arraylike()?,
            TokenKind::Punct(Punct::LParen) => self.exprkind_tuplelike()?,
            TokenKind::Punct(op) => {
                let (kind, prec) = from_prefix(op)
                    .ok_or_else(|| self.error(Unexpected(TokenKind::Punct(op)), parsing))?;
                let ((), after) = prec.prefix().expect("should have a precedence");
                self.consume_peeked();
                Unary { kind, tail: self.exprkind(after)?.into() }.into()
            }
            TokenKind::While => ExprKind::While(self.parse_while()?),
            TokenKind::If => ExprKind::If(self.parse_if()?),
            TokenKind::For => ExprKind::For(self.parse_for()?),
            TokenKind::Break => {
                self.consume_peeked();
                Break { body: self.optional_expr()?.map(Into::into) }.into()
            }
            TokenKind::Return => {
                self.consume_peeked();
                Return { body: self.optional_expr()?.map(Into::into) }.into()
            }
            TokenKind::Continue => {
                self.consume_peeked();
                Continue.into()
            }
            t => Err(self.error(Unexpected(t), Parsing::Unary))?,
        };

        fn from_postfix(op: Punct) -> Option<Precedence> {
            Some(match op {
                Punct::LBrack => Precedence::Index,
                Punct::LParen => Precedence::Postfix,
                _ => None?,
            })
        }

        while let Ok(TokenKind::Punct(op)) = self.peek_kind(parsing) {
            if let Some((before, ())) = from_postfix(op).and_then(Precedence::postfix) {
                if before < power {
                    break;
                }
                self.consume_peeked();

                head = match op {
                    Punct::LBrack => {
                        let indices = sep(Self::expr, Punct::Comma, Punct::RBrack, parsing)(self)?;
                        self.match_op(Punct::RBrack, parsing)?;
                        ExprKind::Index(Index { head: head.into(), indices })
                    }
                    Punct::LParen => {
                        let exprs = sep(Self::expr, Punct::Comma, Punct::RParen, parsing)(self)?;
                        self.match_op(Punct::RParen, parsing)?;
                        Binary {
                            kind: BinaryKind::Call,
                            parts: (head, Tuple { exprs }.into()).into(),
                        }
                        .into()
                    }
                    _ => Err(self.error(Unexpected(TokenKind::Punct(op)), parsing))?,
                };
                continue;
            }
            // infix expressions
            if let Some((kind, prec)) = from_infix(op) {
                let (before, after) = prec.infix().expect("should have a precedence");
                if before < power {
                    break;
                }
                self.consume_peeked();

                let tail = self.exprkind(after)?;
                head = Binary { kind, parts: (head, tail).into() }.into();
                continue;
            }

            if let Some((kind, prec)) = from_assign(op) {
                let (before, after) = prec.infix().expect("should have a precedence");
                if before < power {
                    break;
                }
                self.consume_peeked();

                let tail = self.exprkind(after)?;
                head = Assign { kind, parts: (head, tail).into() }.into();
                continue;
            }
            break;
        }
        Ok(head)
    }

    /// [Array] = '[' ([Expr] ',')* [Expr]? ']'
    ///
    /// Array and ArrayRef are ambiguous until the second token,
    /// so they can't be independent subexpressions
    pub fn exprkind_arraylike(&mut self) -> PResult<ExprKind> {
        const PARSING: Parsing = Parsing::Array;
        const START: Punct = Punct::LBrack;
        const END: Punct = Punct::RBrack;

        self.match_op(START, PARSING)?;
        let out = match self.peek_kind(PARSING)? {
            TokenKind::Punct(END) => Array { values: vec![] }.into(),
            _ => self.exprkind_array_rep()?,
        };
        self.match_op(END, PARSING)?;
        Ok(out)
    }

    /// [ArrayRep] = `[` [Expr] `;` [Expr] `]`
    pub fn exprkind_array_rep(&mut self) -> PResult<ExprKind> {
        const PARSING: Parsing = Parsing::Array;
        const END: Punct = Punct::RBrack;

        let first = self.expr()?;
        Ok(match self.peek_kind(PARSING)? {
            TokenKind::Punct(Punct::Semi) => ArrayRep {
                value: first.kind.into(),
                repeat: {
                    self.consume_peeked();
                    Box::new(self.exprkind(0)?)
                },
            }
            .into(),
            TokenKind::Punct(Punct::RBrack) => Array { values: vec![first] }.into(),
            TokenKind::Punct(Punct::Comma) => Array {
                values: {
                    self.consume_peeked();
                    let mut out = vec![first];
                    out.extend(sep(Self::expr, Punct::Comma, END, PARSING)(self)?);
                    out
                },
            }
            .into(),
            ty => Err(self.error(Unexpected(ty), PARSING))?,
        })
    }
    /// [Group] = `(`([Empty](ExprKind::Empty)|[Expr]|[Tuple])`)`
    ///
    /// [ExprKind::Empty] and [Group] are special cases of [Tuple]
    pub fn exprkind_tuplelike(&mut self) -> PResult<ExprKind> {
        self.match_op(Punct::LParen, Parsing::Group)?;
        let out = match self.peek_kind(Parsing::Group)? {
            TokenKind::Punct(Punct::RParen) => Ok(ExprKind::Empty),
            _ => self.exprkind_group(),
        };
        self.match_op(Punct::RParen, Parsing::Group)?;
        out
    }
    /// [Group] = `(`([Empty](ExprKind::Empty)|[Expr]|[Tuple])`)`
    pub fn exprkind_group(&mut self) -> PResult<ExprKind> {
        let first = self.expr()?;
        match self.peek_kind(Parsing::Group)? {
            TokenKind::Punct(Punct::Comma) => {
                let mut exprs = vec![first];
                self.consume_peeked();
                while TokenKind::Punct(Punct::RParen) != self.peek_kind(Parsing::Tuple)? {
                    exprs.push(self.expr()?);
                    match self.peek_kind(Parsing::Tuple)? {
                        TokenKind::Punct(Punct::Comma) => self.consume_peeked(),
                        _ => break,
                    };
                }
                Ok(Tuple { exprs }.into())
            }
            _ => Ok(Group { expr: first.kind.into() }.into()),
        }
    }
}

/// ## Subexpressions
impl<'t> Parser<'t> {
    /// [AddrOf] = (`&`|`&&`)* [Expr]
    pub fn addrof(&mut self) -> PResult<AddrOf> {
        const PARSING: Parsing = Parsing::AddrOf;
        let mut count = 0;
        loop {
            count += match self.peek_kind(PARSING)? {
                TokenKind::Punct(Punct::Amp) => 1,
                TokenKind::Punct(Punct::AmpAmp) => 2,
                _ => break,
            };
            self.consume_peeked();
        }
        Ok(AddrOf { count, mutable: self.mutability()?, expr: self.exprkind(0)?.into() })
    }
    /// [Literal] = [LITERAL](TokenKind::Literal) | `true` | `false`
    pub fn literal(&mut self) -> PResult<Literal> {
        let Token { ty, data, .. } = self.consume(Parsing::Literal)?;
        match ty {
            TokenKind::True => return Ok(Literal::Bool(true)),
            TokenKind::False => return Ok(Literal::Bool(false)),
            TokenKind::Literal => (),
            t => return Err(self.error(Unexpected(t), Parsing::Literal)),
        }
        Ok(match data {
            TokenData::String(v) => Literal::String(v),
            TokenData::Character(v) => Literal::Char(v),
            TokenData::Integer(v) => Literal::Int(v),
            TokenData::Float(v) => todo!("Literal::Float({v})"),
            _ => panic!("Expected token data for {ty:?}"),
        })
    }
    /// [Block] = `{` [Stmt]* `}`
    pub fn block(&mut self) -> PResult<Block> {
        const A_BLOCK: Parsing = Parsing::Block;
        Ok(Block { stmts: delim(rep(Self::stmt, CURLIES.1, A_BLOCK), CURLIES, A_BLOCK)(self)? })
    }
}
/// ## Control flow subexpressions
impl<'t> Parser<'t> {
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

/// Precedence provides a total ordering among operators
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Precedence {
    Assign,
    Compare,
    Range,
    Index,
    Logic,
    Bitwise,
    Shift,
    Factor,
    Term,
    Unary,
    Postfix,
    Member, // left-associative
}

impl Precedence {
    #[inline]
    pub fn level(self) -> u8 {
        (self as u8) << 1
    }
    pub fn prefix(self) -> Option<((), u8)> {
        match self {
            Self::Unary => Some(((), self.level())),
            _ => None,
        }
    }
    pub fn infix(self) -> Option<(u8, u8)> {
        let level = self.level();
        match self {
            Self::Unary => None,
            Self::Member | Self::Assign => Some((level + 1, level)),
            _ => Some((level, level + 1)),
        }
    }
    pub fn postfix(self) -> Option<(u8, ())> {
        match self {
            Self::Index | Self::Postfix => Some((self.level(), ())),
            _ => None,
        }
    }
}
impl From<AssignKind> for Precedence {
    fn from(_value: AssignKind) -> Self {
        Precedence::Assign
    }
}

impl From<BinaryKind> for Precedence {
    fn from(value: BinaryKind) -> Self {
        use BinaryKind as Op;
        match value {
            Op::Call => Precedence::Postfix,
            Op::Dot => Precedence::Member,
            Op::Mul | Op::Div | Op::Rem => Precedence::Term,
            Op::Add | Op::Sub => Precedence::Factor,
            Op::Shl | Op::Shr => Precedence::Shift,
            Op::BitAnd | Op::BitOr | Op::BitXor => Precedence::Bitwise,
            Op::LogAnd | Op::LogOr | Op::LogXor => Precedence::Logic,
            Op::RangeExc | Op::RangeInc => Precedence::Range,
            Op::Lt | Op::LtEq | Op::Equal | Op::NotEq | Op::GtEq | Op::Gt => Precedence::Compare,
        }
    }
}
impl From<UnaryKind> for Precedence {
    fn from(value: UnaryKind) -> Self {
        use UnaryKind as Op;
        match value {
            Op::Deref | Op::Neg | Op::Not | Op::At | Op::Tilde => Precedence::Unary,
        }
    }
}

/// Creates helper functions for
macro operator($($name:ident ($takes:ident => $returns:ident) {$($t:ident => $p:ident),*$(,)?};)*) {$(
    pub fn $name (value: $takes) -> Option<($returns, Precedence)> {
        match value {
            $($takes::$t => Some(($returns::$p, Precedence::from($returns::$p))),)*
            _ => None?,
        }
    })*
}

operator! {
    from_prefix (Punct => UnaryKind) {
        Star => Deref,
        Minus => Neg,
        Bang => Not,
        At => At,
        Tilde => Tilde,
    };
    from_assign(Punct => AssignKind) {
        Eq => Plain,
        AmpEq => And,
        BarEq => Or,
        XorEq => Xor,
        LtLtEq => Shl,
        GtGtEq => Shr,
        PlusEq => Add,
        MinusEq => Sub,
        StarEq => Mul,
        SlashEq => Div,
        RemEq => Rem,
    };
    from_infix (Punct => BinaryKind) {

        Lt => Lt,
        LtEq => LtEq,
        EqEq => Equal,
        BangEq => NotEq,
        GtEq => GtEq,
        Gt => Gt,
        DotDot => RangeExc,
        DotDotEq => RangeInc,
        AmpAmp => LogAnd,
        BarBar => LogOr,
        XorXor => LogXor,
        Amp => BitAnd,
        Bar => BitOr,
        Xor => BitXor,
        LtLt => Shl,
        GtGt => Shr,
        Plus => Add,
        Minus => Sub,
        Star => Mul,
        Slash => Div,
        Rem => Rem,
        Dot => Dot,
    };
}
