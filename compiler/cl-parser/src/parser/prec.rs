//! Parses an [ExprKind] using a modified pratt parser
//!
//! See also: [Expr::parse], [ExprKind::parse]
//!
//! Implementer's note: [ExprKind::parse] is the public API for parsing [ExprKind]s.
//! Do not call it from within this function.

use super::{Parse, *};

/// Parses an [ExprKind]
pub fn expr(p: &mut Parser, power: u8) -> PResult<Expr> {
    let parsing = Parsing::ExprKind;
    let start = p.loc();
    // Prefix expressions
    let mut head = Expr {
        kind: match p.peek_kind(Parsing::Unary)? {
            literal_like!() => Literal::parse(p)?.into(),
            path_like!() => exprkind_pathlike(p)?,
            TokenKind::Amp | TokenKind::AmpAmp => AddrOf::parse(p)?.into(),
            TokenKind::Grave => Quote::parse(p)?.into(),
            TokenKind::LCurly => Block::parse(p)?.into(),
            TokenKind::LBrack => exprkind_arraylike(p)?,
            TokenKind::LParen => exprkind_tuplelike(p)?,
            TokenKind::Let => Let::parse(p)?.into(),
            TokenKind::Match => Match::parse(p)?.into(),
            TokenKind::While => ExprKind::While(While::parse(p)?),
            TokenKind::If => ExprKind::If(If::parse(p)?),
            TokenKind::For => ExprKind::For(For::parse(p)?),
            TokenKind::Break => ExprKind::Break(Break::parse(p)?),
            TokenKind::Return => ExprKind::Return(Return::parse(p)?),
            TokenKind::Continue => {
                p.consume_peeked();
                ExprKind::Continue
            }

            op => {
                let (kind, prec) =
                    from_prefix(op).ok_or_else(|| p.error(Unexpected(op), parsing))?;
                let ((), after) = prec.prefix().expect("should have a precedence");
                p.consume_peeked();
                Unary { kind, tail: expr(p, after)?.into() }.into()
            }
        },
        span: Span(start, p.loc()),
    };

    fn from_postfix(op: TokenKind) -> Option<Precedence> {
        Some(match op {
            TokenKind::LBrack => Precedence::Index,
            TokenKind::LParen => Precedence::Call,
            TokenKind::LCurly => Precedence::Structor,
            TokenKind::Dot => Precedence::Member,
            TokenKind::As => Precedence::Cast,
            _ => None?,
        })
    }

    while let Ok(op) = p.peek_kind(parsing) {
        // Postfix expressions
        if let Some((before, ())) = from_postfix(op).and_then(Precedence::postfix) {
            if before < power {
                break;
            }

            head = Expr {
                kind: match op {
                    TokenKind::LBrack => {
                        p.consume_peeked();
                        let indices =
                            sep(Expr::parse, TokenKind::Comma, TokenKind::RBrack, parsing)(p)?;
                        p.match_type(TokenKind::RBrack, parsing)?;
                        ExprKind::Index(Index { head: head.into(), indices })
                    }
                    TokenKind::LParen => {
                        p.consume_peeked();
                        let exprs =
                            sep(Expr::parse, TokenKind::Comma, TokenKind::RParen, parsing)(p)?;
                        p.match_type(TokenKind::RParen, parsing)?;
                        Binary {
                            kind: BinaryKind::Call,
                            parts: (
                                head,
                                Expr { kind: Tuple { exprs }.into(), span: Span(start, p.loc()) },
                            )
                                .into(),
                        }
                        .into()
                    }
                    TokenKind::LCurly => match head.kind {
                        ExprKind::Path(path) => ExprKind::Structor(structor_body(p, path)?),
                        _ => break,
                    },
                    TokenKind::Dot => {
                        p.consume_peeked();
                        let kind = MemberKind::parse(p)?;
                        Member { head: Box::new(head), kind }.into()
                    }
                    TokenKind::As => {
                        p.consume_peeked();
                        let ty = Ty::parse(p)?;
                        Cast { head: head.into(), ty }.into()
                    }
                    _ => Err(p.error(Unexpected(op), parsing))?,
                },
                span: Span(start, p.loc()),
            };
            continue;
        }
        // infix expressions
        if let Some((kind, prec)) = from_infix(op) {
            let (before, after) = prec.infix().expect("should have a precedence");
            if before < power {
                break;
            }
            p.consume_peeked();

            let tail = expr(p, after)?;
            head = Expr {
                kind: Binary { kind, parts: (head, tail).into() }.into(),
                span: Span(start, p.loc()),
            };
            continue;
        }

        if let Some((kind, prec)) = from_modify(op) {
            let (before, after) = prec.infix().expect("should have a precedence");
            if before < power {
                break;
            }
            p.consume_peeked();

            let tail = expr(p, after)?;
            head = Expr {
                kind: Modify { kind, parts: (head, tail).into() }.into(),
                span: Span(start, p.loc()),
            };
            continue;
        }

        if let TokenKind::Eq = op {
            let (before, after) = Precedence::Assign
                .infix()
                .expect("should have a precedence");
            if before < power {
                break;
            }
            p.consume_peeked();

            let tail = expr(p, after)?;
            head = Expr {
                kind: Assign { parts: (head, tail).into() }.into(),
                span: Span(start, p.loc()),
            };

            continue;
        }

        if let TokenKind::As = op {
            let before = Precedence::Cast.level();
            if before < power {
                break;
            }
            p.consume_peeked();

            let ty = Ty::parse(p)?;
            head = Expr { kind: Cast { head: head.into(), ty }.into(), span: Span(start, p.loc()) };

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
fn exprkind_arraylike(p: &mut Parser) -> PResult<ExprKind> {
    const P: Parsing = Parsing::Array;
    const START: TokenKind = TokenKind::LBrack;
    const END: TokenKind = TokenKind::RBrack;

    p.match_type(START, P)?;
    let out = match p.peek_kind(P)? {
        END => Array { values: vec![] }.into(),
        _ => exprkind_array_rep(p)?,
    };
    p.match_type(END, P)?;
    Ok(out)
}

/// [ArrayRep] = `[` [Expr] `;` [Expr] `]`
fn exprkind_array_rep(p: &mut Parser) -> PResult<ExprKind> {
    const P: Parsing = Parsing::Array;
    const END: TokenKind = TokenKind::RBrack;

    let first = Expr::parse(p)?;
    Ok(match p.peek_kind(P)? {
        TokenKind::Semi => ArrayRep {
            value: first.into(),
            repeat: {
                p.consume_peeked();
                let value = p.match_type(TokenKind::Literal, Parsing::ArrayRep)?;
                match value.data() {
                    TokenData::Integer(size) => *size as usize,
                    _ => {
                        Err(p.error(ErrorKind::Unexpected(TokenKind::Literal), Parsing::ArrayRep))?
                    }
                }
            },
        }
        .into(),
        TokenKind::RBrack => Array { values: vec![first] }.into(),
        TokenKind::Comma => Array {
            values: {
                p.consume_peeked();
                let mut out = vec![first];
                out.extend(sep(Expr::parse, TokenKind::Comma, END, P)(p)?);
                out
            },
        }
        .into(),
        ty => Err(p.error(Unexpected(ty), P))?,
    })
}

/// [Group] = `(`([Empty](ExprKind::Empty)|[Expr]|[Tuple])`)`
///
/// [ExprKind::Empty] and [Group] are special cases of [Tuple]
fn exprkind_tuplelike(p: &mut Parser) -> PResult<ExprKind> {
    p.match_type(TokenKind::LParen, Parsing::Group)?;
    let out = match p.peek_kind(Parsing::Group)? {
        TokenKind::RParen => Ok(ExprKind::Empty),
        _ => exprkind_group(p),
    };
    p.match_type(TokenKind::RParen, Parsing::Group)?;
    out
}

/// [Group] = `(`([Empty](ExprKind::Empty)|[Expr]|[Tuple])`)`
fn exprkind_group(p: &mut Parser) -> PResult<ExprKind> {
    let first = Expr::parse(p)?;
    match p.peek_kind(Parsing::Group)? {
        TokenKind::Comma => {
            let mut exprs = vec![first];
            p.consume_peeked();
            while TokenKind::RParen != p.peek_kind(Parsing::Tuple)? {
                exprs.push(Expr::parse(p)?);
                match p.peek_kind(Parsing::Tuple)? {
                    TokenKind::Comma => p.consume_peeked(),
                    _ => break,
                };
            }
            Ok(Tuple { exprs }.into())
        }
        _ => Ok(Group { expr: first.into() }.into()),
    }
}

/// Parses an expression beginning with a [Path] (i.e. [Path] or [Structor])
fn exprkind_pathlike(p: &mut Parser) -> PResult<ExprKind> {
    Path::parse(p).map(Into::into)
}

/// [Structor]Body = `{` ([Fielder] `,`)* [Fielder]? `}`
fn structor_body(p: &mut Parser, to: Path) -> PResult<Structor> {
    let init = delim(
        sep(
            Fielder::parse,
            TokenKind::Comma,
            CURLIES.1,
            Parsing::Structor,
        ),
        CURLIES,
        Parsing::Structor,
    )(p)?;

    Ok(Structor { to, init })
}

/// Precedence provides a total ordering among operators
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Precedence {
    Assign,
    Pattern,   // A pattern can contain a structor
    Structor,  // A structor is never a valid conditional
    Condition, // Anything that syntactically needs a block following it
    Logic,
    Compare,
    Range,
    Bitwise,
    Shift,
    Factor,
    Term,
    Unary,
    Index,
    Cast,
    Member, // left-associative
    Call,
    Deref,
}

impl Precedence {
    #[inline]
    pub const fn level(self) -> u8 {
        (self as u8) << 1
    }

    pub fn prefix(self) -> Option<((), u8)> {
        match self {
            Self::Assign => Some(((), self.level())),
            Self::Unary => Some(((), self.level())),
            Self::Deref => Some(((), self.level())),
            _ => None,
        }
    }

    pub fn infix(self) -> Option<(u8, u8)> {
        let level = self.level();
        match self {
            Self::Unary => None,
            Self::Assign => Some((level + 1, level)),
            _ => Some((level, level + 1)),
        }
    }

    pub fn postfix(self) -> Option<(u8, ())> {
        match self {
            Self::Structor | Self::Index | Self::Call | Self::Member | Self::Cast => {
                Some((self.level(), ()))
            }
            _ => None,
        }
    }
}

impl From<ModifyKind> for Precedence {
    fn from(_value: ModifyKind) -> Self {
        Precedence::Assign
    }
}

impl From<BinaryKind> for Precedence {
    fn from(value: BinaryKind) -> Self {
        use BinaryKind as Op;
        match value {
            Op::Call => Precedence::Call,
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
            Op::Loop => Precedence::Assign,
            Op::Deref => Precedence::Deref,
            _ => Precedence::Unary,
        }
    }
}

/// Creates helper functions for turning TokenKinds into AST operators
macro operator($($name:ident ($takes:ident => $returns:ident) {$($t:ident => $p:ident),*$(,)?};)*) {$(
    pub fn $name (value: $takes) -> Option<($returns, Precedence)> {
        match value {
            $($takes::$t => Some(($returns::$p, Precedence::from($returns::$p))),)*
            _ => None?,
        }
    })*
}

operator! {
    from_prefix (TokenKind => UnaryKind) {
        Loop => Loop,
        Star => Deref,
        Minus => Neg,
        Bang => Not,
        DotDot => RangeExc,
        DotDotEq => RangeInc,
        At => At,
        Tilde => Tilde,
    };

    from_modify(TokenKind => ModifyKind) {
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

    from_infix (TokenKind => BinaryKind) {
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
    };
}
