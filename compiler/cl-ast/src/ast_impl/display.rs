//! Implements [Display] for [AST](super::super) Types

use super::*;
use format::{delimiters::*, *};
use std::{
    borrow::Borrow,
    fmt::{Display, Write},
};

fn separate<I: Display, W: Write>(
    iterable: impl IntoIterator<Item = I>,
    sep: &'static str,
) -> impl FnOnce(W) -> std::fmt::Result {
    move |mut f| {
        for (idx, item) in iterable.into_iter().enumerate() {
            if idx > 0 {
                f.write_str(sep)?;
            }
            write!(f, "{item}")?;
        }
        Ok(())
    }
}

impl Display for Mutability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mutability::Not => Ok(()),
            Mutability::Mut => "mut ".fmt(f),
        }
    }
}

impl Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Visibility::Private => Ok(()),
            Visibility::Public => "pub ".fmt(f),
        }
    }
}

impl Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Bool(v) => v.fmt(f),
            Literal::Char(v) => write!(f, "'{}'", v.escape_debug()),
            Literal::Int(v) => v.fmt(f),
            Literal::Float(v) => write!(f, "{:?}", f64::from_bits(*v)),
            Literal::String(v) => write!(f, "\"{}\"", v.escape_debug()),
        }
    }
}

impl Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        separate(&self.items, "\n\n")(f)
    }
}

impl Display for Attrs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { meta } = self;
        if meta.is_empty() {
            return Ok(());
        }
        "#".fmt(f)?;
        separate(meta, ", ")(&mut f.delimit(INLINE_SQUARE))?;
        "\n".fmt(f)
    }
}

impl Display for Meta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { name, kind } = self;
        write!(f, "{name}{kind}")
    }
}

impl Display for MetaKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetaKind::Plain => Ok(()),
            MetaKind::Equals(v) => write!(f, " = {v}"),
            MetaKind::Func(args) => separate(args, ", ")(f.delimit(INLINE_PARENS)),
        }
    }
}

impl Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { span: _, attrs, vis, kind } = self;
        attrs.fmt(f)?;
        vis.fmt(f)?;
        kind.fmt(f)
    }
}

impl Display for ItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ItemKind::Alias(v) => v.fmt(f),
            ItemKind::Const(v) => v.fmt(f),
            ItemKind::Static(v) => v.fmt(f),
            ItemKind::Module(v) => v.fmt(f),
            ItemKind::Function(v) => v.fmt(f),
            ItemKind::Struct(v) => v.fmt(f),
            ItemKind::Enum(v) => v.fmt(f),
            ItemKind::Impl(v) => v.fmt(f),
            ItemKind::Use(v) => v.fmt(f),
        }
    }
}

impl Display for Generics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Generics { vars } = self;
        if !vars.is_empty() {
            separate(vars, ", ")(f.delimit_with("<", ">"))?
        }
        Ok(())
    }
}

impl Display for Alias {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { name, from } = self;
        match from {
            Some(from) => write!(f, "type {name} = {from};"),
            None => write!(f, "type {name};"),
        }
    }
}

impl Display for Const {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { name, ty, init } = self;
        write!(f, "const {name}: {ty} = {init}")
    }
}

impl Display for Static {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { mutable, name, ty, init } = self;
        write!(f, "static {mutable}{name}: {ty} = {init}")
    }
}

impl Display for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { name, file } = self;
        write!(f, "mod {name}")?;
        match file {
            Some(items) => {
                ' '.fmt(f)?;
                write!(f.delimit(BRACES), "{items}")
            }
            None => Ok(()),
        }
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { name, gens, sign: sign @ TyFn { args, rety }, bind, body } = self;
        let types = match args.kind {
            TyKind::Tuple(TyTuple { ref types }) => types.as_slice(),
            TyKind::Empty => Default::default(),
            _ => {
                write!(f, "Invalid function signature: {sign}")?;
                Default::default()
            }
        };
        let bind = match bind {
            Pattern::Tuple(patterns) => patterns.as_slice(),
            _ => {
                write!(f, "Invalid argument binder: {bind}")?;
                Default::default()
            }
        };

        debug_assert_eq!(bind.len(), types.len());
        write!(f, "fn {name}{gens} ")?;
        {
            let mut f = f.delimit(INLINE_PARENS);

            for (idx, (arg, ty)) in bind.iter().zip(types.iter()).enumerate() {
                if idx != 0 {
                    f.write_str(", ")?;
                }
                write!(f, "{arg}: {ty}")?;
            }
        }
        if TyKind::Empty != rety.kind {
            write!(f, " -> {rety}")?
        }

        match body {
            Some(body) => write!(f, " {body}"),
            None => ';'.fmt(f),
        }
    }
}

