use std::{cmp, ops};

use cl_ast::{AstNode, At, Expr, Op, types::Literal};

#[derive(Clone, Copy, Debug, PartialOrd, Ord)]
pub enum Value {
    Int(i128),
    UInt(u128),
    Bool(bool),
}

impl Value {
    pub fn int(&self) -> Option<i128> {
        match self {
            Self::UInt(i) => Some(*i as i128),
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }
    pub fn uint(&self) -> Option<u128> {
        match self {
            Self::UInt(i) => Some(*i),
            Self::Int(i) => Some(*i as u128),
            _ => None,
        }
    }
    pub fn bool(&self) -> Option<bool> {
        match self {
            Self::Int(i) => Some(*i != 0),
            Self::UInt(i) => Some(*i != 0),
            Self::Bool(b) => Some(*b),
        }
    }
}

impl From<i128> for Value {
    fn from(value: i128) -> Self {
        Self::Int(value)
    }
}
impl From<u128> for Value {
    fn from(value: u128) -> Self {
        Self::UInt(value)
    }
}
impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}
impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(v) => v.fmt(f),
            Value::UInt(v) => v.fmt(f),
            Value::Bool(v) => v.fmt(f),
        }
    }
}
impl cmp::PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::Int(_) | Self::UInt(_), _) => self.uint() == other.uint(),
            _ => false,
        }
    }
}
impl cmp::Eq for Value {}
impl ops::Neg for Value {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Value::Int(v) => Value::Int(-v),
            Value::UInt(v) => Value::UInt(!v + 1),
            Value::Bool(v) => Value::Bool(!v),
        }
    }
}
impl ops::Not for Value {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Value::Int(v) => Value::Int(!v),
            Value::UInt(v) => Value::UInt(!v),
            Value::Bool(v) => Value::Bool(!v),
        }
    }
}

pub trait ConstEval {
    fn const_eval(&self) -> Option<Value> {
        None
    }
}

impl<T: ConstEval + AstNode> ConstEval for At<T> {
    fn const_eval(&self) -> Option<Value> {
        self.0.const_eval()
    }
}

fn const_eval_bin<T: ConstEval>(op: Op, exprs: &[T]) -> Option<Value> {
    use Value::*;
    let [lhs, rhs] = exprs else { return None };
    let (lhs, rhs) = (lhs.const_eval()?, rhs.const_eval()?);
    match (op, lhs, rhs) {
        (Op::Mul, Int(lhs), Int(rhs)) => Some(Int(lhs.wrapping_mul(rhs))),
        (Op::Mul, UInt(lhs), UInt(rhs)) => Some(UInt(lhs.wrapping_mul(rhs))),
        (Op::Div, Int(lhs), Int(rhs)) => Some(Int(lhs.wrapping_div(rhs))),
        (Op::Div, UInt(lhs), UInt(rhs)) => Some(UInt(lhs.wrapping_div(rhs))),
        (Op::Rem, Int(lhs), Int(rhs)) => Some(Int(lhs.wrapping_rem(rhs))),
        (Op::Rem, UInt(lhs), UInt(rhs)) => Some(UInt(lhs.wrapping_rem(rhs))),
        (Op::Add, Int(lhs), Int(rhs)) => Some(Int(lhs.wrapping_add(rhs))),
        (Op::Add, UInt(lhs), UInt(rhs)) => Some(UInt(lhs.wrapping_add(rhs))),
        (Op::Sub, Int(lhs), Int(rhs)) => Some(Int(lhs.wrapping_sub(rhs))),
        (Op::Sub, UInt(lhs), UInt(rhs)) => Some(UInt(lhs.wrapping_sub(rhs))),
        (Op::Shl, Int(lhs), Int(rhs)) => Some(Int(lhs.wrapping_shl(rhs as u32))),
        (Op::Shl, UInt(lhs), UInt(rhs)) => Some(UInt(lhs.wrapping_shl(rhs as u32))),
        (Op::Shr, Int(lhs), Int(rhs)) => Some(Int(lhs.wrapping_shr(rhs as u32))),
        (Op::Shr, UInt(lhs), UInt(rhs)) => Some(UInt(lhs.wrapping_shr(rhs as u32))),
        (Op::And, Int(lhs), rhs) => Some(Int(lhs & rhs.int()?)),
        (Op::And, UInt(lhs), rhs) => Some(UInt(lhs & rhs.uint()?)),
        (Op::Xor, Int(lhs), rhs) => Some(Int(lhs ^ rhs.int()?)),
        (Op::Xor, UInt(lhs), rhs) => Some(UInt(lhs ^ rhs.uint()?)),
        (Op::Or, Int(lhs), rhs) => Some(Int(lhs | rhs.int()?)),
        (Op::Or, UInt(lhs), rhs) => Some(UInt(lhs | rhs.uint()?)),
        (Op::Lt, _, _) => None,
        (Op::Leq, _, _) => None,
        (Op::Eq, _, _) => Some(Bool(lhs == rhs)),
        (Op::Neq, _, _) => Some(Bool(lhs != rhs)),
        (Op::Geq, _, _) => None,
        (Op::Gt, _, _) => None,
        _ => unreachable!("eval_bin called with enby op {op} ({lhs}, {rhs})"),
    }
}

impl ConstEval for Expr {
    fn const_eval(&self) -> Option<Value> {
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
                Op::As => todo!("Consteval {op}"),
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
                Op::If => match ats.as_slice() {
                    [cond, pass, _] if cond.const_eval()?.bool()? => pass.const_eval(),
                    [_, _, fail] => fail.const_eval(),
                    _ => todo!("ConstEval {op}"),
                },
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
    fn const_eval(&self) -> Option<Value> {
        match *self {
            Self::Bool(v) => Some(v.into()),
            Self::Char(_) => None,
            Self::Int(v, _) => Some(v.into()),
            Self::Str(_) => None,
        }
    }
}
