use super::*;
use crate::error::{
    Error,
    ErrorKind::{self, *},
    PResult, Parsing,
};
use cl_ast::*;
use cl_lexer::Lexer;

// Precedence climbing expression parser
mod prec;

/// Parses a sequence of [Tokens](Token) into an [AST](cl_ast)
#[derive(Debug)]
pub struct Parser<'t> {
    /// Name of the file being parsed
    file: Sym,
    /// Lazy tokenizer
    lexer: Lexer<'t>,
    /// Look-ahead buffer
    next: Option<Token>,
    /// The location of the current token
    loc: Loc,
}

/// Basic parser functionality
impl<'t> Parser<'t> {
    pub fn new(filename: impl AsRef<str>, lexer: Lexer<'t>) -> Self {
        Self { file: filename.as_ref().into(), loc: Loc::from(&lexer), lexer, next: None }
    }

    /// Gets the location of the last consumed [Token]
    pub fn loc(&self) -> Loc {
        self.loc
    }

    /// Attempts to parse anything that implements the [Parse] trait
    #[inline]
    pub fn parse<P: Parse<'t>>(&mut self) -> PResult<P> {
        P::parse(self)
    }

    /// Constructs an [Error]
    pub fn error(&self, reason: ErrorKind, while_parsing: Parsing) -> Error {
        Error { in_file: self.file, reason, while_parsing, loc: self.loc }
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
}

// the three matched delimiter pairs
/// Square brackets: `[` `]`
const BRACKETS: (TokenKind, TokenKind) = (TokenKind::LBrack, TokenKind::RBrack);

/// Curly braces: `{` `}`
const CURLIES: (TokenKind, TokenKind) = (TokenKind::LCurly, TokenKind::RCurly);

/// Parentheses: `(` `)`
const PARENS: (TokenKind, TokenKind) = (TokenKind::LParen, TokenKind::RParen);

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

/// Expands to a pattern which matches item-like [TokenKind]s
macro item_like() {
    TokenKind::Hash
        | TokenKind::Pub
        | TokenKind::Type
        | TokenKind::Const
        | TokenKind::Static
        | TokenKind::Mod
        | TokenKind::Fn
        | TokenKind::Struct
        | TokenKind::Enum
        | TokenKind::Impl
        | TokenKind::Use
}

/// Expands to a pattern which matches literal-like [TokenKind]s
macro literal_like() {
    TokenKind::True | TokenKind::False | TokenKind::Literal
}

/// Expands to a pattern which matches path-like [TokenKinds](TokenKind)
macro path_like() {
    TokenKind::Super | TokenKind::SelfTy | TokenKind::Identifier | TokenKind::ColonColon
}

pub trait Parse<'t>: Sized {
    /// Parses a Self from the provided [Parser]
    fn parse(p: &mut Parser<'t>) -> PResult<Self>;
}

impl Parse<'_> for Sym {
    /// [Sym] = [`Identifier`](TokenKind::Identifier)
    fn parse(p: &mut Parser) -> PResult<Sym> {
        let tok = p.match_type(TokenKind::Identifier, Parsing::Identifier)?;
        match tok.data() {
            TokenData::String(ident) => Ok(ident.into()),
            _ => panic!("Expected token data for {tok:?}"),
        }
    }
}

impl Parse<'_> for Mutability {
    /// [Mutability] = `mut`?
    #[inline]
    fn parse(p: &mut Parser) -> PResult<Mutability> {
        Ok(match p.match_type(TokenKind::Mut, Parsing::Mutability) {
            Ok(_) => Mutability::Mut,
            Err(_) => Mutability::Not,
        })
    }
}

impl Parse<'_> for Visibility {
    /// [Visibility] = `pub`?
    #[inline]
    fn parse(p: &mut Parser) -> PResult<Self> {
        Ok(match p.match_type(TokenKind::Pub, Parsing::Visibility) {
            Ok(_) => Visibility::Public,
            Err(_) => Visibility::Private,
        })
    }
}

