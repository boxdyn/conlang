use super::*;
use crate::fmt::FmtAdapter;
use std::{fmt::Display, format_args as fmt};

impl<T: Display + Annotation, A: AstTypes> Display for At<T, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: Annotation, A: AstTypes> std::fmt::Debug for At<T, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <A::Annotation as std::fmt::Debug>::fmt(&self.1, f)?;
        f.write_str(" ")?;
        <T as std::fmt::Debug>::fmt(&self.0, f)
    }
}

impl<A: AstTypes> Display for Expr<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Omitted => "/* omitted */".fmt(f),
            Self::Id(id) => id.fmt(f),
            Self::MetId(id) => write!(f, "`{id}"),
            Self::Lit(literal) => literal.fmt(f),
            Self::Use(v) => write!(f, "use {v}"),
            Self::Bind(v) => v.fmt(f),
            Self::Make(v) => v.fmt(f),
            Self::Match(v) => v.fmt(f),

            Self::Op(op @ Op::Continue, exprs) => f.delimit(op, "").list(exprs, "!?,"),
            Self::Op(op @ (Op::If | Op::While), exprs) => match exprs.as_slice() {
                [cond, pass, At(Expr::Op(Op::Tuple, e), _)] if e.is_empty() => {
                    write!(f, "{op}{cond} {pass}")
                }
                [cond, pass, fail] => write!(f, "{op}{cond} {pass} else {fail}"),
                other => f.delimit(fmt!("({op}, "), ")").list(other, ", "),
            },
            Self::Op(Op::Array, exprs) => f.delimit("[", "]").list(exprs, ", "),
            Self::Op(Op::ArRep, exprs) => f.delimit("[", "]").list(exprs, "; "),
            Self::Op(Op::Block, exprs) => f
                .delimit_indented("{", "}")
                .list_wrap("\n", exprs, "\n", "\n"),
            Self::Op(Op::Tuple, exprs) => f.delimit("(", ")").list(exprs, ", "),
            Self::Op(Op::Group, exprs) => f.list(exprs, ", "),
            Self::Op(Op::MetaInner, exprs) => match exprs.as_slice() {
                [meta, expr @ ..] => f.delimit(fmt!("#![{meta}]\n"), "").list(expr, ","),
                [] => write!(f, "#![]"),
            },
            Self::Op(Op::MetaOuter, exprs) => match exprs.as_slice() {
                [meta, expr @ ..] => f.delimit(fmt!("#[{meta}]\n"), "").list(expr, ","),
                [] => write!(f, "#[]"),
            },

            Self::Op(op @ Op::Call, exprs) => match exprs.as_slice() {
                [callee, At(Expr::Op(Op::Tuple, args), _)] => {
                    f.delimit(fmt!("{callee}("), ")").list(args, ", ")
                }
                [callee, args @ ..] => f.delimit(fmt!("{callee}(?"), "?)").list(args, ", "),
                [] => write!(f, "{op}"),
            },
            Self::Op(op @ Op::Index, exprs) => match exprs.as_slice() {
                [callee, args @ ..] => f.delimit(fmt!("{callee}["), "]").list(args, ", "),
                [] => write!(f, "{op}"),
            },

            Self::Op(op @ Op::Do, exprs) => f.list(exprs, op),
            Self::Op(op @ Op::Macro, exprs) => f.delimit(op, "").list(exprs, " => "),
            Self::Op(op @ Op::Try, exprs) => f.delimit("(", fmt!("){op}")).list(exprs, ", "),
            Self::Op(op, exprs) => match exprs.as_slice() {
                [one] => write!(f, "{op}{one}"),
                many => f.delimit("(", ")").list(many, op),
            },
        }
    }
}

