//! Conlang's [Expression](Expr) parser.

use super::{PResult, PResultExt, Parse, ParseError, Parser, no_eof, pat::Prec as PPrec};
use cl_ast::{types::Literal, *};
use cl_token::{TKind, Token};

/// Organizes the precedence hierarchy for syntactic elements
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Prec {
    Min,
    /// The Semicolon Operator gets its own precedence level
    Do,
    /// The body of a function, conditional, etc.
    Body,
    /// An assignment
    Assign,
    /// Constructor for a struct
    Make,
    /// Constructor for a tuple
    Tuple,
    /// The conditional of an `if` or `while` (which is really an `if`)
    Logical,
    /// The short-circuiting "boolean or" operator
    LogOr,
    /// The short-circuiting "boolean and" operator
    LogAnd,
    /// Value comparison operators
    Compare,
    /// Constructor for a Range
    Range,
    /// Binary/bitwise operators
    Binary,
    /// Bit-shifting operators
    Shift,
    /// Addition and Subtraction operators
    Factor,
    /// Multiplication, Division, and Remainder operators
    Term,
    /// Negation, Reference, Try
    Unary,
    /// Place-projection operators (`*x`, `x[1]`, `x.a`, `x.1`)
    Project,
    /// Call subscripting
    Extend,
    Max,
}

impl Prec {
    pub const MIN: usize = Prec::Min.value();

    pub const fn value(self) -> usize {
        self as usize * 2
    }

    pub const fn prev(self) -> usize {
        match self {
            Self::Assign => self.value() + 1,
            _ => self.value(),
        }
    }

    pub const fn next(self) -> usize {
        match self {
            Self::Assign => self.value(),
            _ => self.value() + 1,
        }
    }
}

/// `PseudoOperator`: fake operators used to give certain tokens special behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ps {
    Id,         // Identifier
    Mid,        // MetaIdentifier
    Lit,        // Literal
    Use,        // use Use
    Def,        // any definition (let, struct, enum, fn, ...)
    DocInner,   // Documentation Comment `//!`
    DocOuter,   // Documentation Comment `///`
    For,        // for Pat in Expr Expr else Expr
    Lambda0,    // || Expr
    Lambda,     // | Pat,* | Expr
    DoubleRef,  // && Expr
    Make,       // Expr{ Expr,* }
    Match,      // match Expr { (Pat => Expr),* }
    ImplicitDo, // An implicit semicolon
    Ellipsis,   // An ellipsis (...)
    End,        // Produces an empty value.
    Op(Op),     // A normal [ast::Op]
}

