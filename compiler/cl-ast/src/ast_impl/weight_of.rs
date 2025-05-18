//! Approximates the size of an AST

use std::mem::size_of_val;

use crate::ast::*;
use cl_structures::{intern::interned::Interned, span::Span};

/// Approximates the size of an AST without including indirection (pointers) or padding
pub trait WeightOf {
    /// Approximates the size of a syntax tree without including pointer/indirection or padding.
    fn weight_of(&self) -> usize;
}

impl WeightOf for File {
    fn weight_of(&self) -> usize {
        let Self { name, items } = self;
        name.weight_of() + items.weight_of()
    }
}

impl WeightOf for Attrs {
    fn weight_of(&self) -> usize {
        let Self { meta } = self;
        meta.weight_of()
    }
}

impl WeightOf for Meta {
    fn weight_of(&self) -> usize {
        let Self { name, kind } = self;
        name.weight_of() + kind.weight_of()
    }
}

impl WeightOf for MetaKind {
    fn weight_of(&self) -> usize {
        match self {
            MetaKind::Plain => size_of_val(self),
            MetaKind::Equals(v) => v.weight_of(),
            MetaKind::Func(v) => v.weight_of(),
        }
    }
}

impl WeightOf for Item {
    fn weight_of(&self) -> usize {
        let Self { span, attrs, vis, kind } = self;
        span.weight_of() + attrs.weight_of() + vis.weight_of() + kind.weight_of()
    }
}

impl WeightOf for ItemKind {
    fn weight_of(&self) -> usize {
        match self {
            ItemKind::Module(v) => v.weight_of(),
            ItemKind::Alias(v) => v.weight_of(),
            ItemKind::Enum(v) => v.weight_of(),
            ItemKind::Struct(v) => v.weight_of(),
            ItemKind::Const(v) => v.weight_of(),
            ItemKind::Static(v) => v.weight_of(),
            ItemKind::Function(v) => v.weight_of(),
            ItemKind::Impl(v) => v.weight_of(),
            ItemKind::Use(v) => v.weight_of(),
        }
    }
}

impl WeightOf for Generics {
    fn weight_of(&self) -> usize {
        let Self { vars } = self;
        vars.iter().map(|v| v.weight_of()).sum()
    }
}

impl WeightOf for Module {
    fn weight_of(&self) -> usize {
        let Self { name, file } = self;
        name.weight_of() + file.weight_of()
    }
}

impl WeightOf for Alias {
    fn weight_of(&self) -> usize {
        let Self { name, from } = self;
        name.weight_of() + from.weight_of()
    }
}

impl WeightOf for Const {
    fn weight_of(&self) -> usize {
        let Self { name, ty, init } = self;
        name.weight_of() + ty.weight_of() + init.weight_of()
    }
}

impl WeightOf for Static {
    fn weight_of(&self) -> usize {
        let Self { mutable, name, ty, init } = self;
        mutable.weight_of() + name.weight_of() + ty.weight_of() + init.weight_of()
    }
}

impl WeightOf for Function {
    fn weight_of(&self) -> usize {
        let Self { name, gens, sign, bind, body } = self;
        name.weight_of() + gens.weight_of() + sign.weight_of() + bind.weight_of() + body.weight_of()
    }
}

impl WeightOf for Struct {
    fn weight_of(&self) -> usize {
        let Self { name, gens, kind } = self;
        name.weight_of() + gens.weight_of() + kind.weight_of()
    }
}

impl WeightOf for StructKind {
    fn weight_of(&self) -> usize {
        match self {
            StructKind::Empty => size_of_val(self),
            StructKind::Tuple(items) => items.weight_of(),
            StructKind::Struct(sm) => sm.weight_of(),
        }
    }
}

impl WeightOf for StructMember {
    fn weight_of(&self) -> usize {
        let Self { vis, name, ty } = self;
        vis.weight_of() + name.weight_of() + ty.weight_of()
    }
}

impl WeightOf for Enum {
    fn weight_of(&self) -> usize {
        let Self { name, gens, variants } = self;
        name.weight_of() + gens.weight_of() + variants.weight_of()
    }
}

impl WeightOf for Variant {
    fn weight_of(&self) -> usize {
        let Self { name, kind, body } = self;
        name.weight_of() + kind.weight_of() + body.weight_of()
    }
}

impl WeightOf for Impl {
    fn weight_of(&self) -> usize {
        let Self { target, body } = self;
        target.weight_of() + body.weight_of()
    }
}