impl Parse<'_> for Literal {
    /// [Literal] = [LITERAL](TokenKind::Literal) | `true` | `false`
    fn parse(p: &mut Parser) -> PResult<Literal> {
        let Token { ty, data, .. } = p.consume(Parsing::Literal)?;
        match ty {
            TokenKind::True => return Ok(Literal::Bool(true)),
            TokenKind::False => return Ok(Literal::Bool(false)),
            TokenKind::Literal => (),
            t => return Err(p.error(Unexpected(t), Parsing::Literal)),
        }
        Ok(match data {
            TokenData::String(v) => Literal::String(v),
            TokenData::Character(v) => Literal::Char(v),
            TokenData::Integer(v) => Literal::Int(v),
            TokenData::Float(v) => Literal::Float(v.to_bits()),
            _ => panic!("Expected token data for {ty:?}"),
        })
    }
}

impl Parse<'_> for File {
    /// Parses a [File]
    fn parse(p: &mut Parser<'_>) -> PResult<Self> {
        let mut items = vec![];
        while match p.peek_kind(Parsing::File) {
            Ok(TokenKind::RCurly) | Err(Error { reason: EndOfInput, .. }) => false,
            Ok(_) => true,
            Err(e) => Err(e)?,
        } {
            items.push(Item::parse(p)?)
        }
        Ok(File { name: p.file.to_ref(), items })
    }
}

impl Parse<'_> for Attrs {
    /// Parses an [attribute set](Attrs)
    fn parse(p: &mut Parser) -> PResult<Attrs> {
        if p.match_type(TokenKind::Hash, Parsing::Attrs).is_err() {
            return Ok(Attrs { meta: vec![] });
        }
        let meta = delim(
            sep(Meta::parse, TokenKind::Comma, BRACKETS.1, Parsing::Attrs),
            BRACKETS,
            Parsing::Attrs,
        )(p)?;
        Ok(Attrs { meta })
    }
}

impl Parse<'_> for Meta {
    /// Parses a single [attribute](Meta)
    fn parse(p: &mut Parser) -> PResult<Meta> {
        Ok(Meta { name: Sym::parse(p)?, kind: MetaKind::parse(p)? })
    }
}

impl Parse<'_> for MetaKind {
    /// Parses data associated with a [Meta] attribute
    fn parse(p: &mut Parser) -> PResult<MetaKind> {
        const P: Parsing = Parsing::Meta;
        let lit_tuple = delim(
            sep(Literal::parse, TokenKind::Comma, PARENS.1, P),
            PARENS,
            P,
        );
        Ok(match p.peek_kind(P) {
            Ok(TokenKind::Eq) => {
                p.consume_peeked();
                MetaKind::Equals(Literal::parse(p)?)
            }
            Ok(TokenKind::LParen) => MetaKind::Func(lit_tuple(p)?),
            _ => MetaKind::Plain,
        })
    }
}

// --- Items ---

impl Parse<'_> for Item {
    /// Parses an [Item]
    ///
    /// See also: [ItemKind::parse]
    fn parse(p: &mut Parser<'_>) -> PResult<Self> {
        let start = p.loc();
        Ok(Item {
            attrs: Attrs::parse(p)?,
            vis: Visibility::parse(p)?,
            kind: ItemKind::parse(p)?,
            span: Span(start, p.loc()),
        })
    }
}

impl Parse<'_> for ItemKind {
    /// Parses an [ItemKind]
    ///
    /// See also: [Item::parse]
    fn parse(p: &mut Parser) -> PResult<Self> {
        Ok(match p.peek_kind(Parsing::Item)? {
            TokenKind::Type => Alias::parse(p)?.into(),
            TokenKind::Const => Const::parse(p)?.into(),
            TokenKind::Static => Static::parse(p)?.into(),
            TokenKind::Mod => Module::parse(p)?.into(),
            TokenKind::Fn => Function::parse(p)?.into(),
            TokenKind::Struct => Struct::parse(p)?.into(),
            TokenKind::Enum => Enum::parse(p)?.into(),
            TokenKind::Impl => Impl::parse(p)?.into(),
            TokenKind::Use => Use::parse(p)?.into(),
            t => Err(p.error(Unexpected(t), Parsing::Item))?,
        })
    }
}

impl Parse<'_> for Generics {
    fn parse(p: &mut Parser<'_>) -> PResult<Self> {
        const P: Parsing = Parsing::Generics;
        let vars = match p.peek_kind(P)? {
            TokenKind::Lt => delim(
                sep(Sym::parse, TokenKind::Comma, TokenKind::Gt, P),
                (TokenKind::Lt, TokenKind::Gt),
                P,
            )(p)?,
            _ => Vec::new(),
        };
        Ok(Generics { vars })
    }
}

