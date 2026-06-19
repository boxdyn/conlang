use std::ops;

use cl_ast::{AstNode, At, Expr, Op, types::Literal};

pub trait ConstEval {
    fn const_eval(&self) -> Option<i128> {
        None
    }
}

impl<T: ConstEval + AstNode> ConstEval for At<T> {
    fn const_eval(&self) -> Option<i128> {
        self.0.const_eval()
    }
}

fn const_eval_bin<T: ConstEval>(op: Op, exprs: &[T]) -> Option<i128> {
    let [lhs, rhs] = exprs else { return None };
    let (lhs, rhs) = (lhs.const_eval()?, rhs.const_eval()?);
    match op {
        Op::Mul => Some(lhs * rhs),
        Op::Div => Some(lhs / rhs),
        Op::Rem => Some(lhs % rhs),
        Op::Add => Some(lhs + rhs),
        Op::Sub => Some(lhs - rhs),
        Op::Shl => Some(lhs << rhs as u32),
        Op::Shr => Some(lhs >> rhs as u32),
        Op::And => Some(lhs & rhs),
        Op::Xor => Some(lhs ^ rhs),
        Op::Or => Some(lhs | rhs),
        Op::Lt => None,
        Op::Leq => None,
        Op::Eq => None,
        Op::Neq => None,
        Op::Geq => None,
        Op::Gt => None,
        _ => unreachable!("eval_bin called with enby op {op} ({lhs}, {rhs})"),
    }
}

impl ConstEval for Expr {
    fn const_eval(&self) -> Option<i128> {
        match self {
            Self::Omitted => None,
            Self::Id(_) => todo!("Consteval paths"),
            Self::MetId(_) => None,
            Self::Lit(lit) => lit.const_eval(),
            Self::Use(_) => None,
            Self::Bind(_) => None,
            Self::Make(_) => None,
            Self::Match(_) => None,
            Self::Label(_) => None,
            Self::Op(op, ats) => match op {
                Op::Do => ats.last().and_then(At::const_eval),
                Op::As => ats.first().and_then(At::const_eval),
                Op::Block => ats.first().and_then(At::const_eval),
                Op::Array => todo!("Consteval {op}"),
                Op::ArRep => todo!("Consteval {op}"),
                Op::Group => ats.first().and_then(At::const_eval),
                Op::Tuple => todo!("Consteval {op}"),
                Op::MetaInner => ats.last().and_then(At::const_eval),
                Op::MetaOuter => ats.last().and_then(At::const_eval),
                Op::Try => ats.first().and_then(At::const_eval),
                Op::Index => todo!("Consteval {op}"),
                Op::Call => todo!("Consteval {op}"),
                Op::Pub => ats.first().and_then(At::const_eval),
                Op::Const => ats.first().and_then(At::const_eval),
                Op::Static => ats.first().and_then(At::const_eval),
                Op::Macro => todo!("Consteval {op}"),
                Op::Quote => todo!("Consteval {op}"),
                Op::Loop => todo!("Consteval {op}"),
                Op::If => todo!("Consteval {op}"),
                Op::While => todo!("Consteval {op}"),
                Op::Defer => todo!("Consteval {op}"),
                Op::Break => todo!("Consteval {op}"),
                Op::Return => todo!("Consteval {op}"),
                Op::Continue => todo!("Consteval {op}"),
                Op::Dot => todo!("Consteval {op}"),
                Op::RangeEx => todo!("Consteval {op}"),
                Op::RangeIn => todo!("Consteval {op}"),
                Op::Neg => ats.first().and_then(At::const_eval).map(ops::Neg::neg),
                Op::Not => ats.first().and_then(At::const_eval).map(ops::Not::not),
                Op::Identity => ats.first().and_then(At::const_eval),
                Op::Refer => todo!("Consteval {op}"),
                Op::Deref => todo!("Consteval {op}"),
                Op::Mul => const_eval_bin(*op, ats),
                Op::Div => const_eval_bin(*op, ats),
                Op::Rem => const_eval_bin(*op, ats),
                Op::Add => const_eval_bin(*op, ats),
                Op::Sub => const_eval_bin(*op, ats),
                Op::Shl => const_eval_bin(*op, ats),
                Op::Shr => const_eval_bin(*op, ats),
                Op::And => const_eval_bin(*op, ats),
                Op::Xor => const_eval_bin(*op, ats),
                Op::Or => const_eval_bin(*op, ats),
                Op::Lt => const_eval_bin(*op, ats),
                Op::Leq => const_eval_bin(*op, ats),
                Op::Eq => const_eval_bin(*op, ats),
                Op::Neq => const_eval_bin(*op, ats),
                Op::Geq => const_eval_bin(*op, ats),
                Op::Gt => const_eval_bin(*op, ats),
                Op::LogAnd => todo!("Consteval {op}"),
                Op::LogXor => todo!("Consteval {op}"),
                Op::LogOr => todo!("Consteval {op}"),
                Op::Set => todo!("Consteval {op}"),
                Op::MulSet => todo!("Consteval {op}"),
                Op::DivSet => todo!("Consteval {op}"),
                Op::RemSet => todo!("Consteval {op}"),
                Op::AddSet => todo!("Consteval {op}"),
                Op::SubSet => todo!("Consteval {op}"),
                Op::ShlSet => todo!("Consteval {op}"),
                Op::ShrSet => todo!("Consteval {op}"),
                Op::AndSet => todo!("Consteval {op}"),
                Op::XorSet => todo!("Consteval {op}"),
                Op::OrSet => todo!("Consteval {op}"),
            },
        }
    }
}

impl ConstEval for Literal {
    fn const_eval(&self) -> Option<i128> {
        match *self {
            Self::Bool(_) => None,
            Self::Char(_) => None,
            Self::Int(v, _) => Some(v as _),
            Self::Str(_) => None,
        }
    }
}
