//! The parser takes a stream of [`Token`]s from the [`Lexer`], and turns them into [`crate::ast`]
//! nodes.
use cl_ast::{
    types::{Literal, Path},
    *,
};
use cl_lexer::{LexError, LexFailure, Lexer};
use cl_structures::span::Span;
use cl_token::{Lexeme, TKind, Token};

pub trait Parse<'t> {
    type Prec: Copy + Default;

    fn parse(p: &mut Parser<'t>, _level: Self::Prec) -> PResult<Self>
    where Self: Sized;
}

pub mod expr;
pub mod pat;

pub mod error;
pub use error::{EOF, PResult, PResultExt, ParseError, no_eof};

/// Handles stateful extraction from a [Lexer], with single-[Token] lookahead.
#[derive(Debug)]
pub struct Parser<'t> {
    pub lexer: Lexer<'t>,
    pub next_tok: Option<PResult<Token>>,
    pub last_loc: Span,
    pub elide_do: bool,
}

impl<'t> Parser<'t> {
    /// Constructs a new Parser
    pub fn new(lexer: Lexer<'t>) -> Self {
        Self { last_loc: lexer.span(), lexer, next_tok: None, elide_do: false }
    }

    /// The identity function. This exists to make production chaining easier.
    pub const fn then<T>(&self, t: T) -> T {
        t
    }

    /// Gets the [struct@Span] of the last-consumed [Token]
    pub const fn span(&self) -> Span {
        self.last_loc
    }

    /// Parses a value that implements the [Parse] trait.
    pub fn parse<T: Parse<'t>>(&mut self, level: T::Prec) -> PResult<T> {
        Parse::parse(self, level)
    }

    /// Parses a value that implements the [Parse] trait, and asserts the entire input
    /// has been consumed.
    pub fn parse_entire<T: Parse<'t>>(&mut self, level: T::Prec) -> PResult<T> {
        let out = Parse::parse(self, level);
        match self.peek().allow_eof()? {
            Some(t) => Err(ParseError::ExpectedEOF(t.kind, t.span)),
            None => out,
        }
    }

    /// Peeks the next [`Token`]. Returns [`ParseError::FromLexer`] on lexer error.
    pub fn peek(&mut self) -> PResult<&Token> {
        let next_tok = match self.next_tok.take() {
            Some(tok) => tok,
            None => loop {
                match self.lexer.scan() {
                    Ok(Token { kind: TKind::Comment, .. }) => {}
                    Ok(tok) => break Ok(tok),
                    Err(LexError { pos, res: LexFailure::EOF }) => Err(ParseError::EOF(pos))?,
                    Err(e) => break Err(ParseError::FromLexer(e)),
                }
            },
        };
        let next_tok = self.next_tok.insert(next_tok);
        next_tok.as_ref().map_err(|e| *e)
    }

    /// Peeks the next token if it matches the `expected` [`TKind`]
    pub fn peek_if(&mut self, expected: TKind) -> PResult<Option<&Token>> {
        match self.peek() {
            Ok(tok) if tok.kind == expected => Ok(Some(tok)),
            Ok(_) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Consumes and returns the currently-peeked [Token].
    pub fn take(&mut self) -> PResult<Token> {
        let tok = self
            .next_tok
            .take()
            .unwrap_or(Err(ParseError::UnexpectedEOF(self.last_loc)));

        if let Ok(tok) = &tok {
            self.last_loc = tok.span;
            self.elide_do = matches!(
                tok.kind,
                TKind::RCurly | TKind::Semi | TKind::DotDot | TKind::DotDotEq
            )
        }

        tok
    }

    /// Consumes the currently-peeked [Token], returning its lexeme without cloning.
    pub fn take_lexeme(&mut self) -> PResult<Lexeme> {
        self.take().map(|tok| tok.lexeme)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> PResult<Token> {
        self.peek().no_eof()?;
        self.take() // .expect("should have token here")
    }

    /// Consumes and returns the next [`Token`] if it matches the `expected` [`TKind`]
    pub fn next_if(&mut self, expected: TKind) -> PResult<Result<Token, TKind>> {
        match self.peek() {
            Ok(t) if t.kind == expected => self.take().map(Ok),
            Ok(t) => Ok(Err(t.kind)),
            Err(e) => Err(e),
        }
    }

    /// Parses a list of P separated by `sep` tokens, ending in an `end` token.
    /// ```ignore
    /// List<T> = (T sep)* T? end ;
    /// ```
    pub fn list<P: Parse<'t>>(
        &mut self,
        mut elems: Vec<P>,
        level: P::Prec,
        sep: TKind,
        end: TKind,
    ) -> PResult<Vec<P>> {
        // TODO: This loses lexer errors
        while self.peek_if(end).no_eof()?.is_none() {
            elems.push(self.parse(level).no_eof()?);
            match self.peek_if(sep)? {
                Some(_) => self.consume(),
                None => break,
            };
        }
        let kind = self.peek().map(Token::kind)?;
        if kind == end {
            self.consume();
        } else if let Ok((first, _)) = kind.split()
            && first == end
        {
            self.split()?;
        } else {
            return Err(ParseError::Expected(end, kind, self.span()));
        }
        Ok(elems)
    }

    /// Parses a list of one or more P at level `level`, separated by `sep` tokens
    /// ```ignore
    /// UnterminatedList<P> = P (sep P)*
    /// ```
    pub fn list_bare<P: Parse<'t>>(
        &mut self,
        mut elems: Vec<P>,
        level: P::Prec,
        sep: TKind,
    ) -> PResult<Vec<P>> {
        loop {
            let elem = self.parse(level).no_eof()?;
            elems.push(elem);
            match self.peek_if(sep) {
                Ok(Some(_)) => self.consume(),
                Ok(None) | Err(ParseError::EOF(_)) => break Ok(elems),
                Err(e) => Err(e)?,
            };
        }
    }

    /// Parses into an [`Option<P>`] if the next token is `next`
    pub fn opt_if<P: Parse<'t>>(&mut self, level: P::Prec, next: TKind) -> PResult<Option<P>> {
        Ok(match self.next_if(next)? {
            Ok(_) => Some(self.parse(level).no_eof()?),
            Err(_) => None,
        })
    }

    /// Parses a P unless the next [Token]'s [TKind] is `end`
    pub fn opt<P: Parse<'t>>(&mut self, level: P::Prec, end: TKind) -> PResult<Option<P>> {
        let out = match self.peek_if(end)? {
            None => Some(self.parse(level).no_eof()?),
            Some(_) => None,
        };
        self.expect(end)?;
        Ok(out)
    }

    /// Ensures the next [Token]'s [TKind] is `next`
    pub fn expect(&mut self, next: TKind) -> PResult<&mut Self> {
        self.next_if(next)?
            .map_err(|tk| ParseError::Expected(next, tk, self.span()))?;
        Ok(self)
    }

    /// Consumes the currently peeked token without returning it.
    pub fn consume(&mut self) -> &mut Self {
        if self.next_tok.as_ref().is_some_and(|tok| tok.is_ok()) {
            let _ = self.take();
        }
        self
    }

    /// Consumes the next token, and attempts to split it into multiple.
    ///
    /// If the next token cannot be split, it will be returned.
    pub fn split(&mut self) -> PResult<Token> {
        let Token { lexeme, kind, span } = self.next()?;
        let kind = match kind.split() {
            Err(_) => kind,
            Ok((out, next)) => {
                self.next_tok = Some(Ok(Token { lexeme: lexeme.clone(), kind: next, span }));
                out
            }
        };
        Ok(Token { lexeme, kind, span })
    }
}