impl Parse<'_> for Alias {
    /// Parses a [`type` alias](Alias)
    fn parse(p: &mut Parser<'_>) -> PResult<Self> {
        const P: Parsing = Parsing::Alias;
        p.consume_peeked();

        let out = Ok(Alias {
            name: Sym::parse(p)?,
            from: if p.match_type(TokenKind::Eq, P).is_ok() {
                Some(Ty::parse(p)?.into())
            } else {
                None
            },
        });
        p.match_type(TokenKind::Semi, P)?;
        out
    }
}

impl Parse<'_> for Const {
    /// Parses a [compile-time constant](Const)
    fn parse(p: &mut Parser) -> PResult<Const> {
        const P: Parsing = Parsing::Const;
        p.consume_peeked();

        let out = Ok(Const {
            name: Sym::parse(p)?,
            ty: {
                p.match_type(TokenKind::Colon, P)?;
                Ty::parse(p)?.into()
            },
            init: {
                p.match_type(TokenKind::Eq, P)?;
                Expr::parse(p)?.into()
            },
        });
        p.match_type(TokenKind::Semi, P)?;
        out
    }
}

impl Parse<'_> for Static {
    /// Parses a [`static` item](Static)
    fn parse(p: &mut Parser) -> PResult<Static> {
        const P: Parsing = Parsing::Static;
        p.consume_peeked();

        let out = Ok(Static {
            mutable: Mutability::parse(p)?,
            name: Sym::parse(p)?,
            ty: {
                p.match_type(TokenKind::Colon, P)?;
                Ty::parse(p)?.into()
            },
            init: {
                p.match_type(TokenKind::Eq, P)?;
                Expr::parse(p)?.into()
            },
        });
        p.match_type(TokenKind::Semi, P)?;
        out
    }
}

impl Parse<'_> for Module {
    /// Parses a [Module]
    fn parse(p: &mut Parser<'_>) -> PResult<Self> {
        p.consume_peeked();

        Ok(Module {
            name: Sym::parse(p)?,
            file: {
                const P: Parsing = Parsing::ModuleKind;
                let inline = delim(Parse::parse, CURLIES, P);

                match p.peek_kind(P)? {
                    TokenKind::LCurly => Some(inline(p)?),
                    TokenKind::Semi => {
                        p.consume_peeked();
                        None
                    }
                    got => Err(p.error(ExpectedToken { want: TokenKind::Semi, got }, P))?,
                }
            },
        })
    }
}

impl Parse<'_> for Function {
    /// Parses a [Function] definition
    fn parse(p: &mut Parser) -> PResult<Function> {
        const P: Parsing = Parsing::Function;
        p.consume_peeked();

        let name = Sym::parse(p)?;
        let gens = Generics::parse(p)?;
        let (bind, types) = delim(FnSig::parse, PARENS, P)(p)?;
        let sign = TyFn {
            args: Box::new(match types.len() {
                0 => TyKind::Empty,
                _ => TyKind::Tuple(TyTuple { types }),
            }),
            rety: Ok(match p.match_type(TokenKind::Arrow, Parsing::TyFn) {
                Ok(_) => Some(Ty::parse(p)?),
                Err(_) => None,
            })?
            .map(Box::new),
        };
        Ok(Function {
            name,
            gens,
            sign,
            bind,
            body: match p.peek_kind(P)? {
                TokenKind::Semi => {
                    p.consume_peeked();
                    None
                }
                _ => Some(Expr::parse(p)?),
            },
        })
    }
}

type FnSig = (Pattern, Vec<TyKind>);

impl Parse<'_> for FnSig {
    /// Parses the [parameters](Param) associated with a Function
    fn parse(p: &mut Parser) -> PResult<FnSig> {
        const P: Parsing = Parsing::Function;
        let (mut params, mut types) = (vec![], vec![]);
        while Ok(TokenKind::RParen) != p.peek_kind(P) {
            let (param, ty) = TypedParam::parse(p)?;
            params.push(param);
            types.push(ty);
            if p.match_type(TokenKind::Comma, P).is_err() {
                break;
            }
        }
        Ok((Pattern::Tuple(params), types))
    }
}