impl Display for Struct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { name, gens, kind } = self;
        write!(f, "struct {name}{gens}{kind}")
    }
}

impl Display for StructKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StructKind::Empty => ';'.fmt(f),
            StructKind::Tuple(v) => separate(v, ", ")(f.delimit(INLINE_PARENS)),
            StructKind::Struct(v) => separate(v, ",\n")(f.delimit(SPACED_BRACES)),
        }
    }
}

impl Display for StructMember {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { vis, name, ty } = self;
        write!(f, "{vis}{name}: {ty}")
    }
}

impl Display for Enum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { name, gens, variants } = self;
        write!(f, "enum {name}{gens}")?;
        separate(variants, ",\n")(f.delimit(SPACED_BRACES))
    }
}

impl Display for Variant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { name, kind, body } = self;
        write!(f, "{name}{kind}")?;
        match body {
            Some(body) => write!(f, " {body}"),
            None => Ok(()),
        }
    }
}

impl Display for Impl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { gens, target, body } = self;
        write!(f, "impl{gens} {target} ")?;
        write!(f.delimit(BRACES), "{body}")
    }
}

impl Display for ImplKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImplKind::Type(t) => t.fmt(f),
            ImplKind::Trait { impl_trait, for_type } => {
                write!(f, "{impl_trait} for {for_type}")
            }
        }
    }
}

impl Display for Use {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { absolute, tree } = self;
        f.write_str(if *absolute { "use ::" } else { "use " })?;
        write!(f, "{tree};")
    }
}

impl Display for UseTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UseTree::Tree(tree) => separate(tree, ", ")(f.delimit(INLINE_BRACES)),
            UseTree::Path(path, rest) => write!(f, "{path}::{rest}"),
            UseTree::Alias(path, name) => write!(f, "{path} as {name}"),
            UseTree::Name(name) => write!(f, "{name}"),
            UseTree::Glob => write!(f, "*"),
        }
    }
}

impl Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { span: _, kind, gens } = self;
        write!(f, "{kind}{gens}")
    }
}

impl Display for TyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TyKind::Never => "!".fmt(f),
            TyKind::Empty => "()".fmt(f),
            TyKind::Infer => "_".fmt(f),
            TyKind::Path(v) => v.fmt(f),
            TyKind::Array(v) => v.fmt(f),
            TyKind::Slice(v) => v.fmt(f),
            TyKind::Tuple(v) => v.fmt(f),
            TyKind::Ref(v) => v.fmt(f),
            TyKind::Ptr(v) => v.fmt(f),
            TyKind::Fn(v) => v.fmt(f),
        }
    }
}

impl Display for TyArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { ty, count } = self;
        write!(f, "[{ty}; {count}]")
    }
}

impl Display for TySlice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { ty } = self;
        write!(f, "[{ty}]")
    }
}

impl Display for TyTuple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        separate(&self.types, ", ")(f.delimit(INLINE_PARENS))
    }
}

impl Display for TyRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let &Self { count, mutable, ref to } = self;
        for _ in 0..count {
            f.write_char('&')?;
        }
        write!(f, "{mutable}{to}")
    }
}

impl Display for TyPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { to } = self;
        write!(f, "*{to}")
    }
}

impl Display for TyFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { args, rety } = self;
        write!(f, "fn {args}")?;
        if TyKind::Empty != rety.kind {
            write!(f, " -> {rety}")?;
        }
        Ok(())
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { absolute, parts } = self;
        if *absolute {
            "::".fmt(f)?;
        }
        separate(parts, "::")(f)
    }
}

impl Display for PathPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathPart::SuperKw => "super".fmt(f),
            PathPart::SelfTy => "Self".fmt(f),
            PathPart::Ident(id) => id.fmt(f),
        }
    }
}

impl Display for Stmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Stmt { span: _, kind, semi } = self;
        write!(f, "{kind}{semi}")
    }
}

impl Display for StmtKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StmtKind::Empty => Ok(()),
            StmtKind::Item(v) => v.fmt(f),
            StmtKind::Expr(v) => v.fmt(f),
        }
    }
}

impl Display for Semi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Semi::Terminated => ';'.fmt(f),
            Semi::Unterminated => Ok(()),
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