impl<'t> Parse<'t> for Path {
    type Prec = ();

    fn parse(p: &mut Parser<'t>, _level: Self::Prec) -> PResult<Self> {
        let mut parts = vec![];
        if p.next_if(TKind::ColonColon)?.is_ok() {
            parts.push("".into()); // the "root"
        }
        while let Ok(id) = p.next_if(TKind::Identifier)? {
            parts.push(
                id.lexeme
                    .str()
                    .expect("Identifier should have String")
                    .into(),
            );
            if let None | Some(Err(_)) = p.next_if(TKind::ColonColon).allow_eof()? {
                break;
            }
        }

        Ok(Path { parts })
    }
}

impl<'t> Parse<'t> for Literal {
    type Prec = ();
    fn parse(p: &mut Parser<'t>, _level: ()) -> PResult<Self> {
        let tok = p.peek()?;
        Ok(match tok.kind {
            TKind::True => p.consume().then(Literal::Bool(true)),
            TKind::False => p.consume().then(Literal::Bool(false)),
            TKind::Character | TKind::Integer | TKind::String => {
                match p.take().expect("should have Token after peek").lexeme {
                    Lexeme::String(str) => Literal::Str(str),
                    Lexeme::Integer(int, base) => Literal::Int(int, base),
                    Lexeme::Char(chr) => Literal::Char(chr),
                }
            }
            other => Err(ParseError::NotLiteral(other, tok.span))?,
        })
    }
}

impl<'t> Parse<'t> for Use {
    type Prec = ();

    fn parse(p: &mut Parser<'t>, _level: Self::Prec) -> PResult<Self> {
        let tok = p.next()?;
        Ok(match tok.kind {
            TKind::Star => p.then(Use::Glob),
            TKind::Identifier => {
                let name = tok.lexeme.str().expect("should have String").into();
                match p.peek().map(Token::kind).allow_eof()? {
                    Some(TKind::ColonColon) => Use::Path(name, p.consume().parse(())?),
                    Some(TKind::As) => Use::Alias(
                        name,
                        p.consume()
                            .next_if(TKind::Identifier)?
                            .map_err(|e| ParseError::Expected(TKind::Identifier, e, p.span()))?
                            .lexeme
                            .str()
                            .expect("Identifier should have string")
                            .into(),
                    ),
                    _ => Use::Name(name),
                }
            }
            TKind::LCurly => Use::Tree(p.list(vec![], (), TKind::Comma, TKind::RCurly)?),
            _ => Err(ParseError::NotUse(tok.kind, tok.span))?,
        })
    }
}

impl<'t, P: Parse<'t> + Annotation> Parse<'t> for At<P> {
    type Prec = P::Prec;
    fn parse(p: &mut Parser<'t>, level: P::Prec) -> PResult<Self>
    where Self: Sized {
        let start = p.span();
        Ok(At(p.parse(level)?, start.merge(p.span())))
    }
}

impl<'t, P: Parse<'t>> Parse<'t> for Box<P> {
    type Prec = P::Prec;
    fn parse(p: &mut Parser<'t>, level: P::Prec) -> PResult<Self>
    where Self: Sized {
        Ok(Box::new(p.parse(level)?))
    }
}