type TypedParam = (Pattern, TyKind);

impl Parse<'_> for TypedParam {
    /// Parses a single function [parameter](Param)
    fn parse(p: &mut Parser) -> PResult<(Pattern, TyKind)> {
        Ok((
            Pattern::parse(p)?,
            if p.match_type(TokenKind::Colon, Parsing::Param).is_ok() {
                TyKind::parse(p)?
            } else {
                TyKind::Infer
            },
        ))
    }
}

impl Parse<'_> for Struct {
    /// Parses a [`struct` definition](Struct)
    fn parse(p: &mut Parser) -> PResult<Struct> {
        p.match_type(TokenKind::Struct, Parsing::Struct)?;
        Ok(Struct { name: Sym::parse(p)?, gens: Generics::parse(p)?, kind: StructKind::parse(p)? })
    }
}

impl Parse<'_> for StructKind {
    /// Parses the various [kinds of Struct](StructKind)
    fn parse(p: &mut Parser<'_>) -> PResult<Self> {
        const P: Parsing = Parsing::StructKind;
        Ok(match p.peek_kind(P) {
            Ok(TokenKind::LParen) => StructKind::Tuple(delim(
                sep(Ty::parse, TokenKind::Comma, PARENS.1, P),
                PARENS,
                P,
            )(p)?),
            Ok(TokenKind::LCurly) => StructKind::Struct(delim(
                sep(StructMember::parse, TokenKind::Comma, CURLIES.1, P),
                CURLIES,
                P,
            )(p)?),
            Ok(_) | Err(Error { reason: ErrorKind::EndOfInput, .. }) => StructKind::Empty,
            Err(e) => Err(e)?,
        })
    }
}

impl Parse<'_> for StructMember {
    /// Parses a single [StructMember]
    fn parse(p: &mut Parser) -> PResult<StructMember> {
        const P: Parsing = Parsing::StructMember;
        Ok(StructMember {
            vis: Visibility::parse(p)?,
            name: Sym::parse(p)?,
            ty: {
                p.match_type(TokenKind::Colon, P)?;
                Ty::parse(p)?
            },
        })
    }
}

impl Parse<'_> for Enum {
    /// Parses an [`enum`](Enum) definition
    fn parse(p: &mut Parser) -> PResult<Enum> {
        p.match_type(TokenKind::Enum, Parsing::Enum)?;
        Ok(Enum {
            name: Sym::parse(p)?,
            gens: Generics::parse(p)?,
            variants: {
                const P: Parsing = Parsing::EnumKind;
                match p.peek_kind(P)? {
                    TokenKind::LCurly => delim(
                        sep(Variant::parse, TokenKind::Comma, TokenKind::RCurly, P),
                        CURLIES,
                        P,
                    )(p)?,
                    t => Err(p.error(Unexpected(t), P))?,
                }
            },
        })
    }
}

impl Parse<'_> for Variant {
    /// Parses an [`enum`](Enum) [Variant]
    fn parse(p: &mut Parser) -> PResult<Variant> {
        let name = Sym::parse(p)?;
        let kind;
        let body;

        if p.match_type(TokenKind::Eq, Parsing::Variant).is_ok() {
            kind = StructKind::Empty;
            body = Some(Box::new(Expr::parse(p)?));
        } else {
            kind = StructKind::parse(p)?;
            body = None;
        }

        Ok(Variant { name, kind, body })
    }
}

impl Parse<'_> for Impl {
    fn parse(p: &mut Parser) -> PResult<Impl> {
        const P: Parsing = Parsing::Impl;
        p.match_type(TokenKind::Impl, P)?;

        Ok(Impl { target: ImplKind::parse(p)?, body: delim(File::parse, CURLIES, P)(p)? })
    }
}

impl Parse<'_> for ImplKind {
    fn parse(p: &mut Parser<'_>) -> PResult<Self> {
        const P: Parsing = Parsing::ImplKind;

        let target = Ty::parse(p)?;

        if p.match_type(TokenKind::For, P).is_err() {
            Ok(ImplKind::Type(target))
        } else if let TyKind::Path(impl_trait) = target.kind {
            Ok(ImplKind::Trait { impl_trait, for_type: Ty::parse(p)?.into() })
        } else {
            Err(Error {
                in_file: p.file,
                reason: ExpectedParsing { want: Parsing::Path },
                while_parsing: P,
                loc: target.span.head,
            })?
        }
    }
}

