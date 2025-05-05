//! Converts between major enums and enum variants
use super::*;

impl<T: AsRef<str>> From<T> for PathPart {
    fn from(value: T) -> Self {
        match value.as_ref() {
            "super" => PathPart::SuperKw,
            ident => PathPart::Ident(ident.into()),
        }
    }
}

macro impl_from ($(impl From for $T:ty {$($from:ty => $to:expr),*$(,)?})*) {$($(
    impl From<$from> for $T {
        fn from(value: $from) -> Self {
            $to(value.into()) // Uses *tuple constructor*
        }
    }
    impl From<Box<$from>> for $T {
        fn from(value: Box<$from>) -> Self {
            $to((*value).into())
        }
    }
)*)*}

impl_from! {
    impl From for ItemKind {
        Alias => ItemKind::Alias,
        Const => ItemKind::Const,
        Static => ItemKind::Static,
        Module => ItemKind::Module,
        Function => ItemKind::Function,
        Struct => ItemKind::Struct,
        Enum => ItemKind::Enum,
        Impl => ItemKind::Impl,
        Use => ItemKind::Use,
    }
    impl From for StructKind {
        Vec<Ty> => StructKind::Tuple,
        // TODO: Struct members in struct
    }
    impl From for VariantKind {
        u128 => VariantKind::CLike,
        Ty => VariantKind::Tuple,
        // TODO: enum struct variants
    }
    impl From for TyKind {
        Path => TyKind::Path,
        TyTuple => TyKind::Tuple,
        TyRef => TyKind::Ref,
        TyFn => TyKind::Fn,
    }
    impl From for StmtKind {
        Item => StmtKind::Item,
        Expr => StmtKind::Expr,
    }
    impl From for ExprKind {
        Let => ExprKind::Let,
        Quote => ExprKind::Quote,
        Match => ExprKind::Match,
        Assign => ExprKind::Assign,
        Modify => ExprKind::Modify,
        Binary => ExprKind::Binary,
        Unary => ExprKind::Unary,
        Cast => ExprKind::Cast,
        Member => ExprKind::Member,
        Index => ExprKind::Index,
        Path => ExprKind::Path,
        Literal => ExprKind::Literal,
        Array => ExprKind::Array,
        ArrayRep => ExprKind::ArrayRep,
        AddrOf => ExprKind::AddrOf,
        Block => ExprKind::Block,
        Group => ExprKind::Group,
        Tuple => ExprKind::Tuple,
        While => ExprKind::While,
        If => ExprKind::If,
        For => ExprKind::For,
        Break => ExprKind::Break,
        Return => ExprKind::Return,
    }
    impl From for Literal {
        bool => Literal::Bool,
        char => Literal::Char,
        u128 => Literal::Int,
        String => Literal::String,
    }
}

impl From<Option<Expr>> for Else {
    fn from(value: Option<Expr>) -> Self {
        Self { body: value.map(Into::into) }
    }
}
impl From<Expr> for Else {
    fn from(value: Expr) -> Self {
        Self { body: Some(value.into()) }
    }
}

impl TryFrom<Expr> for Pattern {
    type Error = Expr;

    /// Performs the conversion. On failure, returns the *first* non-pattern subexpression.
    fn try_from(value: Expr) -> Result<Self, Self::Error> {
        Ok(match value.kind {
            ExprKind::Literal(literal) => Pattern::Literal(literal),
            ExprKind::Path(Path { absolute: false, ref parts }) => match parts.as_slice() {
                [PathPart::Ident(name)] => Pattern::Name(*name),
                _ => Err(value)?,
            },
            ExprKind::Empty => Pattern::Tuple(vec![]),
            ExprKind::Group(Group { expr }) => Pattern::Tuple(vec![Pattern::try_from(*expr)?]),
            ExprKind::Tuple(Tuple { exprs }) => Pattern::Tuple(
                exprs
                    .into_iter()
                    .map(Pattern::try_from)
                    .collect::<Result<_, _>>()?,
            ),
            ExprKind::AddrOf(AddrOf { mutable, expr }) => {
                Pattern::Ref(mutable, Box::new(Pattern::try_from(*expr)?))
            }
            ExprKind::Array(Array { values }) => Pattern::Array(
                values
                    .into_iter()
                    .map(Pattern::try_from)
                    .collect::<Result<_, _>>()?,
            ),
            ExprKind::Binary(Binary { kind: BinaryKind::Call, parts }) => {
                let (Expr { kind: ExprKind::Path(path), .. }, args) = *parts else {
                    return Err(parts.0);
                };
                match args.kind {
                    ExprKind::Empty | ExprKind::Tuple(_) => {}
                    _ => return Err(args),
                }
                let Pattern::Tuple(args) = Pattern::try_from(args)? else {
                    unreachable!("Arguments should be convertible to pattern!")
                };
                Pattern::TupleStruct(path, args)
            }
            ExprKind::Binary(Binary { kind: BinaryKind::RangeExc, parts }) => {
                let (head, tail) = (Pattern::try_from(parts.0)?, Pattern::try_from(parts.1)?);
                Pattern::RangeExc(head.into(), tail.into())
            }
            ExprKind::Binary(Binary { kind: BinaryKind::RangeInc, parts }) => {
                let (head, tail) = (Pattern::try_from(parts.0)?, Pattern::try_from(parts.1)?);
                Pattern::RangeInc(head.into(), tail.into())
            }
            ExprKind::Unary(Unary { kind: UnaryKind::RangeExc, tail }) => {
                Pattern::Rest(Some(Pattern::try_from(*tail)?.into()))
            }
            ExprKind::Structor(Structor { to, init }) => {
                let fields = init
                    .into_iter()
                    .map(|Fielder { name, init }| {
                        Ok((name, init.map(|i| Pattern::try_from(*i)).transpose()?))
                    })
                    .collect::<Result<_, Self::Error>>()?;
                Pattern::Struct(to, fields)
            }
            _ => Err(value)?,
        })
    }
}
