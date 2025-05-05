use crate::{
    ast::{ExprKind as Ek, *},
    ast_visitor::{Fold, fold::or_fold_expr_kind},
};

pub struct ConstantFolder;

macro bin_rule(
    match ($kind: ident, $head: expr, $tail: expr) {
        $(($op:ident, $impl:expr, $($ty:ident -> $rety:ident),*)),*$(,)?
    }
) {
    #[allow(clippy::all)]
    match ($kind, $head, $tail) {
    $($((   BinaryKind::$op,
            Expr { kind: ExprKind::Literal(Literal::$ty(a)), .. },
            Expr { kind: ExprKind::Literal(Literal::$ty(b)), .. },
        ) => {
            ExprKind::Literal(Literal::$rety($impl(a, b)))
        },)*)*
        (kind, head, tail) => ExprKind::Binary(Binary {
            kind,
            parts: Box::new((head, tail)),
        }),
    }
}

macro un_rule(
    match ($kind: ident, $tail: expr) {
        $(($op:ident, $impl:expr, $($ty:ident),*)),*$(,)?
    }
) {
    match ($kind, $tail) {
    $($((UnaryKind::$op, Expr { kind: ExprKind::Literal(Literal::$ty(v)), .. }) => {
            ExprKind::Literal(Literal::$ty($impl(v)))
        },)*)*
        (kind, tail) => ExprKind::Unary(Unary { kind, tail: Box::new(tail) }),
    }
}

impl Fold for ConstantFolder {
    fn fold_expr_kind(&mut self, kind: Ek) -> Ek {
        match kind {
            Ek::Group(Group { expr }) => self.fold_expr_kind(expr.kind),
            Ek::Binary(Binary { kind, parts }) => {
                let (head, tail) = *parts;
                bin_rule! (match (kind, self.fold_expr(head), self.fold_expr(tail)) {
                    (Lt,     |a, b| a < b,  Bool -> Bool, Int -> Bool),
                    (LtEq,   |a, b| a <= b, Bool -> Bool, Int -> Bool),
                    (Equal,  |a, b| a == b, Bool -> Bool, Int -> Bool),
                    (NotEq,  |a, b| a != b, Bool -> Bool, Int -> Bool),
                    (GtEq,   |a, b| a >= b, Bool -> Bool, Int -> Bool),
                    (Gt,     |a, b| a > b,  Bool -> Bool, Int -> Bool),
                    (BitAnd, |a, b| a & b,  Bool -> Bool, Int -> Int),
                    (BitOr,  |a, b| a | b,  Bool -> Bool, Int -> Int),
                    (BitXor, |a, b| a ^ b,  Bool -> Bool, Int -> Int),
                    (Shl,    |a, b| a << b, Int -> Int),
                    (Shr,    |a, b| a >> b, Int -> Int),
                    (Add,    |a, b| a + b,  Int -> Int),
                    (Sub,    |a, b| a - b,  Int -> Int),
                    (Mul,    |a, b| a * b,  Int -> Int),
                    (Div,    |a, b| a / b,  Int -> Int),
                    (Rem,    |a, b| a % b,  Int -> Int),
                    // Cursed bit-smuggled float shenanigans
                    (Lt,     |a, b| (f64::from_bits(a) < f64::from_bits(b)),  Float -> Bool),
                    (LtEq,   |a, b| (f64::from_bits(a) >= f64::from_bits(b)), Float -> Bool),
                    (Equal,  |a, b| (f64::from_bits(a) == f64::from_bits(b)), Float -> Bool),
                    (NotEq,  |a, b| (f64::from_bits(a) != f64::from_bits(b)), Float -> Bool),
                    (GtEq,   |a, b| (f64::from_bits(a) <= f64::from_bits(b)), Float -> Bool),
                    (Gt,     |a, b| (f64::from_bits(a) > f64::from_bits(b)),  Float -> Bool),
                    (Add,    |a, b| (f64::from_bits(a) + f64::from_bits(b)).to_bits(),  Float -> Float),
                    (Sub,    |a, b| (f64::from_bits(a) - f64::from_bits(b)).to_bits(),  Float -> Float),
                    (Mul,    |a, b| (f64::from_bits(a) * f64::from_bits(b)).to_bits(),  Float -> Float),
                    (Div,    |a, b| (f64::from_bits(a) / f64::from_bits(b)).to_bits(),  Float -> Float),
                    (Rem,    |a, b| (f64::from_bits(a) % f64::from_bits(b)).to_bits(),  Float -> Float),
                })
            }
            Ek::Unary(Unary { kind, tail }) => {
                un_rule! (match (kind, self.fold_expr(*tail)) {
                    (Not, std::ops::Not::not, Int, Bool),
                    (Neg, std::ops::Not::not, Int, Bool),
                    (Neg, |f| (-f64::from_bits(f)).to_bits(), Float),
                    (At, std::ops::Not::not, Float), /* Lmao */
                })
            }
            _ => or_fold_expr_kind(self, kind),
        }
    }
}