/// Tries to map the incoming [Token] to a prefix [expression operator](Op)
/// and its [precedence level](Prec)
fn from_prefix(token: &Token) -> PResult<(Ps, Prec)> {
    Ok(match token.kind {
        TKind::InDoc => (Ps::DocInner, Prec::Min),
        TKind::OutDoc => (Ps::DocOuter, Prec::Max),
        TKind::Do => (Ps::Op(Op::Do), Prec::Do),
        TKind::Semi => (Ps::End, Prec::Body),

        TKind::Identifier | TKind::ColonColon => (Ps::Id, Prec::Max),
        TKind::Dollar => (Ps::Mid, Prec::Max),
        TKind::True | TKind::False | TKind::Character | TKind::Integer | TKind::String => {
            (Ps::Lit, Prec::Max)
        }
        TKind::Use => (Ps::Use, Prec::Max),

        TKind::Pub => (Ps::Op(Op::Pub), Prec::Max),
        TKind::Const => (Ps::Op(Op::Const), Prec::Max),
        TKind::Static => (Ps::Op(Op::Static), Prec::Max),
        TKind::For => (Ps::For, Prec::Max),
        TKind::Match => (Ps::Match, Prec::Max),
        TKind::Macro => (Ps::Op(Op::Macro), Prec::Assign),

        TKind::Fn
        | TKind::Mod
        | TKind::Impl
        | TKind::Let
        | TKind::Type
        | TKind::Struct
        | TKind::Enum => (Ps::Def, Prec::Max),

        TKind::Loop => (Ps::Op(Op::Loop), Prec::Body),
        TKind::If => (Ps::Op(Op::If), Prec::Body),
        TKind::While => (Ps::Op(Op::While), Prec::Body),
        TKind::Defer => (Ps::Op(Op::Defer), Prec::Body),
        TKind::Break => (Ps::Op(Op::Break), Prec::Body),
        TKind::Return => (Ps::Op(Op::Return), Prec::Body),
        TKind::Continue => (Ps::Op(Op::Continue), Prec::Max),

        TKind::LCurly => (Ps::Op(Op::Block), Prec::Min),
        TKind::RCurly => (Ps::End, Prec::Do),
        TKind::LBrack => (Ps::Op(Op::Array), Prec::Tuple),
        TKind::RBrack => (Ps::End, Prec::Tuple),
        TKind::LParen => (Ps::Op(Op::Group), Prec::Min),
        TKind::RParen => (Ps::End, Prec::Tuple),
        TKind::Amp => (Ps::Op(Op::Refer), Prec::Unary),
        TKind::AmpAmp => (Ps::DoubleRef, Prec::Unary),
        TKind::Bang => (Ps::Op(Op::Not), Prec::Unary),
        TKind::BangBang => (Ps::Op(Op::Identity), Prec::Unary),
        TKind::Bar => (Ps::Lambda, Prec::Body),
        TKind::BarBar => (Ps::Lambda0, Prec::Body),
        TKind::DotDot => (Ps::Op(Op::RangeEx), Prec::Range),
        TKind::DotDotDot => (Ps::Ellipsis, Prec::Max),
        TKind::DotDotEq => (Ps::Op(Op::RangeIn), Prec::Range),
        TKind::Minus => (Ps::Op(Op::Neg), Prec::Unary),
        TKind::Plus => (Ps::Op(Op::Identity), Prec::Unary),
        TKind::Star => (Ps::Op(Op::Deref), Prec::Unary),
        TKind::Hash => (Ps::Op(Op::MetaOuter), Prec::Max),
        TKind::HashBang => (Ps::Op(Op::MetaInner), Prec::Max),

        kind => Err(ParseError::NotPrefix(kind, token.span))?,
    })
}