impl Display for ExprKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExprKind::Empty => "()".fmt(f),
            ExprKind::Closure(v) => v.fmt(f),
            ExprKind::Quote(v) => v.fmt(f),
            ExprKind::Let(v) => v.fmt(f),
            ExprKind::Match(v) => v.fmt(f),
            ExprKind::Assign(v) => v.fmt(f),
            ExprKind::Modify(v) => v.fmt(f),
            ExprKind::Binary(v) => v.fmt(f),
            ExprKind::Unary(v) => v.fmt(f),
            ExprKind::Cast(v) => v.fmt(f),
            ExprKind::Member(v) => v.fmt(f),
            ExprKind::Index(v) => v.fmt(f),
            ExprKind::Structor(v) => v.fmt(f),
            ExprKind::Path(v) => v.fmt(f),
            ExprKind::Literal(v) => v.fmt(f),
            ExprKind::Array(v) => v.fmt(f),
            ExprKind::ArrayRep(v) => v.fmt(f),
            ExprKind::AddrOf(v) => v.fmt(f),
            ExprKind::Block(v) => v.fmt(f),
            ExprKind::Group(v) => v.fmt(f),
            ExprKind::Tuple(v) => v.fmt(f),
            ExprKind::While(v) => v.fmt(f),
            ExprKind::If(v) => v.fmt(f),
            ExprKind::For(v) => v.fmt(f),
            ExprKind::Break(v) => v.fmt(f),
            ExprKind::Return(v) => v.fmt(f),
            ExprKind::Continue => "continue".fmt(f),
        }
    }
}

impl Display for Closure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { arg, body } = self;
        match arg.as_ref() {
            Pattern::Tuple(args) => separate(args, ", ")(f.delimit_with("|", "|")),
            _ => arg.fmt(f),
        }?;
        write!(f, " {body}")
    }
}

impl Display for Quote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { quote } = self;
        write!(f, "`{quote}`")
    }
}

impl Display for Let {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { mutable, name, ty, init } = self;
        write!(f, "let {mutable}{name}")?;
        if let Some(value) = ty {
            write!(f, ": {value}")?;
        }
        if let Some(value) = init {
            write!(f, " = {value}")?;
        }
        Ok(())
    }
}

impl Display for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Pattern::Name(sym) => sym.fmt(f),
            Pattern::Path(path) => path.fmt(f),
            Pattern::Literal(literal) => literal.fmt(f),
            Pattern::Rest(Some(name)) => write!(f, "..{name}"),
            Pattern::Rest(None) => "..".fmt(f),
            Pattern::Ref(mutability, pattern) => write!(f, "&{mutability}{pattern}"),
            Pattern::RangeExc(head, tail) => write!(f, "{head}..{tail}"),
            Pattern::RangeInc(head, tail) => write!(f, "{head}..={tail}"),
            Pattern::Tuple(patterns) => separate(patterns, ", ")(f.delimit(INLINE_PARENS)),
            Pattern::Array(patterns) => separate(patterns, ", ")(f.delimit(INLINE_SQUARE)),
            Pattern::Struct(path, items) => {
                write!(f, "{path} ")?;
                let f = &mut f.delimit(INLINE_BRACES);
                for (idx, (name, item)) in items.iter().enumerate() {
                    if idx != 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{name}")?;
                    if let Some(pattern) = item {
                        write!(f, ": {pattern}")?
                    }
                }
                Ok(())
            }
            Pattern::TupleStruct(path, items) => {
                write!(f, "{path}")?;
                separate(items, ", ")(f.delimit(INLINE_PARENS))
            }
        }
    }
}

impl Display for Match {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { scrutinee, arms } = self;
        write!(f, "match {scrutinee} ")?;
        separate(arms, ",\n")(f.delimit(BRACES))
    }
}

impl Display for MatchArm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(pat, expr) = self;
        write!(f, "{pat} => {expr}")
    }
}

impl Display for Assign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { parts } = self;
        write!(f, "{} = {}", parts.0, parts.1)
    }
}

impl Display for Modify {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { kind, parts } = self;
        write!(f, "{} {kind} {}", parts.0, parts.1)
    }
}

impl Display for ModifyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModifyKind::Mul => "*=",
            ModifyKind::Div => "/=",
            ModifyKind::Rem => "%=",
            ModifyKind::Add => "+=",
            ModifyKind::Sub => "-=",
            ModifyKind::And => "&=",
            ModifyKind::Or => "|=",
            ModifyKind::Xor => "^=",
            ModifyKind::Shl => "<<=",
            ModifyKind::Shr => ">>=",
        }
        .fmt(f)
    }
}