impl Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Op::Do => ";\n",
            Op::As => " as ",
            Op::Macro => "macro ",
            Op::Block => "{}",
            Op::Array => "[]",
            Op::ArRep => "; ",
            Op::Group => "()",
            Op::Tuple => "()",
            Op::MetaInner => "#![]",
            Op::MetaOuter => "#[]",
            Op::Try => "?",
            Op::Index => "",
            Op::Call => "",
            Op::Pub => "pub ",
            Op::Const => "const ",
            Op::Static => "static ",
            Op::Loop => "loop ",
            Op::If => "if ",
            Op::While => "while ",
            Op::Break => "break ",
            Op::Return => "return ",
            Op::Continue => "continue",
            Op::Dot => ".",
            Op::RangeEx => "..",
            Op::RangeIn => "..=",
            Op::Neg => "-",
            Op::Not => "!",
            Op::Identity => "!!",
            Op::Refer => "&",
            Op::Deref => "*",
            Op::Mul => " * ",
            Op::Div => " / ",
            Op::Rem => " % ",
            Op::Add => " + ",
            Op::Sub => " - ",
            Op::Shl => " << ",
            Op::Shr => " >> ",
            Op::And => " & ",
            Op::Xor => " ^ ",
            Op::Or => " | ",
            Op::Lt => " < ",
            Op::Leq => " <= ",
            Op::Eq => " == ",
            Op::Neq => " != ",
            Op::Geq => " >= ",
            Op::Gt => " > ",
            Op::LogAnd => " && ",
            Op::LogXor => " ^^ ",
            Op::LogOr => " || ",
            Op::Set => " = ",
            Op::MulSet => " *= ",
            Op::DivSet => " /= ",
            Op::RemSet => " %= ",
            Op::AddSet => " += ",
            Op::SubSet => " -= ",
            Op::ShlSet => " <<= ",
            Op::ShrSet => " >>= ",
            Op::AndSet => " &= ",
            Op::XorSet => " ^= ",
            Op::OrSet => " |= ",
        })
    }
}

impl<A: AstTypes> Display for Use<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Glob => "*".fmt(f),
            Self::Name(name) => name.fmt(f),
            Self::Alias(name, alias) => write!(f, "{name} as {alias}"),
            Self::Path(segment, rest) => write!(f, "{segment}::{rest}"),
            Self::Tree(items) => match items.len() {
                0 => "{}".fmt(f),
                1..=3 => f.delimit("{ ", " }").list(items, ", "),
                _ => f
                    .delimit_indented("{", "}")
                    .list_wrap("\n", items, ",\n", ",\n"),
            },
        }
    }
}

impl<A: AstTypes> Display for Bind<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(op, gens, pat, exprs) = self;
        op.fmt(f)?;
        if !gens.is_empty() {
            f.delimit("<", "> ").list(gens, ", ")?;
        }

        match (op, exprs.as_slice()) {
            (_, [At(Expr::Omitted, _)]) => write!(f, "{pat}"),
            (BindOp::Fn | BindOp::Mod | BindOp::Impl, [At(Expr::Op(Op::Block, _), _)]) => {
                f.delimit(fmt!("{pat} "), "").list(exprs, ",!? ")
            }
            (BindOp::Fn, _) => f.delimit(fmt!("{pat} = "), "").list(exprs, ""),
            (BindOp::Mod | BindOp::Impl, _) => f.delimit(fmt!("{pat} "), "").list(exprs, "!?;"),
            (BindOp::Struct | BindOp::Enum, _) => match pat {
                Pat::Op(PatOp::TypePrefixed, bind) => match bind.as_slice() {
                    [name, Pat::Op(PatOp::Record, parts)] => f
                        .delimit_indented(fmt!("{name} {{"), "}")
                        .list_wrap("\n", parts, ",\n", ",\n"),
                    [name, Pat::Op(PatOp::Tuple, parts)] => {
                        f.delimit(fmt!("{name}("), ")").list(parts, ", ")
                    }
                    _ => pat.fmt(f),
                },
                _ => pat.fmt(f),
            },
            (BindOp::For, [iter, pass, At(Expr::Op(Op::Tuple, e), _)]) if e.is_empty() => {
                write!(f, "{pat} in {iter} {pass}")
            }
            (BindOp::For, [iter, pass, fail]) => write!(f, "{pat} in {iter} {pass} else {fail}"),
            (BindOp::For, other) => f.delimit(fmt!("{pat} in [["), "]]!?").list(other, ", "),
            (_, []) => write!(f, "{pat}"),
            (_, [value]) => write!(f, "{pat} = {value}"),
            (_, [value, fail]) => write!(f, "{pat} = {value} else {fail}"),

            (_, other) => f.delimit(fmt!("{pat} ("), ")").list(other, ", "),
        }
    }
}