impl Parse<'_> for Use {
    fn parse(p: &mut Parser) -> PResult<Use> {
        const P: Parsing = Parsing::Use;
        p.match_type(TokenKind::Use, P)?;

        let absolute = p.match_type(TokenKind::ColonColon, P).is_ok();
        let tree = UseTree::parse(p)?;
        p.match_type(TokenKind::Semi, P)?;
        Ok(Use { tree, absolute })
    }
}

impl Parse<'_> for UseTree {
    fn parse(p: &mut Parser) -> PResult<Self> {
        const P: Parsing = Parsing::UseTree;
        // glob import
        Ok(match p.peek_kind(P)? {
            TokenKind::Star => {
                p.consume_peeked();
                UseTree::Glob
            }
            TokenKind::LCurly => UseTree::Tree(delim(
                sep(Parse::parse, TokenKind::Comma, CURLIES.1, P),
                CURLIES,
                P,
            )(p)?),
            TokenKind::Super | TokenKind::Identifier => {
                let name = PathPart::parse(p)?;
                if p.match_type(TokenKind::ColonColon, P).is_ok() {
                    UseTree::Path(name, Box::new(UseTree::parse(p)?))
                } else {
                    let PathPart::Ident(name) = name else {
                        Err(p.error(ErrorKind::ExpectedParsing { want: Parsing::Identifier }, P))?
                    };
                    if p.match_type(TokenKind::As, P).is_ok() {
                        UseTree::Alias(name, p.parse()?)
                    } else {
                        UseTree::Name(name)
                    }
                }
            }
            t => Err(p.error(Unexpected(t), Parsing::UseTree))?,
        })
    }
}

// --- Types ---

impl Parse<'_> for Ty {
    /// Parses a [Ty]
    ///
    /// See also: [TyKind::parse]
    fn parse(p: &mut Parser<'_>) -> PResult<Self> {
        let start = p.loc();
        Ok(Ty { kind: TyKind::parse(p)?, span: Span(start, p.loc()) })
    }
}

impl Parse<'_> for TyKind {
    /// Parses a [TyKind]
    ///
    /// See also: [Ty::parse]
    fn parse(p: &mut Parser) -> PResult<TyKind> {
        const P: Parsing = Parsing::TyKind;
        let out = match p.peek_kind(P)? {
            TokenKind::Bang => {
                p.consume_peeked();
                TyKind::Never
            }
            TokenKind::Amp | TokenKind::AmpAmp => TyRef::parse(p)?.into(),
            TokenKind::LBrack => {
                p.match_type(BRACKETS.0, Parsing::TySlice)?;
                let ty = TyKind::parse(p)?;
                let (out, kind) = match p.match_type(TokenKind::Semi, Parsing::TyArray).is_ok() {
                    true => {
                        let literal = p.match_type(TokenKind::Literal, Parsing::TyArray)?;
                        let &TokenData::Integer(count) = literal.data() else {
                            Err(p.error(Unexpected(TokenKind::Literal), Parsing::TyArray))?
                        };
                        (
                            TyKind::Array(TyArray { ty: Box::new(ty), count: count as _ }),
                            Parsing::TyArray,
                        )
                    }
                    false => (
                        TyKind::Slice(TySlice { ty: Box::new(ty) }),
                        Parsing::TySlice,
                    ),
                };
                p.match_type(BRACKETS.1, kind)?;
                out
            }
            TokenKind::LParen => {
                let out = TyTuple::parse(p)?;
                match out.types.is_empty() {
                    true => TyKind::Empty,
                    false => TyKind::Tuple(out),
                }
            }
            TokenKind::Fn => TyFn::parse(p)?.into(),
            path_like!() => {
                let path = Path::parse(p)?;
                if path.is_sinkhole() {
                    TyKind::Infer
                } else {
                    TyKind::Path(path)
                }
            }
            t => Err(p.error(Unexpected(t), P))?,
        };

        Ok(out)
    }
}