/// Tries to map the incoming [Token] to an infix [expression operator](Op)
/// and its [precedence level](Prec)
const fn from_infix(token: &Token) -> PResult<(Ps, Prec)> {
    Ok(match token.kind {
        TKind::Semi => (Ps::Op(Op::Do), Prec::Do), // the inspiration
        TKind::In => (Ps::Op(Op::Do), Prec::Do),

        TKind::Eq => (Ps::Op(Op::Set), Prec::Assign),
        TKind::StarEq => (Ps::Op(Op::MulSet), Prec::Assign),
        TKind::SlashEq => (Ps::Op(Op::DivSet), Prec::Assign),
        TKind::RemEq => (Ps::Op(Op::RemSet), Prec::Assign),
        TKind::PlusEq => (Ps::Op(Op::AddSet), Prec::Assign),
        TKind::MinusEq => (Ps::Op(Op::SubSet), Prec::Assign),
        TKind::LtLtEq => (Ps::Op(Op::ShlSet), Prec::Assign),
        TKind::GtGtEq => (Ps::Op(Op::ShrSet), Prec::Assign),
        TKind::AmpEq => (Ps::Op(Op::AndSet), Prec::Assign),
        TKind::XorEq => (Ps::Op(Op::XorSet), Prec::Assign),
        TKind::BarEq => (Ps::Op(Op::OrSet), Prec::Assign),
        TKind::Comma => (Ps::Op(Op::Tuple), Prec::Tuple),
        TKind::LCurly => (Ps::Make, Prec::Make),
        TKind::XorXor => (Ps::Op(Op::LogXor), Prec::Logical),
        TKind::BarBar => (Ps::Op(Op::LogOr), Prec::LogOr),
        TKind::AmpAmp => (Ps::Op(Op::LogAnd), Prec::LogAnd),
        TKind::Lt => (Ps::Op(Op::Lt), Prec::Compare),
        TKind::LtEq => (Ps::Op(Op::Leq), Prec::Compare),
        TKind::EqEq => (Ps::Op(Op::Eq), Prec::Compare),
        TKind::BangEq => (Ps::Op(Op::Neq), Prec::Compare),
        TKind::GtEq => (Ps::Op(Op::Geq), Prec::Compare),
        TKind::Gt => (Ps::Op(Op::Gt), Prec::Compare),
        TKind::DotDot => (Ps::Op(Op::RangeEx), Prec::Range),
        TKind::DotDotEq => (Ps::Op(Op::RangeIn), Prec::Range),
        TKind::Amp => (Ps::Op(Op::And), Prec::Binary),
        TKind::Xor => (Ps::Op(Op::Xor), Prec::Binary),
        TKind::Bar => (Ps::Op(Op::Or), Prec::Binary),
        TKind::LtLt => (Ps::Op(Op::Shl), Prec::Shift),
        TKind::GtGt => (Ps::Op(Op::Shr), Prec::Shift),
        TKind::Plus => (Ps::Op(Op::Add), Prec::Factor),
        TKind::Minus => (Ps::Op(Op::Sub), Prec::Factor),
        TKind::Star => (Ps::Op(Op::Mul), Prec::Term),
        TKind::Slash => (Ps::Op(Op::Div), Prec::Term),
        TKind::Rem => (Ps::Op(Op::Rem), Prec::Term),

        TKind::Question => (Ps::Op(Op::Try), Prec::Unary),
        TKind::Dot => (Ps::Op(Op::Dot), Prec::Project),
        TKind::LBrack => (Ps::Op(Op::Index), Prec::Project),
        TKind::LParen => (Ps::Op(Op::Call), Prec::Extend),

        TKind::RParen | TKind::RBrack | TKind::RCurly => (Ps::End, Prec::Max),
        TKind::As => (Ps::Op(Op::As), Prec::Unary),
        _ => (Ps::ImplicitDo, Prec::Do),
    })
}

impl<'t> Parse<'t> for Expr {
    type Prec = usize;