impl Display for BindOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Let => "let ",
            Self::Type => "type ",
            Self::Struct => "struct ",
            Self::Enum => "enum ",
            Self::Fn => "fn ",
            Self::Mod => "mod ",
            Self::Impl => "impl ",
            Self::For => "for ",
        })
    }
}

impl<A: AstTypes> Display for Make<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(expr, make_arms) = self;
        f.delimit(fmt!("({expr} {{"), "})").list(make_arms, ", ")
    }
}

impl<A: AstTypes> Display for MakeArm<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self(name, Some(body)) => write!(f, "{name}: {body}"),
            Self(name, None) => write!(f, "{name}"),
        }
    }
}

impl<A: AstTypes> Display for Match<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(scrutinee, arms) = self;
        f.delimit_indented(fmt!("match {scrutinee} {{"), "}")
            .list_wrap("\n", arms, ",\n", ",\n")
    }
}

impl<A: AstTypes> Display for MatchArm<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(pat, expr) = self;
        write!(f, "{pat} => {expr}")
    }
}

impl<A: AstTypes> Display for Pat<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ignore => "_".fmt(f),
            Self::Never => "!".fmt(f),
            Self::Value(literal) => literal.fmt(f),
            Self::MetId(name) => write!(f, "`{name}"),
            Self::Name(name) => name.fmt(f),
            Self::Op(PatOp::Record, pats) => f.delimit("{ ", " }").list(pats, ", "),
            Self::Op(PatOp::Tuple, pats) => f.delimit("(", ")").list(pats, ", "),
            Self::Op(PatOp::Slice, pats) => f.delimit("[", "]").list(pats, ", "),
            Self::Op(op @ PatOp::ArRep, pats) => f.delimit("[", "]").list(pats, op),
            Self::Op(op @ (PatOp::Typed | PatOp::Fn), pats) => match pats.as_slice() {
                [fun] => write!(f, "fn {fun}"), // TODO: reconsider this
                pats => f.list(pats, op),
            },
            Self::Op(op @ PatOp::Alt, pats) => f.list(pats, op),
            Self::Op(op @ PatOp::Generic, pats) => match pats.as_slice() {
                [] => op.fmt(f),
                [first, rest @ ..] => f.delimit(fmt!("{first}<"), ">").list(rest, ", "),
            },
            Self::Op(op @ PatOp::TypePrefixed, pats) => match pats.as_slice() {
                [] => op.fmt(f),
                [first, rest @ ..] => f.delimit(fmt!("{first} "), "").list(rest, ",? "),
            },
            Self::Op(op, pats) => match pats.as_slice() {
                [] => op.fmt(f),
                [rest] => write!(f, "{op}{rest}"),
                _ => f.delimit("(", ")").list(pats, op),
            },
        }
    }
}

impl Display for PatOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Pub => "pub ",
            Self::Mut => "mut ",
            Self::Ref => "&",
            Self::Ptr => "*",
            Self::Rest => "..",
            Self::RangeEx => "..",
            Self::RangeIn => "..=",
            Self::Record => ", ",
            Self::Tuple => ", ",
            Self::Slice => ", ",
            Self::ArRep => "; ",
            Self::Typed => ": ",
            Self::Generic => "T<>",
            Self::TypePrefixed => "T()",
            Self::Fn => " -> ",
            Self::Alt => " | ",
        })
    }
}