impl Parse<'_> for TyTuple {
    /// [TyTuple] = `(` ([Ty] `,`)* [Ty]? `)`
    fn parse(p: &mut Parser) -> PResult<TyTuple> {
        const P: Parsing = Parsing::TyTuple;
        Ok(TyTuple {
            types: delim(sep(TyKind::parse, TokenKind::Comma, PARENS.1, P), PARENS, P)(p)?,
        })
    }
}

impl Parse<'_> for TyRef {
    /// [TyRef] = (`&`|`&&`)* [Path]
    fn parse(p: &mut Parser) -> PResult<TyRef> {
        const P: Parsing = Parsing::TyRef;
        let mut count = 0;
        loop {
            match p.peek_kind(P)? {
                TokenKind::Amp => count += 1,
                TokenKind::AmpAmp => count += 2,
                _ => break,
            }
            p.consume_peeked();
        }
        Ok(TyRef { count, mutable: Mutability::parse(p)?, to: Box::new(Ty::parse(p)?) })
    }
}

impl Parse<'_> for TyFn {
    /// [TyFn] = `fn` [TyTuple] (-> [Ty])?
    fn parse(p: &mut Parser) -> PResult<TyFn> {
        const P: Parsing = Parsing::TyFn;
        p.match_type(TokenKind::Fn, P)?;

        let args = delim(sep(TyKind::parse, TokenKind::Comma, PARENS.1, P), PARENS, P)(p)?;

        Ok(TyFn {
            args: Box::new(match args {
                t if t.is_empty() => TyKind::Empty,
                types => TyKind::Tuple(TyTuple { types }),
            }),
            rety: match p.match_type(TokenKind::Arrow, Parsing::TyFn) {
                Ok(_) => Some(Ty::parse(p)?),
                Err(_) => None,
            }
            .map(Into::into),
        })
    }
}

// --- Paths ---

impl Parse<'_> for Path {
    /// Parses a [Path]
    ///
    /// See also: [PathPart::parse], [Sym::parse]
    ///
    /// [Path] = `::` *RelativePath*? | *RelativePath* \
    /// *RelativePath* = [PathPart] (`::` [PathPart])*
    fn parse(p: &mut Parser) -> PResult<Path> {
        const P: Parsing = Parsing::Path;
        let absolute = p.match_type(TokenKind::ColonColon, P).is_ok();
        let mut parts = vec![];

        if absolute {
            match PathPart::parse(p) {
                Ok(part) => parts.push(part),
                Err(_) => return Ok(Path { absolute, parts }),
            }
        } else {
            parts.push(PathPart::parse(p)?)
        };

        while p.match_type(TokenKind::ColonColon, Parsing::Path).is_ok() {
            parts.push(PathPart::parse(p)?)
        }

        Ok(Path { absolute, parts })
    }
}

impl Parse<'_> for PathPart {
    /// [PathPart] = `super` | `self` | [`Identifier`](TokenKind::Identifier)
    fn parse(p: &mut Parser) -> PResult<PathPart> {
        const P: Parsing = Parsing::PathPart;
        let out = match p.peek_kind(P)? {
            TokenKind::Super => PathPart::SuperKw,
            TokenKind::SelfTy => PathPart::SelfTy,
            TokenKind::Identifier => PathPart::Ident(Sym::parse(p)?),
            t => return Err(p.error(Unexpected(t), P)),
        };
        // Note: this relies on identifier not peeking
        p.consume_peeked();
        Ok(out)
    }
}

// --- Statements ---

impl Parse<'_> for Stmt {
    /// Parses a [Stmt]
    ///
    /// See also: [StmtKind::parse]
    fn parse(p: &mut Parser) -> PResult<Stmt> {
        let start = p.loc();
        Ok(Stmt {
            kind: StmtKind::parse(p)?,
            semi: match p.match_type(TokenKind::Semi, Parsing::Stmt) {
                Ok(_) => Semi::Terminated,
                _ => Semi::Unterminated,
            },
            span: Span(start, p.loc()),
        })
    }
}

impl Parse<'_> for StmtKind {
    /// Parses a [StmtKind]
    ///
    /// See also: [Stmt::parse]
    fn parse(p: &mut Parser) -> PResult<StmtKind> {
        Ok(match p.peek_kind(Parsing::StmtKind)? {
            TokenKind::Semi => StmtKind::Empty,
            item_like!() => Item::parse(p)?.into(),
            _ => Expr::parse(p)?.into(),
        })
    }
}

