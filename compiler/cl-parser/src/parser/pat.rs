//! Conlang's [Pattern](Pat) parser.

use super::{PResult, PResultExt, Parse, ParseError, Parser, expr::Prec as ExPrec};
use cl_ast::{
    types::{Literal, Path},
    *,
};
use cl_token::{TKind, Token};

/// Precedence levels of value and type pattern expressions.
///
/// Lower (toward [Prec::Min]) precedence levels can contain
/// all higher (toward [Prec::Max]) precedence levels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Prec {
    /// The lowest precedence
    #[default]
    Min,
    /// "Alternate" pattern: `Pat | Pat`
    Alt,
    /// Tuple pattern: `Pat,+`
    Tuple,
    /// Type annotation: `Pat : Pat`
    Typed,
    /// Function signature: `foo(bar: baz, ..) -> qux`
    Fn,
    /// Range pattern: `Pat .. Pat`, `Pat ..= Pat`
    Range,
    /// The highest precedence
    Max,
}

macro_rules! intify {
    ($enum:ident($value:path) = $min:ident, $max: ident, $($variant:ident),*$(,)?) => {
        #[expect(non_upper_case_globals)] {
        const $min: u32 = $enum::$min as _;
        const $max: u32 = $enum::$max as _;
        $(const $variant: u32 = $enum::$variant as _;)*
        match $value {
            ..=$min => $enum::$min,
            $($variant => $enum::$variant,)*
            $max.. => $enum::$max,
        }
    }};
}

impl Prec {
    pub const fn from_int(value: u32) -> Self {
        intify! {Prec(value) = Min, Max, Alt, Tuple, Typed, Fn, Range}
    }