impl WeightOf for ImplKind {
    fn weight_of(&self) -> usize {
        match self {
            ImplKind::Type(ty) => ty.weight_of(),
            ImplKind::Trait { impl_trait, for_type } => {
                impl_trait.weight_of() + for_type.weight_of()
            }
        }
    }
}

impl WeightOf for Use {
    fn weight_of(&self) -> usize {
        let Self { absolute, tree } = self;
        absolute.weight_of() + tree.weight_of()
    }
}

impl WeightOf for UseTree {
    fn weight_of(&self) -> usize {
        match self {
            UseTree::Tree(tr) => tr.weight_of(),
            UseTree::Path(pa, tr) => pa.weight_of() + tr.weight_of(),
            UseTree::Alias(src, dst) => src.weight_of() + dst.weight_of(),
            UseTree::Name(src) => src.weight_of(),
            UseTree::Glob => size_of_val(self),
        }
    }
}

impl WeightOf for Ty {
    fn weight_of(&self) -> usize {
        let Self { span, kind } = self;
        span.weight_of() + kind.weight_of()
    }
}

impl WeightOf for TyKind {
    fn weight_of(&self) -> usize {
        match self {
            TyKind::Never | TyKind::Empty | TyKind::Infer => size_of_val(self),
            TyKind::Path(v) => v.weight_of(),
            TyKind::Array(v) => v.weight_of(),
            TyKind::Slice(v) => v.weight_of(),
            TyKind::Tuple(v) => v.weight_of(),
            TyKind::Ref(v) => v.weight_of(),
            TyKind::Fn(v) => v.weight_of(),
        }
    }
}

impl WeightOf for TyArray {
    fn weight_of(&self) -> usize {
        let Self { ty, count } = self;
        ty.weight_of() + count.weight_of()
    }
}

impl WeightOf for TySlice {
    fn weight_of(&self) -> usize {
        let Self { ty } = self;
        ty.weight_of()
    }
}

impl WeightOf for TyTuple {
    fn weight_of(&self) -> usize {
        let Self { types } = self;
        types.weight_of()
    }
}

impl WeightOf for TyRef {
    fn weight_of(&self) -> usize {
        let Self { mutable, count, to } = self;
        mutable.weight_of() + count.weight_of() + to.weight_of()
    }
}

impl WeightOf for TyFn {
    fn weight_of(&self) -> usize {
        let Self { args, rety } = self;
        args.weight_of() + rety.weight_of()
    }
}

impl WeightOf for Path {
    fn weight_of(&self) -> usize {
        let Self { absolute, parts } = self;
        absolute.weight_of() + parts.weight_of()
    }
}

impl WeightOf for PathPart {
    fn weight_of(&self) -> usize {
        match self {
            PathPart::SuperKw => size_of_val(self),
            PathPart::SelfTy => size_of_val(self),
            PathPart::Ident(interned) => interned.weight_of(),
        }
    }
}

impl WeightOf for Stmt {
    fn weight_of(&self) -> usize {
        let Self { span, kind, semi } = self;
        span.weight_of() + kind.weight_of() + semi.weight_of()
    }
}

impl WeightOf for StmtKind {
    fn weight_of(&self) -> usize {
        match self {
            StmtKind::Empty => size_of_val(self),
            StmtKind::Item(item) => item.weight_of(),
            StmtKind::Expr(expr) => expr.weight_of(),
        }
    }
}

impl WeightOf for Expr {
    fn weight_of(&self) -> usize {
        let Self { span, kind } = self;
        span.weight_of() + kind.weight_of()
    }
}