// --- Expressions ---

impl Parse<'_> for Expr {
    /// Parses an [Expr]
    fn parse(p: &mut Parser) -> PResult<Expr> {
        prec::expr(p, 0)
    }
}

impl Parse<'_> for Quote {
    fn parse(p: &mut Parser<'_>) -> PResult<Self> {
        let quote = delim(
            Expr::parse,
            (TokenKind::Grave, TokenKind::Grave),
            Parsing::ExprKind,
        )(p)?
        .into();
        Ok(Quote { quote })
    }
}

impl Parse<'_> for Let {
    fn parse(p: &mut Parser) -> PResult<Let> {
        p.consume_peeked();
        Ok(Let {
            mutable: Mutability::parse(p)?,
            name: Pattern::parse(p)?,
            ty: if p.match_type(TokenKind::Colon, Parsing::Let).is_ok() {
                Some(Ty::parse(p)?.into())
            } else {
                None
            },
            init: if p.match_type(TokenKind::Eq, Parsing::Let).is_ok() {
                Some(Expr::parse(p)?.into())
            } else {
                None
            },
        })
    }
}

impl Parse<'_> for MemberKind {
    fn parse(p: &mut Parser) -> PResult<MemberKind> {
        const P: Parsing = Parsing::Member;
        const DEL: (TokenKind, TokenKind) = PARENS; // delimiter
        match p.peek_kind(P)? {
            TokenKind::Identifier => {
                let name = Sym::parse(p)?;
                if p.match_type(DEL.0, P).is_err() {
                    Ok(MemberKind::Struct(name))
                } else {
                    let exprs = sep(Expr::parse, TokenKind::Comma, DEL.1, P)(p)?;
                    p.match_type(DEL.1, P)?; // should succeed
                    Ok(MemberKind::Call(name, Tuple { exprs }))
                }
            }
            TokenKind::Literal => {
                let name = Literal::parse(p)?; // TODO: Maybe restrict this to just
                Ok(MemberKind::Tuple(name))
            }
            t => Err(p.error(Unexpected(t), P)),
        }
    }
}

impl Parse<'_> for Fielder {
    /// [Fielder] = [`Identifier`](TokenKind::Identifier) (`:` [Expr])?
    fn parse(p: &mut Parser) -> PResult<Fielder> {
        const P: Parsing = Parsing::Fielder;
        Ok(Fielder {
            name: Sym::parse(p)?,
            init: match p.match_type(TokenKind::Colon, P) {
                Ok(_) => Some(Box::new(Expr::parse(p)?)),
                Err(_) => None,
            },
        })
    }
}

impl Parse<'_> for AddrOf {
    /// [AddrOf] = (`&`|`&&`)* [Expr]
    fn parse(p: &mut Parser) -> PResult<AddrOf> {
        const P: Parsing = Parsing::AddrOf;
        match p.peek_kind(P)? {
            TokenKind::Amp => {
                p.consume_peeked();
                Ok(AddrOf { mutable: Mutability::parse(p)?, expr: Expr::parse(p)?.into() })
            }
            TokenKind::AmpAmp => {
                let start = p.loc();
                p.consume_peeked();
                Ok(AddrOf {
                    mutable: Mutability::Not,
                    expr: Expr {
                        kind: ExprKind::AddrOf(AddrOf {
                            mutable: Mutability::parse(p)?,
                            expr: Expr::parse(p)?.into(),
                        }),
                        span: Span(start, p.loc()),
                    }
                    .into(),
                })
            }
            got => Err(p.error(ExpectedToken { want: TokenKind::Amp, got }, P)),
        }
    }
}

impl Parse<'_> for Block {
    /// [Block] = `{` [Stmt]* `}`
    fn parse(p: &mut Parser) -> PResult<Block> {
        const A_BLOCK: Parsing = Parsing::Block;
        Ok(Block { stmts: delim(rep(Parse::parse, CURLIES.1, A_BLOCK), CURLIES, A_BLOCK)(p)? })
    }
}

/// Conditions (which precede curly-braced blocks) get special treatment
fn condition(p: &mut Parser) -> PResult<Expr> {
    prec::expr(p, prec::Precedence::Condition.level())
}

