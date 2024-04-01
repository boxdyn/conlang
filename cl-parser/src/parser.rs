use super::*;
use crate::error::{
    Error,
    ErrorKind::{self, *},
    PResult, Parsing,
};
use cl_ast::*;
use cl_lexer::Lexer;

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
                t if t.ty() == Type::Invalid => continue,
                t if t.ty() == Type::Comment => continue,
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
    /// Looks ahead at the next [Token]'s [Type]
    pub fn peek_type(&mut self, while_parsing: Parsing) -> PResult<Type> {
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
    /// Consumes the next [Token] if it matches the pattern [Type]
    pub fn match_type(&mut self, want: Type, while_parsing: Parsing) -> PResult<Token> {
        let got = self.peek_type(while_parsing)?;
        if got == want {
            Ok(self.consume_peeked().expect("should not fail after peek"))
        } else {
            Err(self.error(Expected { want, got }, while_parsing))
        }
    }
    /// Consumes the next token if it matches the pattern [Keyword]
    pub fn match_kw(&mut self, pat: Keyword, while_parsing: Parsing) -> PResult<Token> {
        self.match_type(Type::Keyword(pat), while_parsing)
    }
}

// the three matched delimiter pairs
/// Square brackets: `[` `]`
const BRACKETS: (Type, Type) = (Type::LBrack, Type::RBrack);
/// Curly braces: `{` `}`
const CURLIES: (Type, Type) = (Type::LCurly, Type::RCurly);
/// Parentheses: `(` `)`
const PARENS: (Type, Type) = (Type::LParen, Type::RParen);