    /// Parses an [Expr]ession.
    ///
    /// The `level` parameter indicates the operator binding level of the expression.
    fn parse(p: &mut Parser<'t>, level: usize) -> PResult<Self> {
        const MIN: usize = Prec::MIN;

        // Prefix
        let tok @ &Token { kind, span, .. } = p.peek()?;
        let (op, prec) = from_prefix(tok)?;
        no_eof(move || {
            let mut head = match op {
                // "End" is produced when an "empty" expression is syntactically required.
                // This happens when a semi or closing delimiter begins an expression.
                // The token which emitted "End" cannot be consumed, as it is expected
                // elsewhere.
                Ps::End if level <= prec.next() => Expr::Omitted,
                Ps::End => Err(ParseError::NotPrefix(kind, span))?,

                Ps::Id => Expr::Id(p.parse(())?),
                Ps::Mid => Expr::MetId(p.consume().next()?.lexeme.to_string().as_str().into()),
                Ps::Lit => Expr::Lit(p.parse(())?),
                Ps::Use => Expr::Use(p.consume().parse(())?),
                Ps::Def => Expr::Bind(p.parse(())?),
                Ps::For => parse_for(p, ())?,
                Ps::Match => Expr::Match(p.parse(())?),
                Ps::Lambda | Ps::Lambda0 => {
                    p.split()?; // is either `||`, which can be split, or `|`, which can't

                    let args = p
                        .opt(PPrec::Tuple, TKind::Bar)?
                        .map(|At(pat, span): At<Pat>| pat.to_tuple(span).at(span))
                        .unwrap_or(At(Pat::Op(PatOp::Tuple, vec![]), span.merge(p.span())));

                    let rety = p
                        .opt_if(PPrec::Fn, TKind::Arrow)?
                        .unwrap_or(Pat::Ignore.at(p.span()));

                    Expr::Bind(Box::new(Bind(
                        BindOp::Fn,
                        vec![],
                        Pat::Op(PatOp::Fn, vec![args, rety]).at(span.merge(p.span())),
                        vec![p.parse(prec.next())?],
                    )))
                }
                Ps::DocOuter | Ps::DocInner => {
                    let comment = Literal::Str(p.take_lexeme()?.string().unwrap());
                    let comment = Expr::Lit(comment).at(span);
                    // TODO: collapse consecutive doc comments into one
                    let next = match p.peek().allow_eof()? {
                        Some(_) => p.parse(prec.next())?,
                        None => Expr::Omitted.at(span),
                    };
                    Expr::Op(
                        match op {
                            Ps::DocOuter => Op::MetaOuter,
                            _ => Op::MetaInner,
                        },
                        vec![comment, next],
                    )
                }
                Ps::Ellipsis => p.consume().then(Expr::Omitted),

                // Warning: the guard of this pattern modifies the parser.
                Ps::Op(Op::MetaOuter | Op::MetaInner)
                    if p.consume().expect(TKind::LBrack).is_err() =>
                {
                    return p.parse(level);
                }
                Ps::Op(op @ (Op::MetaOuter | Op::MetaInner)) => Expr::Op(
                    op,
                    vec![
                        p.opt(MIN, TKind::RBrack)?
                            .unwrap_or_else(|| Expr::Omitted.at(span)),
                        p.parse(prec.next())?,
                    ],
                ),
                Ps::Op(Op::Block) => Expr::Op(
                    Op::Block,
                    p.consume().opt(MIN, kind.flip())?.into_iter().collect(),
                ),
                Ps::Op(Op::Array) => parse_array(p)?,
                Ps::Op(Op::Group) => match p.consume().opt(MIN, kind.flip())? {
                    Some(value) => Expr::Op(Op::Group, vec![value]),
                    None => Expr::Op(Op::Tuple, vec![]),
                },
                Ps::Op(Op::Continue) => p.consume().then(Expr::Op(Op::Continue, vec![])),
                Ps::Op(op @ (Op::If | Op::While)) => {
                    p.consume();
                    let exprs = vec![
                        // conditional restricted to Logical operators or above
                        p.parse(Prec::Logical.value())?,
                        p.parse(prec.next())?,
                        match p.peek() {
                            Ok(Token { kind: TKind::Else, .. }) => {
                                p.consume().parse(prec.next())?
                            }
                            _ => Expr::Omitted.at(span.merge(p.span())),
                        },
                    ];
                    Expr::Op(op, exprs)
                }
                Ps::DoubleRef => {
                    p.split()?;
                    Expr::Op(Op::Refer, vec![p.parse(prec.next())?])
                }

                Ps::Op(op) => Expr::Op(op, vec![p.consume().parse(prec.next())?]),
                _ => unimplemented!("prefix {op:?}"),
            };

            // Infix and Postfix
            while let Ok(Some(tok @ &Token { kind, .. })) = p.peek().allow_eof()
                && let Ok((op, prec)) = from_infix(tok)
                && level <= prec.prev()
                && op != Ps::End
            {
                let span = span.merge(p.span());

                head = match op {
                    // Make (structor expressions) are context-sensitive
                    Ps::Make => match &head {
                        Expr::Id(_) | Expr::MetId(_) => Expr::Make(Box::new(Make(
                            head.at(span),
                            p.consume().list(vec![], (), TKind::Comma, TKind::RCurly)?,
                        ))),
                        _ => break,
                    },
                    // As is ImplicitDo (semicolon elision)
                    Ps::ImplicitDo if p.elide_do => head.and_do(span, p.parse(prec.next())?),
                    Ps::ImplicitDo => break,
                    // Allow `;` at end of file
                    Ps::Op(Op::Do) => head.and_do(
                        span,
                        match p.consume().peek().allow_eof()? {
                            Some(_) => p.parse(prec.next())?,
                            None => At(Expr::Omitted, p.span()),
                        },
                    ),
                    Ps::Op(Op::Index) => Expr::Op(
                        Op::Index,
                        p.consume()
                            .list(vec![head.at(span)], 0, TKind::Comma, TKind::RBrack)?,
                    ),
                    Ps::Op(Op::Call) => {
                        let head = head.at(span);
                        let args = match p.consume().opt::<At<Expr>>(0, TKind::RParen)? {
                            None => Expr::Op(Op::Tuple, vec![]).at(span),
                            Some(At(expr, span)) => expr.to_tuple(span).at(span),
                        };
                        Expr::Op(Op::Call, vec![head, args])
                    }
                    Ps::Op(op @ Op::Tuple) => Expr::Op(
                        op,
                        p.consume()
                            .list_bare(vec![head.at(span)], prec.next(), kind)?,
                    ) // then remove `...`s
                    .deomit(),
                    Ps::Op(op @ Op::Try) => {
                        p.consume();
                        Expr::Op(op, vec![head.at(span)])
                    }
                    Ps::Op(op) => {
                        Expr::Op(op, vec![head.at(span), p.consume().parse(prec.next())?])
                    }
                    _ => Err(ParseError::NotInfix(kind, span))?,
                }
            }

            Ok(head)
        })
    }
}