impl Parse<'_> for While {
    /// [While] = `while` [Expr] [Block] [Else]?
    #[rustfmt::skip]
    fn parse(p: &mut Parser) -> PResult<While> {
        p.match_type(TokenKind::While, Parsing::While)?;
        Ok(While {
            cond: condition(p)?.into(),
            pass: Block::parse(p)?.into(),
            fail: Else::parse(p)?
        })
    }
}

impl Parse<'_> for If {
    /// [If] = <code>`if` [Expr] [Block] [Else]?</code>
    #[rustfmt::skip] // second line is barely not long enough
    fn parse(p: &mut Parser) -> PResult<If> {
        p.match_type(TokenKind::If, Parsing::If)?;
        Ok(If {
            cond: condition(p)?.into(),
            pass: Block::parse(p)?.into(),
            fail: Else::parse(p)?,
        })
    }
}

impl Parse<'_> for For {
    /// [For]: `for` [Pattern] `in` [Expr] [Block] [Else]?
    #[rustfmt::skip]
    fn parse(p: &mut Parser) -> PResult<For> {
        p.match_type(TokenKind::For, Parsing::For)?;
        let bind = Pattern::parse(p)?;
        p.match_type(TokenKind::In, Parsing::For)?;
        Ok(For {
            bind,
            cond: condition(p)?.into(),
            pass: Block::parse(p)?.into(),
            fail: Else::parse(p)?,
        })
    }
}

impl Parse<'_> for Else {
    /// [Else]: (`else` [Block])?
    fn parse(p: &mut Parser) -> PResult<Else> {
        match p.peek_kind(Parsing::Else) {
            Ok(TokenKind::Else) => {
                p.consume_peeked();
                Ok(Expr::parse(p)?.into())
            }
            Ok(_) | Err(Error { reason: EndOfInput, .. }) => Ok(None.into()),
            Err(e) => Err(e),
        }
    }
}

impl Parse<'_> for Break {
    /// [Break] = `break` (*unconsumed* `;` | [Expr])
    fn parse(p: &mut Parser) -> PResult<Break> {
        p.match_type(TokenKind::Break, Parsing::Break)?;
        Ok(Break { body: ret_body(p, Parsing::Break)? })
    }
}

impl Parse<'_> for Return {
    /// [Return] = `return` (*unconsumed* `;` | [Expr])
    fn parse(p: &mut Parser) -> PResult<Return> {
        p.match_type(TokenKind::Return, Parsing::Return)?;
        Ok(Return { body: ret_body(p, Parsing::Return)? })
    }
}

impl Parse<'_> for Pattern {
    fn parse(p: &mut Parser<'_>) -> PResult<Self> {
        let value = prec::expr(p, prec::Precedence::Pattern.level())?;
        Pattern::try_from(value).map_err(|e| p.error(InvalidPattern(e.into()), Parsing::Pattern))
    }
}

impl Parse<'_> for Match {
    /// [Match] = `match` [Expr] `{` [MatchArm],* `}`
    fn parse(p: &mut Parser<'_>) -> PResult<Self> {
        p.match_type(TokenKind::Match, Parsing::Match)?;
        Ok(Match {
            scrutinee: condition(p)?.into(),
            arms: delim(
                sep(MatchArm::parse, TokenKind::Comma, CURLIES.1, Parsing::Match),
                CURLIES,
                Parsing::Match,
            )(p)?,
        })
    }
}

impl Parse<'_> for MatchArm {
    /// [MatchArm] = [Pattern] `=>` [Expr]
    fn parse(p: &mut Parser<'_>) -> PResult<Self> {
        let pat = Pattern::parse(p)?;
        p.match_type(TokenKind::FatArrow, Parsing::MatchArm)?;
        let expr = Expr::parse(p)?;
        Ok(MatchArm(pat, expr))
    }
}

/// ret_body = (*unconsumed* `;` | [Expr])
fn ret_body(p: &mut Parser, while_parsing: Parsing) -> PResult<Option<Box<Expr>>> {
    Ok(match p.peek_kind(while_parsing)? {
        TokenKind::Semi => None,
        _ => Some(Expr::parse(p)?.into()),
    })
}