impl WeightOf for ExprKind {
    fn weight_of(&self) -> usize {
        match self {
            ExprKind::Empty => size_of_val(self),
            ExprKind::Closure(v) => v.weight_of(),
            ExprKind::Quote(v) => v.weight_of(),
            ExprKind::Let(v) => v.weight_of(),
            ExprKind::Match(v) => v.weight_of(),
            ExprKind::Assign(v) => v.weight_of(),
            ExprKind::Modify(v) => v.weight_of(),
            ExprKind::Binary(v) => v.weight_of(),
            ExprKind::Unary(v) => v.weight_of(),
            ExprKind::Cast(v) => v.weight_of(),
            ExprKind::Member(v) => v.weight_of(),
            ExprKind::Index(v) => v.weight_of(),
            ExprKind::Structor(v) => v.weight_of(),
            ExprKind::Path(v) => v.weight_of(),
            ExprKind::Literal(v) => v.weight_of(),
            ExprKind::Array(v) => v.weight_of(),
            ExprKind::ArrayRep(v) => v.weight_of(),
            ExprKind::AddrOf(v) => v.weight_of(),
            ExprKind::Block(v) => v.weight_of(),
            ExprKind::Group(v) => v.weight_of(),
            ExprKind::Tuple(v) => v.weight_of(),
            ExprKind::While(v) => v.weight_of(),
            ExprKind::If(v) => v.weight_of(),
            ExprKind::For(v) => v.weight_of(),
            ExprKind::Break(v) => v.weight_of(),
            ExprKind::Return(v) => v.weight_of(),
            ExprKind::Continue => size_of_val(self),
        }
    }
}

impl WeightOf for Closure {
    fn weight_of(&self) -> usize {
        let Self { arg, body } = self;
        arg.weight_of() + body.weight_of()
    }
}

impl WeightOf for Quote {
    fn weight_of(&self) -> usize {
        let Self { quote } = self;
        quote.weight_of()
    }
}

impl WeightOf for Let {
    fn weight_of(&self) -> usize {
        let Self { mutable, name, ty, init } = self;
        mutable.weight_of() + name.weight_of() + ty.weight_of() + init.weight_of()
    }
}

impl WeightOf for Pattern {
    fn weight_of(&self) -> usize {
        match self {
            Pattern::Name(s) => size_of_val(s),
            Pattern::Path(p) => p.weight_of(),
            Pattern::Literal(literal) => literal.weight_of(),
            Pattern::Rest(Some(pattern)) => pattern.weight_of(),
            Pattern::Rest(None) => 0,
            Pattern::Ref(mutability, pattern) => mutability.weight_of() + pattern.weight_of(),
            Pattern::RangeExc(head, tail) => head.weight_of() + tail.weight_of(),
            Pattern::RangeInc(head, tail) => head.weight_of() + tail.weight_of(),
            Pattern::Tuple(patterns) | Pattern::Array(patterns) => patterns.weight_of(),
            Pattern::Struct(path, items) => {
                let sitems: usize = items
                    .iter()
                    .map(|(name, opt)| name.weight_of() + opt.weight_of())
                    .sum();
                path.weight_of() + sitems
            }
            Pattern::TupleStruct(path, patterns) => path.weight_of() + patterns.weight_of(),
        }
    }
}

impl WeightOf for Match {
    fn weight_of(&self) -> usize {
        let Self { scrutinee, arms } = self;
        scrutinee.weight_of() + arms.weight_of()
    }
}

impl WeightOf for MatchArm {
    fn weight_of(&self) -> usize {
        let Self(pattern, expr) = self;
        pattern.weight_of() + expr.weight_of()
    }
}

impl WeightOf for Assign {
    fn weight_of(&self) -> usize {
        let Self { parts } = self;

        parts.0.weight_of() + parts.1.weight_of()
    }
}

impl WeightOf for Modify {
    #[rustfmt::skip]
    fn weight_of(&self) -> usize {
        let Self { kind, parts } = self;
        kind.weight_of()
            + parts.0.weight_of()
            + parts.1.weight_of()
    }
}

impl WeightOf for Binary {
    fn weight_of(&self) -> usize {
        let Self { kind, parts } = self;

        kind.weight_of() + parts.0.weight_of() + parts.1.weight_of()
    }
}

impl WeightOf for Unary {
    #[rustfmt::skip]
    fn weight_of(&self) -> usize {
        let Self { kind, tail } = self;
         kind.weight_of() + tail.weight_of()
    }
}

impl WeightOf for Cast {
    fn weight_of(&self) -> usize {
        let Self { head, ty } = self;
        head.weight_of() + ty.weight_of()
    }
}

impl WeightOf for Member {
    fn weight_of(&self) -> usize {
        let Self { head, kind } = self;

        head.weight_of() + kind.weight_of() // accounting
    }
}

impl WeightOf for MemberKind {
    fn weight_of(&self) -> usize {
        match self {
            MemberKind::Call(_, tuple) => tuple.weight_of(),
            MemberKind::Struct(_) => 0,
            MemberKind::Tuple(literal) => literal.weight_of(),
        }
    }
}