/// Parses an array with 0 or more elements, or an array-repetition
fn parse_array(p: &mut Parser<'_>) -> PResult<Expr> {
    if p.consume().peek()?.kind == TKind::RBrack {
        p.consume();
        return Ok(Expr::Op(Op::Array, vec![]));
    }

    let prec = Prec::Tuple;
    let item = p.parse(prec.value())?;
    let repeat = p.opt_if(prec.next(), TKind::Semi)?;
    p.expect(TKind::RBrack)?;

    Ok(match (repeat, item) {
        (Some(repeat), item) => Expr::Op(Op::ArRep, vec![item, repeat]),
        (None, At(Expr::Op(Op::Tuple, items), _)) => Expr::Op(Op::Array, items),
        (None, item) => Expr::Op(Op::Array, vec![item]),
    })
}

/// Parses a `match` expression
///
/// ```ignore
/// match scrutinee {
///     (Pat => Expr);*
/// }
/// ```
impl<'t> Parse<'t> for Match {
    type Prec = ();

    fn parse(p: &mut Parser<'t>, _level: Self::Prec) -> PResult<Self>
    where Self: Sized {
        Ok(Self(
            p.consume().parse(Prec::Tuple.value())?,
            p.expect(TKind::LCurly)?
                .list(vec![], (), TKind::Semi, TKind::RCurly)?,
        ))
    }
}

impl<'t> Parse<'t> for MatchArm {
    type Prec = ();