impl Display for Binary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { kind, parts } = self;
        let (head, tail) = parts.borrow();
        match kind {
            BinaryKind::Call => write!(f, "{head}{tail}"),
            _ => write!(f, "{head} {kind} {tail}"),
        }
    }
}

impl Display for BinaryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryKind::Lt => "<",
            BinaryKind::LtEq => "<=",
            BinaryKind::Equal => "==",
            BinaryKind::NotEq => "!=",
            BinaryKind::GtEq => ">=",
            BinaryKind::Gt => ">",
            BinaryKind::RangeExc => "..",
            BinaryKind::RangeInc => "..=",
            BinaryKind::LogAnd => "&&",
            BinaryKind::LogOr => "||",
            BinaryKind::LogXor => "^^",
            BinaryKind::BitAnd => "&",
            BinaryKind::BitOr => "|",
            BinaryKind::BitXor => "^",
            BinaryKind::Shl => "<<",
            BinaryKind::Shr => ">>",
            BinaryKind::Add => "+",
            BinaryKind::Sub => "-",
            BinaryKind::Mul => "*",
            BinaryKind::Div => "/",
            BinaryKind::Rem => "%",
            BinaryKind::Call => "()",
        }
        .fmt(f)
    }
}

impl Display for Unary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { kind, tail } = self;
        write!(f, "{kind}{tail}")
    }
}

impl Display for UnaryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnaryKind::Loop => "loop ",
            UnaryKind::Deref => "*",
            UnaryKind::Neg => "-",
            UnaryKind::Not => "!",
            UnaryKind::RangeExc => "..",
            UnaryKind::RangeInc => "..=",
            UnaryKind::At => "@",
            UnaryKind::Tilde => "~",
        }
        .fmt(f)
    }
}

impl Display for Cast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { head, ty } = self;
        write!(f, "{head} as {ty}")
    }
}

impl Display for Member {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { head, kind } = self;
        write!(f, "{head}.{kind}")
    }
}

impl Display for MemberKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemberKind::Call(name, args) => write!(f, "{name}{args}"),
            MemberKind::Struct(name) => write!(f, "{name}"),
            MemberKind::Tuple(name) => write!(f, "{name}"),
        }
    }
}

impl Display for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { head, indices } = self;
        write!(f, "{head}")?;
        separate(indices, ", ")(f.delimit(INLINE_SQUARE))
    }
}

impl Display for Structor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { to, init } = self;
        write!(f, "{to} ")?;
        separate(init, ", ")(f.delimit(INLINE_BRACES))
    }
}

impl Display for Fielder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { name, init } = self;
        write!(f, "{name}")?;
        if let Some(init) = init {
            write!(f, ": {init}")?;
        }
        Ok(())
    }
}

impl Display for Array {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        separate(&self.values, ", ")(f.delimit(INLINE_SQUARE))
    }
}

impl Display for ArrayRep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { value, repeat } = self;
        write!(f, "[{value}; {repeat}]")
    }
}

impl Display for AddrOf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { mutable, expr } = self;
        write!(f, "&{mutable}{expr}")
    }
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { stmts } = self;

        match stmts.as_slice() {
            [] => "{}".fmt(f),
            stmts => separate(stmts, "\n")(f.delimit(BRACES)),
        }
    }
}

impl Display for Group {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({})", self.expr)
    }
}

impl Display for Tuple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { exprs } = self;

        match exprs.as_slice() {
            [] => write!(f, "()"),
            [expr] => write!(f, "({expr},)"),
            exprs => separate(exprs, ", ")(f.delimit(INLINE_PARENS)),
        }
    }
}

impl Display for While {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { cond, pass, fail } = self;
        write!(f, "while {cond} {pass}{fail}")
    }
}

impl Display for If {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { cond, pass, fail } = self;
        write!(f, "if {cond} {pass}{fail}")
    }
}

impl Display for For {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { bind, cond, pass, fail } = self;
        write!(f, "for {bind} in {cond} {pass}{fail}")
    }
}

impl Display for Else {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.body {
            Some(body) => write!(f, " else {body}"),
            _ => Ok(()),
        }
    }
}

impl Display for Break {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "break")?;
        match &self.body {
            Some(body) => write!(f, " {body}"),
            _ => Ok(()),
        }
    }
}

impl Display for Return {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "return")?;
        match &self.body {
            Some(body) => write!(f, " {body}"),
            _ => Ok(()),
        }
    }
}