impl WeightOf for Index {
    fn weight_of(&self) -> usize {
        let Self { head, indices } = self;
        head.weight_of() + indices.weight_of()
    }
}

impl WeightOf for Literal {
    fn weight_of(&self) -> usize {
        match self {
            Literal::Bool(v) => v.weight_of(),
            Literal::Char(v) => v.weight_of(),
            Literal::Int(v) => v.weight_of(),
            Literal::Float(v) => v.weight_of(),
            Literal::String(v) => v.weight_of(),
        }
    }
}

impl WeightOf for Structor {
    fn weight_of(&self) -> usize {
        let Self { to, init } = self;
        to.weight_of() + init.weight_of()
    }
}

impl WeightOf for Fielder {
    fn weight_of(&self) -> usize {
        let Self { name, init } = self;
        name.weight_of() + init.weight_of()
    }
}

impl WeightOf for Array {
    fn weight_of(&self) -> usize {
        let Self { values } = self;
        values.weight_of()
    }
}

impl WeightOf for ArrayRep {
    fn weight_of(&self) -> usize {
        let Self { value, repeat } = self;
        value.weight_of() + repeat.weight_of()
    }
}

impl WeightOf for AddrOf {
    fn weight_of(&self) -> usize {
        let Self { mutable, expr } = self;
        mutable.weight_of() + expr.weight_of()
    }
}

impl WeightOf for Block {
    fn weight_of(&self) -> usize {
        let Self { stmts } = self;
        stmts.weight_of()
    }
}

impl WeightOf for Group {
    fn weight_of(&self) -> usize {
        let Self { expr } = self;
        expr.weight_of()
    }
}

impl WeightOf for Tuple {
    fn weight_of(&self) -> usize {
        let Self { exprs } = self;
        exprs.weight_of()
    }
}

impl WeightOf for While {
    fn weight_of(&self) -> usize {
        let Self { cond, pass, fail } = self;
        cond.weight_of() + pass.weight_of() + fail.weight_of()
    }
}

impl WeightOf for If {
    fn weight_of(&self) -> usize {
        let Self { cond, pass, fail } = self;
        cond.weight_of() + pass.weight_of() + fail.weight_of()
    }
}

impl WeightOf for For {
    fn weight_of(&self) -> usize {
        let Self { bind, cond, pass, fail } = self;
        bind.weight_of() + cond.weight_of() + pass.weight_of() + fail.weight_of()
    }
}

impl WeightOf for Else {
    fn weight_of(&self) -> usize {
        let Self { body } = self;
        body.weight_of()
    }
}

impl WeightOf for Break {
    fn weight_of(&self) -> usize {
        let Self { body } = self;
        body.weight_of()
    }
}

impl WeightOf for Return {
    fn weight_of(&self) -> usize {
        let Self { body } = self;
        body.weight_of()
    }
}

// ------------ SizeOf Blanket Implementations

impl<T: WeightOf> WeightOf for Option<T> {
    fn weight_of(&self) -> usize {
        match self {
            Some(t) => t.weight_of().max(size_of_val(t)),
            None => size_of_val(self),
        }
    }
}

impl<T: WeightOf> WeightOf for [T] {
    fn weight_of(&self) -> usize {
        self.iter().map(WeightOf::weight_of).sum()
    }
}

impl<T: WeightOf> WeightOf for Vec<T> {
    fn weight_of(&self) -> usize {
        size_of::<Self>() + self.iter().map(WeightOf::weight_of).sum::<usize>()
    }
}

impl<T: WeightOf> WeightOf for Box<T> {
    fn weight_of(&self) -> usize {
        (**self).weight_of() + size_of::<Self>()
    }
}

impl WeightOf for str {
    fn weight_of(&self) -> usize {
        self.len()
    }
}

impl_size_of! {
    // primitives
    u8, u16, u32, u64, u128, usize,
    i8, i16, i32, i64, i128, isize,
    f32, f64, bool, char,
    // cl-structures
    Span,
    // cl-ast
    Visibility, Mutability, Semi, ModifyKind, BinaryKind, UnaryKind
}

impl<T> WeightOf for Interned<'_, T> {
    fn weight_of(&self) -> usize {
        size_of_val(self) // interned values are opaque to SizeOF
    }
}

macro impl_size_of($($T:ty),*$(,)?) {
    $(impl WeightOf for $T {
        fn weight_of(&self) -> usize {
            ::std::mem::size_of_val(self)
        }
    })*
}