    /// Returns the level of precedence higher than this one
    pub const fn next(self) -> Self {
        Self::from_int(self as u32 + 1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prefix {
    /// Consume (and disregard) this prefix token
    Consume,
    Underscore,
    Never,
    MetId,
    DocInner,
    DocOuter,
    Id,
    Array,
    Constant,
    Op(PatOp),
    Split(PatOp),
}

fn from_prefix(token: &Token) -> PResult<(Prefix, Prec)> {
    Ok(match token.kind {
        TKind::True
        | TKind::False
        | TKind::Character
        | TKind::Integer
        | TKind::String
        | TKind::Minus
        | TKind::Const => (Prefix::Constant, Prec::Max),
        TKind::InDoc => (Prefix::DocInner, Prec::Typed),
        TKind::OutDoc => (Prefix::DocOuter, Prec::Typed),
        TKind::Hash => (Prefix::Op(PatOp::MetaOuter), Prec::Typed),
        TKind::HashBang => (Prefix::Op(PatOp::MetaInner), Prec::Typed),
        TKind::Identifier if token.lexeme.str() == Some("_") => (Prefix::Underscore, Prec::Max),
        TKind::ColonColon | TKind::Identifier => (Prefix::Id, Prec::Max),
        TKind::Bang => (Prefix::Never, Prec::Max),
        TKind::Amp => (Prefix::Op(PatOp::Ref), Prec::Fn),
        TKind::AmpAmp => (Prefix::Split(PatOp::Ref), Prec::Fn),
        TKind::Star => (Prefix::Op(PatOp::Ptr), Prec::Max),
        TKind::Mut => (Prefix::Op(PatOp::Mut), Prec::Typed),
        TKind::Pub => (Prefix::Op(PatOp::Pub), Prec::Typed),
        TKind::Dollar => (Prefix::MetId, Prec::Max),

        TKind::Fn => (Prefix::Op(PatOp::Fn), Prec::Fn),
        TKind::Bar => (Prefix::Consume, Prec::Alt),
        TKind::DotDot => (Prefix::Op(PatOp::Rest), Prec::Max),
        TKind::DotDotEq => (Prefix::Op(PatOp::RangeIn), Prec::Max),
        TKind::LCurly => (Prefix::Op(PatOp::Record), Prec::Typed),
        TKind::LParen => (Prefix::Op(PatOp::Tuple), Prec::Fn),
        TKind::LBrack => (Prefix::Array, Prec::Max),
        kind => Err(ParseError::NotPrefix(kind, token.span))?,
    })
}

/// Tries to map the incoming Token to a [pattern operator](PatOp)
/// and its following [precedence level](Prec)
fn from_infix(token: &Token) -> Option<(PatOp, Prec)> {
    Some(match token.kind {
        TKind::Arrow => (PatOp::Fn, Prec::Fn),
        TKind::Bar => (PatOp::Alt, Prec::Alt),
        TKind::Comma => (PatOp::Tuple, Prec::Tuple),
        TKind::Colon => (PatOp::Typed, Prec::Typed),
        TKind::DotDot => (PatOp::RangeEx, Prec::Range),
        TKind::DotDotEq => (PatOp::RangeIn, Prec::Range),
        TKind::If => (PatOp::Guard, Prec::Fn),
        TKind::LCurly => (PatOp::TypePrefixed, Prec::Fn),
        TKind::LBrack => (PatOp::TypePrefixed, Prec::Fn),
        TKind::LParen => (PatOp::TypePrefixed, Prec::Fn),
        TKind::Lt => (PatOp::Generic, Prec::Fn),
        _ => None?,
    })
}

impl<'t> Parse<'t> for Pat {
    type Prec = Prec;

    fn parse(p: &mut Parser<'t>, level: Prec) -> PResult<Self> {
        let tok @ &Token { kind, span, .. } = p.peek()?;
        let (op, prec) = from_prefix(tok)?;

        let mut head = match op {
            Prefix::Consume => p.consume().parse(level)?,
            Prefix::Underscore => p.consume().then(Pat::Ignore),
            Prefix::Never => p.consume().then(Pat::Never),
            Prefix::MetId => Pat::MetId(p.consume().next()?.lexeme.to_string().as_str().into()),
            Prefix::Constant => Pat::Value(p.parse(ExPrec::Unary.value())?),
            Prefix::Array => parse_array_pat(p)?,
            Prefix::Id => {
                let At(mut path, span): At<Path> = p.parse(())?;
                // Name or Value pattern?
                match path.parts.len() {
                    1 => Pat::Name(path.parts.pop().expect("name has 1 part")),
                    _ => Pat::Value(Box::new(At(Expr::Id(path), span))),
                }
            }
            Prefix::Op(op @ (PatOp::Record | PatOp::Tuple)) => Pat::Op(
                op,
                p.consume()
                    .list(vec![], Prec::Tuple.next(), TKind::Comma, kind.flip())?,
            ),
            Prefix::Op(op @ (PatOp::Rest | PatOp::RangeEx | PatOp::RangeIn)) => {
                // next token must continue a pattern
                match p.consume().peek().allow_eof()? {
                    Some(tok) if from_prefix(tok).is_ok() => Pat::Op(op, vec![p.parse(prec)?]),
                    _ => Pat::Op(op, vec![]),
                }
            }

            Prefix::DocOuter | Prefix::DocInner => {
                let comment = Literal::Str(p.take_lexeme()?.string().unwrap());
                let comment = Expr::Lit(comment).at(span);
                Pat::Op(
                    match op {
                        Prefix::DocOuter => PatOp::MetaOuter,
                        _ => PatOp::MetaInner,
                    },
                    vec![Pat::Value(comment.into()).at(span), p.parse(prec.next())?],
                )
            }
            Prefix::Op(op @ (PatOp::MetaOuter | PatOp::MetaInner)) => Pat::Op(
                op,
                vec![
                    Pat::Value(Box::new(
                        p.consume()
                            .expect(TKind::LBrack)?
                            .opt(ExPrec::MIN, TKind::RBrack)?
                            .unwrap_or_else(|| Expr::Op(Op::Tuple, vec![]).at(span)),
                    ))
                    .at(span),
                    p.parse(prec)?,
                ],
            ),
            Prefix::Op(op) => Pat::Op(op, vec![p.consume().parse(prec)?]),
            Prefix::Split(op) => {
                p.split()?;
                Pat::Op(op, vec![p.parse(prec)?])
            }
        };

        while let Ok(Some(tok @ &Token { kind, span: end, .. })) = p.peek().allow_eof()
            && let Some((op, prec)) = from_infix(tok)
            && level <= prec
        {
            let span = span.merge(end);
            head = match op {
                PatOp::RangeEx | PatOp::RangeIn => Pat::Op(
                    op,
                    if let Some(tok) = p.consume().peek().allow_eof()?
                        && from_prefix(tok).is_ok()
                    {
                        vec![head.at(span), p.parse(prec)?]
                    } else {
                        vec![head.at(span)]
                    },
                ),
                PatOp::Guard => {
                    let At(cond, cspan) = p.consume().parse(ExPrec::Logical.value())?;
                    let cond = Pat::Value(Box::new(At(cond, cspan))).at(cspan);
                    Pat::Op(op, vec![head.at(span), cond])
                }
                PatOp::Generic => Pat::Op(
                    PatOp::Generic,
                    p.consume().list(
                        vec![head.at(span)],
                        Prec::Typed,
                        TKind::Comma,
                        kind.flip(),
                    )?,
                ),
                PatOp::TypePrefixed => match prefix_level(&head, level, tok) {
                    Some(_prec) => add_typeprefix(p, head.at(span), Prec::Typed)?,
                    _ => break,
                },
                PatOp::Tuple => Pat::Op(
                    op,
                    p.consume()
                        .list_bare(vec![head.at(span)], prec.next(), kind)?,
                ),
                PatOp::Fn => Pat::Op(op, vec![head.at(span), p.consume().parse(prec)?]),
                _ => Pat::Op(op, vec![head.at(span), p.consume().parse(prec.next())?]),
            }
        }
        Ok(head)
    }
}

fn prefix_level(pat: &Pat, level: Prec, tok: &Token) -> Option<Prec> {
    let (_, prec) = from_prefix(tok).ok()?;
    match pat {
        Pat::Name(_) | Pat::Value(_) | Pat::Op(PatOp::Generic, _) if level <= prec => Some(prec),
        _ => None,
    }
}

fn add_typeprefix(p: &mut Parser<'_>, name: At<Pat>, level: Prec) -> PResult<Pat> {
    let At(mut name, span) = name;
    name = Pat::Op(
        PatOp::TypePrefixed,
        vec![name.at(span.merge(p.span())), p.parse(level)?],
    );
    Ok(name)
}

fn parse_array_pat(p: &mut Parser<'_>) -> PResult<Pat> {
    if p.consume().peek()?.kind == TKind::RBrack {
        p.consume();
        return Ok(Pat::Op(PatOp::Slice, vec![]));
    }

    let item = p.parse(Prec::Tuple)?;
    let repeat = p.opt_if(Prec::Tuple, TKind::Semi)?;
    p.expect(TKind::RBrack)?;

    Ok(match (repeat, item) {
        (Some(repeat), item) => Pat::Op(PatOp::ArRep, vec![item, repeat]),
        (None, At(Pat::Op(PatOp::Tuple, items), ..)) => Pat::Op(PatOp::Slice, items),
        (None, item) => Pat::Op(PatOp::Slice, vec![item]),
    })
}