    fn parse(p: &mut Parser<'t>, _level: Self::Prec) -> PResult<Self>
    where Self: Sized {
        // <T,*>
        // let generics = match p.next_if(TKind::Lt)? {
        //     Ok(_) => p.list(vec![], (), TKind::Comma, TKind::Gt)?,
        //     Err(_) => vec![],
        // };

        // Pat
        let pat = p.parse(PPrec::Min)?;
        p.expect(TKind::FatArrow)?;
        let body = p.parse(Prec::Body.value())?;

        Ok(Self(pat, body))
    }
}

/// Parses a `for` loop expression
fn parse_for(p: &mut Parser<'_>, _level: ()) -> PResult<Expr> {
    // for Pat
    let pat = p.consume().parse(PPrec::Tuple)?;
    // in Expr
    let iter: At<Expr> = p.expect(TKind::In)?.parse(Prec::Logical.next())?;
    // Expr
    let pass: At<Expr> = p.parse(Prec::Body.next())?;
    let pspan = pass.1;
    // else Expr?
    let fail = match p.next_if(TKind::Else).allow_eof()? {
        Some(Ok(_)) => p.parse(Prec::Body.next())?,
        _ => Expr::Omitted.at(pspan),
    };
    /*
    TODO: desugar for into loop-match:
    for `pat in `iter `pass else `fail
    ==>
    match (`iter).into_iter() {
        #iter => loop match #iter.next() {
            None => break `fail,
            Some(`pat) => `pass,
        },
    }
    */

    Ok(Expr::Bind(Box::new(Bind(
        BindOp::For,
        vec![],
        pat,
        vec![iter, pass, fail],
    ))))
}

/// Returns the [BindOp], [pattern precedence](PPrec), [arrow TKind](TKind), [body precedence](Prec),
/// and [else precedence](Prec), (if applicable,) which controls the parsing of Bind expressions.
#[rustfmt::skip]
#[allow(clippy::type_complexity)]
fn from_bind(p: &mut Parser<'_>) -> PResult<(BindOp, PPrec, Option<TKind>, Option<Prec>, Option<Prec>)> {
    let bk = match p.peek()?.kind {
        // Token            Operator        Pat prec      Body Token             Body prec            Else prec
        TKind::Let =>    (BindOp::Let,    PPrec::Tuple, Some(TKind::Eq),       Some(Prec::Tuple),  Some(Prec::Body)),
        TKind::Type =>   (BindOp::Type,   PPrec::Alt,   Some(TKind::Eq),       Some(Prec::Tuple),  None),
        TKind::Struct => (BindOp::Struct, PPrec::Tuple, None,                  None,               None),
        TKind::Enum =>   (BindOp::Enum,   PPrec::Tuple, None,                  None,               None),
        TKind::Fn =>     (BindOp::Fn,     PPrec::Fn,    None,                  Some(Prec::Body),   None),
        TKind::Mod =>    (BindOp::Mod,    PPrec::Max,   None,                  Some(Prec::Body),   None),
        TKind::Impl =>   (BindOp::Impl,   PPrec::Fn,    None,                  Some(Prec::Body),   None),
        other => return Err(ParseError::NotBind(other, p.span()))
    };

    p.consume();
    Ok(bk)
}

impl<'t> Parse<'t> for Bind {
    type Prec = ();

    fn parse(p: &mut Parser<'t>, _level: Self::Prec) -> PResult<Self> {
        // let
        let (level, patp, arrow, bodyp, failp) = from_bind(p)?;

        // <T,*>
        let generics = match p.next_if(TKind::Lt)? {
            Ok(_) => p.list(vec![], (), TKind::Comma, TKind::Gt)?,
            Err(_) => vec![],
        };

        // Pat
        let pat = p.parse(patp)?;

        let Some(bodyp) = bodyp else {
            return Ok(Self(level, generics, pat, vec![]));
        };

        // `=>` for match, `=` for `let`, `type`
        if let Some(arrow) = arrow {
            if p.next_if(arrow).allow_eof()?.is_none_or(|v| v.is_err()) {
                return Ok(Self(level, generics, pat, vec![]));
            }
        } else {
            // Allow prefix `=`? for the rest of them
            p.next_if(TKind::Eq).allow_eof()?;
        }

        // `=` Expr
        let body = p.parse(bodyp.value())?;

        let Some(failp) = failp else {
            return Ok(Self(level, generics, pat, vec![body]));
        };

        // `else` Expr
        if p.next_if(TKind::Else)
            .allow_eof()?
            .is_none_or(|v| v.is_err())
        {
            return Ok(Self(level, generics, pat, vec![body]));
        }

        let fail = p.parse(failp.value())?;

        Ok(Self(level, generics, pat, vec![body, fail]))
    }
}

impl<'t> Parse<'t> for MakeArm {
    type Prec = ();

    fn parse(p: &mut Parser<'t>, _level: ()) -> PResult<Self> {
        let name = p
            .next_if(TKind::Identifier)?
            .map_err(|tk| ParseError::Expected(TKind::Identifier, tk, p.span()))?;
        Ok(MakeArm(
            name.lexeme
                .str()
                .expect("Identifier should have String")
                .into(),
            p.opt_if(Prec::Tuple.next(), TKind::Colon)?,
        ))
    }
}