/// Parses constructions of the form `delim.0 f delim.1` (i.e. `(` `foobar` `)`)
const fn delim<'t, T>(
    f: impl Fn(&mut Parser<'t>) -> PResult<T>,
    delim: (Type, Type),
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
    sep: Type,
    until: Type,
    while_parsing: Parsing,
) -> impl Fn(&mut Parser<'t>) -> PResult<Vec<T>> {
    move |parser| {
        let mut args = vec![];
        while until != parser.peek_type(while_parsing)? {
            args.push(f(parser)?);
            if sep != parser.peek_type(while_parsing)? {
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
    until: Type,
    while_parsing: Parsing,
) -> impl Fn(&mut Parser<'t>) -> PResult<Vec<T>> {
    move |parser| {
        let mut out = vec![];
        while until != parser.peek_type(while_parsing)? {
            out.push(f(parser)?)
        }
        Ok(out)
    }
}

/// Expands to a pattern which matches item-like [Token] [Type]s
macro item_like() {
    Type::Hash
        | Type::Keyword(
            Keyword::Pub
                | Keyword::Type
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
        while match self.peek_type(Parsing::File) {
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
        const PARSING: Parsing = Parsing::Stmt;
        let start = self.loc();
        Ok(Stmt {
            kind: self.stmtkind()?,
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

/// Attribute parsing
impl<'t> Parser<'t> {
    /// Parses an [attribute set](Attrs)
    pub fn attributes(&mut self) -> PResult<Attrs> {
        if self.match_type(Type::Hash, Parsing::Attrs).is_err() {
            return Ok(Attrs { meta: vec![] });
        }
        let meta = delim(
            sep(Self::meta, Type::Comma, BRACKETS.1, Parsing::Attrs),
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
            sep(Self::literal, Type::Comma, PARENS.1, PARSING),
            PARENS,
            PARSING,
        );
        Ok(match self.peek_type(PARSING) {
            Ok(Type::Eq) => {
                self.consume_peeked();
                MetaKind::Equals(self.literal()?)
            }
            Ok(Type::LParen) => MetaKind::Func(lit_tuple(self)?),
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
        Ok(match self.peek_type(Parsing::Item)? {
            Type::Keyword(Keyword::Type) => self.parse_alias()?.into(),
            Type::Keyword(Keyword::Const) => self.parse_const()?.into(),
            Type::Keyword(Keyword::Static) => self.parse_static()?.into(),
            Type::Keyword(Keyword::Mod) => self.parse_module()?.into(),
            Type::Keyword(Keyword::Fn) => self.parse_function()?.into(),
            Type::Keyword(Keyword::Struct) => self.parse_struct()?.into(),
            Type::Keyword(Keyword::Enum) => self.parse_enum()?.into(),
            Type::Keyword(Keyword::Impl) => self.parse_impl()?.into(),
            t => Err(self.error(Unexpected(t), Parsing::Item))?,
        })
    }

    pub fn parse_alias(&mut self) -> PResult<Alias> {
        const PARSING: Parsing = Parsing::Alias;
        self.match_kw(Keyword::Type, PARSING)?;
        let out = Ok(Alias {
            to: self.identifier()?,
            from: if self.match_type(Type::Eq, PARSING).is_ok() {
                Some(self.ty()?.into())
            } else {
                None
            },
        });
        self.match_type(Type::Semi, PARSING)?;
        out
    }

    pub fn parse_const(&mut self) -> PResult<Const> {
        const PARSING: Parsing = Parsing::Const;
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
        const PARSING: Parsing = Parsing::Static;
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
        const PARSING: Parsing = Parsing::Module;
        self.match_kw(Keyword::Mod, PARSING)?;
        Ok(Module { name: self.identifier()?, kind: self.modulekind()? })
    }
    pub fn modulekind(&mut self) -> PResult<ModuleKind> {
        const PARSING: Parsing = Parsing::ModuleKind;
        let inline = delim(Self::file, CURLIES, PARSING);

        match self.peek_type(PARSING)? {
            Type::LCurly => Ok(ModuleKind::Inline(inline(self)?)),
            Type::Semi => {
                self.consume_peeked();
                Ok(ModuleKind::Outline)
            }
            got => Err(self.error(Expected { want: Type::Semi, got }, PARSING)),
        }
    }
    pub fn parse_function(&mut self) -> PResult<Function> {
        const PARSING: Parsing = Parsing::Function;
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
        const PARSING: Parsing = Parsing::Function;
        delim(
            sep(Self::parse_param, Type::Comma, PARENS.1, PARSING),
            PARENS,
            PARSING,
        )(self)
    }
    pub fn parse_param(&mut self) -> PResult<Param> {
        Ok(Param {
            mutability: self.mutability()?,
            name: self.identifier()?,
            ty: {
                self.match_type(Type::Colon, Parsing::Param)?;
                self.ty()?.into()
            },
        })
    }
    pub fn parse_struct(&mut self) -> PResult<Struct> {
        const PARSING: Parsing = Parsing::Struct;
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
        const PARSING: Parsing = Parsing::StructKind;

        Ok(StructKind::Tuple(delim(
            sep(Self::ty, Type::Comma, PARENS.1, PARSING),
            PARENS,
            PARSING,
        )(self)?))
    }
    pub fn structkind_struct(&mut self) -> PResult<StructKind> {
        const PARSING: Parsing = Parsing::StructKind;
        Ok(StructKind::Struct(delim(
            sep(Self::struct_member, Type::Comma, CURLIES.1, PARSING),
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
                self.match_type(Type::Colon, PARSING)?;
                self.ty()?
            },
        })
    }
    pub fn parse_enum(&mut self) -> PResult<Enum> {
        const PARSING: Parsing = Parsing::Enum;
        self.match_kw(Keyword::Enum, PARSING)?;
        Err(self.error(Todo, PARSING))
    }
    pub fn parse_impl(&mut self) -> PResult<Impl> {
        const PARSING: Parsing = Parsing::Impl;
        self.match_kw(Keyword::Impl, PARSING)?;
        Err(self.error(Todo, PARSING))
    }

    pub fn visibility(&mut self) -> PResult<Visibility> {
        if let Type::Keyword(Keyword::Pub) = self.peek_type(Parsing::Visibility)? {
            self.consume_peeked();
            return Ok(Visibility::Public);
        };
        Ok(Visibility::Private)
    }
    pub fn mutability(&mut self) -> PResult<Mutability> {
        if let Type::Keyword(Keyword::Mut) = self.peek_type(Parsing::Mutability)? {
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
        let out = match self.peek_type(PARSING)? {
            Type::Bang => {
                self.consume_peeked();
                TyKind::Never
            }
            Type::Keyword(Keyword::SelfTy) => {
                self.consume_peeked();
                TyKind::SelfTy
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
        const PARSING: Parsing = Parsing::TyTuple;
        Ok(TyTuple {
            types: delim(
                sep(Self::ty, Type::Comma, PARENS.1, PARSING),
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
        const PARSING: Parsing = Parsing::TyFn;
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
        const PARSING: Parsing = Parsing::PathPart;
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
        let tok = self.match_type(Type::Identifier, Parsing::Identifier)?;
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
        Ok(match self.peek_type(Parsing::StmtKind)? {
            Type::Semi => StmtKind::Empty,
            Type::Keyword(Keyword::Let) => self.parse_let()?.into(),
            item_like!() => self.item()?.into(),
            _ => self.expr()?.into(),
        })
    }

    pub fn parse_let(&mut self) -> PResult<Let> {
        self.match_kw(Keyword::Let, Parsing::Let)?;
        Ok(Let {
            mutable: self.mutability()?,
            name: self.identifier()?,
            ty: if Ok(Type::Colon) == self.peek_type(Parsing::Let) {
                self.consume_peeked();
                Some(self.ty()?.into())
            } else {
                None
            },
            init: if Ok(Type::Eq) == self.peek_type(Parsing::Let) {
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
        const PARSING: Parsing = Parsing::Index;
        let head = self.expr_from(Self::exprkind_primary)?;
        if Ok(Type::LBrack) != self.peek_type(PARSING) {
            return Ok(head.kind);
        }

        let mut indices = vec![];
        while Ok(Type::LBrack) == self.peek_type(PARSING) {
            indices.push(delim(Self::tuple, BRACKETS, PARSING)(self)?.into());
        }
        Ok(Index { head: head.into(), indices }.into())
    }
    /// Delegates to the set of highest-priority rules based on unambiguous pattern matching
    pub fn exprkind_primary(&mut self) -> PResult<ExprKind> {
        match self.peek_type(Parsing::Expr)? {
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
            _ => Err(self.error(Nothing, Parsing::Expr)),
        }
    }
    /// [Array] = '[' ([Expr] ',')* [Expr]? ']'
    ///
    /// Array and ArrayRef are ambiguous until the second token,
    /// so they can't be independent subexpressions
    pub fn exprkind_array(&mut self) -> PResult<ExprKind> {
        const PARSING: Parsing = Parsing::Array;
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
        const PARSING: Parsing = Parsing::Array;
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
                    out.extend(sep(Self::expr, Type::Comma, END, PARSING)(self)?);
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
        self.match_type(Type::LParen, Parsing::Group)?;
        let out = match self.peek_type(Parsing::Group)? {
            Type::RParen => Ok(ExprKind::Empty),
            _ => self.exprkind_group(),
        };
        match self.peek_type(Parsing::Group) {
            Ok(Type::RParen) => self.consume_peeked(),
            _ => Err(self.error(UnmatchedParentheses, Parsing::Group))?,
        };
        out
    }
    /// [Group] = `(`([Empty](ExprKind::Empty)|[Expr]|[Tuple])`)`
    pub fn exprkind_group(&mut self) -> PResult<ExprKind> {
        let first = self.expr()?;
        match self.peek_type(Parsing::Group)? {
            Type::Comma => {
                let mut exprs = vec![first];
                self.consume_peeked();
                while Type::RParen != self.peek_type(Parsing::Tuple)? {
                    exprs.push(self.expr()?);
                    match self.peek_type(Parsing::Tuple)? {
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
        let tok = self.consume(Parsing::Literal)?;
        // keyword literals true and false
        match tok.ty() {
            Type::Keyword(Keyword::True) => return Ok(Literal::Bool(true)),
            Type::Keyword(Keyword::False) => return Ok(Literal::Bool(false)),
            Type::String | Type::Character | Type::Integer | Type::Float => (),
            t => return Err(self.error(Unexpected(t), Parsing::Literal)),
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
            match self.peek_type(Parsing::Tuple)? {
                Type::Comma => self.consume_peeked(),
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
        self.match_kw(Keyword::Break, Parsing::Break)?;
        Ok(Break { body: self.optional_expr()?.map(Into::into) })
    }
    /// [Return] = `return` [Expr]?
    pub fn parse_return(&mut self) -> PResult<Return> {
        self.match_kw(Keyword::Return, Parsing::Return)?;
        Ok(Return { body: self.optional_expr()?.map(Into::into) })
    }
    /// [Continue] = `continue`
    pub fn parse_continue(&mut self) -> PResult<Continue> {
        self.match_kw(Keyword::Continue, Parsing::Continue)?;
        Ok(Continue)
    }
    /// [While] = `while` [Expr] [Block] [Else]?
    pub fn parse_while(&mut self) -> PResult<While> {
        self.match_kw(Keyword::While, Parsing::While)?;
        Ok(While {
            cond: self.expr()?.into(),
            pass: self.block()?.into(),
            fail: self.parse_else()?,
        })
    }
    /// [If] = <code>`if` [Expr] [Block] [Else]?</code>
    #[rustfmt::skip] // second line is barely not long enough
    pub fn parse_if(&mut self) -> PResult<If> {
    self.match_kw(Keyword::If, Parsing::If)?;
    Ok(If {
        cond: self.expr()?.into(),
        pass: self.block()?.into(),
        fail: self.parse_else()?,
    })
}
    /// [For]: `for` Pattern (TODO) `in` [Expr] [Block] [Else]?
    pub fn parse_for(&mut self) -> PResult<For> {
        self.match_kw(Keyword::For, Parsing::For)?;
        let bind = self.identifier()?;
        self.match_kw(Keyword::In, Parsing::For)?;
        Ok(For {
            bind,
            cond: self.expr()?.into(),
            pass: self.block()?.into(),
            fail: self.parse_else()?,
        })
    }
    /// [Else]: (`else` [Block])?
    pub fn parse_else(&mut self) -> PResult<Else> {
        match self.peek_type(Parsing::Else) {
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
    const PARSING: Parsing = Parsing::$returns;
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
            Tilde => Tilde, // ~
        };
    }
    pub fn member_op(&mut self) -> PResult<()> {
        const PARSING: Parsing = Parsing::Member;
        match self.peek(PARSING)?.ty() {
            Type::Dot => {}
            t => Err(self.error(Unexpected(t), PARSING))?,
        }
        self.consume_peeked();
        Ok(())
    }
}
