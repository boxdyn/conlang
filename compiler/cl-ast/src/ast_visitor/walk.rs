//! Accepts an AST Visitor. Walks the AST, calling the visitor on each step.

use super::visit::Visit;
use crate::ast::*;
use cl_structures::span::Span;

/// Helps a [Visitor](Visit) walk through `Self`.
pub trait Walk {
    /// Calls the respective `visit_*` function in V
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V);

    #[allow(unused)]
    /// Walks the children of self, visiting them in V
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {}
}

impl Walk for Span {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_span(self);
    }
}
impl Walk for Sym {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_sym(self);
    }
}
impl Walk for Mutability {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_mutability(self);
    }
}
impl Walk for Visibility {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_visibility(self);
    }
}
impl Walk for bool {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_bool(self);
    }
}
impl Walk for char {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_char(self);
    }
}
impl Walk for u128 {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_int(self);
    }
}
impl Walk for u64 {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_smuggled_float(self);
    }
}
impl Walk for str {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_string(self);
    }
}
impl Walk for Literal {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_literal(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        match self {
            Literal::Bool(value) => value.children(v),
            Literal::Char(value) => value.children(v),
            Literal::Int(value) => value.children(v),
            Literal::Float(value) => value.children(v),
            Literal::String(value) => value.children(v),
        };
    }
}
impl Walk for File {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_file(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let File { name: _, items } = self;
        items.iter().for_each(|i| v.visit_item(i));
    }
}
impl Walk for Attrs {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_attrs(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Attrs { meta } = self;
        meta.children(v);
    }
}
impl Walk for Meta {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_meta(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Meta { name, kind } = self;
        name.visit_in(v);
        kind.visit_in(v);
    }
}
impl Walk for MetaKind {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_meta_kind(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        match self {
            MetaKind::Plain => {}
            MetaKind::Equals(lit) => lit.visit_in(v),
            MetaKind::Func(lits) => lits.visit_in(v),
        }
    }
}
impl Walk for Item {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_item(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Item { span, attrs, vis, kind } = self;
        span.visit_in(v);
        attrs.visit_in(v);
        vis.visit_in(v);
        kind.visit_in(v);
    }
}
impl Walk for ItemKind {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_item_kind(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        match self {
            ItemKind::Module(value) => value.visit_in(v),
            ItemKind::Alias(value) => value.visit_in(v),
            ItemKind::Enum(value) => value.visit_in(v),
            ItemKind::Struct(value) => value.visit_in(v),
            ItemKind::Const(value) => value.visit_in(v),
            ItemKind::Static(value) => value.visit_in(v),
            ItemKind::Function(value) => value.visit_in(v),
            ItemKind::Impl(value) => value.visit_in(v),
            ItemKind::Use(value) => value.visit_in(v),
        }
    }
}
impl Walk for Generics {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_generics(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Self { vars } = self;
        vars.visit_in(v);
    }
}
impl Walk for Module {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_module(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Module { name, file } = self;
        name.visit_in(v);
        file.visit_in(v);
    }
}
impl Walk for Alias {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_alias(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Alias { name, from } = self;
        name.visit_in(v);
        from.visit_in(v);
    }
}
impl Walk for Const {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_const(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Const { name, ty, init } = self;
        name.visit_in(v);
        ty.visit_in(v);
        init.visit_in(v);
    }
}
impl Walk for Static {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_static(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Static { mutable, name, ty, init } = self;
        mutable.visit_in(v);
        name.visit_in(v);
        ty.visit_in(v);
        init.visit_in(v);
    }
}
impl Walk for Function {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_function(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Function { name, gens, sign, bind, body } = self;
        name.visit_in(v);
        gens.visit_in(v);
        sign.visit_in(v);
        bind.visit_in(v);
        body.visit_in(v);
    }
}
impl Walk for Struct {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_struct(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Struct { name, gens, kind } = self;
        name.visit_in(v);
        gens.visit_in(v);
        kind.visit_in(v);
    }
}
impl Walk for StructKind {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_struct_kind(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        match self {
            StructKind::Empty => {}
            StructKind::Tuple(tys) => tys.visit_in(v),
            StructKind::Struct(ms) => ms.visit_in(v),
        }
    }
}
impl Walk for StructMember {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_struct_member(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let StructMember { vis, name, ty } = self;
        vis.visit_in(v);
        name.visit_in(v);
        ty.visit_in(v);
    }
}
impl Walk for Enum {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_enum(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Enum { name, gens, variants } = self;
        name.visit_in(v);
        gens.visit_in(v);
        variants.visit_in(v);
    }
}
impl Walk for Variant {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_variant(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Variant { name, kind, body } = self;
        name.visit_in(v);
        kind.visit_in(v);
        body.visit_in(v);
    }
}
impl Walk for Impl {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_impl(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Impl { target, body } = self;
        target.visit_in(v);
        body.visit_in(v);
    }
}
impl Walk for ImplKind {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_impl_kind(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        match self {
            ImplKind::Type(t) => t.visit_in(v),
            ImplKind::Trait { impl_trait, for_type } => {
                impl_trait.visit_in(v);
                for_type.visit_in(v);
            }
        }
    }
}
impl Walk for Use {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_use(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Use { absolute: _, tree } = self;
        tree.visit_in(v);
    }
}
impl Walk for UseTree {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_use_tree(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        match self {
            UseTree::Tree(tree) => tree.iter().for_each(|t| t.visit_in(v)),
            UseTree::Path(part, tree) => {
                part.visit_in(v);
                tree.visit_in(v);
            }
            UseTree::Alias(from, to) => {
                from.visit_in(v);
                to.visit_in(v);
            }
            UseTree::Name(name) => name.visit_in(v),
            UseTree::Glob => {}
        }
    }
}
impl Walk for Ty {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_ty(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Ty { span, kind } = self;
        span.visit_in(v);
        kind.visit_in(v);
    }
}
impl Walk for TyKind {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_ty_kind(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        match self {
            TyKind::Never => {}
            TyKind::Empty => {}
            TyKind::Infer => {}
            TyKind::Path(value) => value.visit_in(v),
            TyKind::Array(value) => value.visit_in(v),
            TyKind::Slice(value) => value.visit_in(v),
            TyKind::Tuple(value) => value.visit_in(v),
            TyKind::Ref(value) => value.visit_in(v),
            TyKind::Fn(value) => value.visit_in(v),
        }
    }
}
impl Walk for TyArray {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_ty_array(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let TyArray { ty, count: _ } = self;
        ty.visit_in(v);
        // count.walk(v); // not available
    }
}
impl Walk for TySlice {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_ty_slice(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let TySlice { ty } = self;
        ty.visit_in(v);
    }
}
impl Walk for TyTuple {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_ty_tuple(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let TyTuple { types } = self;
        types.visit_in(v);
    }
}
impl Walk for TyRef {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_ty_ref(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let TyRef { mutable, count: _, to } = self;
        mutable.children(v);
        to.children(v);
    }
}
impl Walk for TyFn {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_ty_fn(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let TyFn { args, rety } = self;
        args.visit_in(v);
        rety.visit_in(v);
    }
}
impl Walk for Path {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_path(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Path { absolute: _, parts } = self;
        parts.visit_in(v);
    }
}
impl Walk for PathPart {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_path_part(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        match self {
            PathPart::SuperKw => {}
            PathPart::SelfTy => {}
            PathPart::Ident(sym) => sym.visit_in(v),
        }
    }
}
impl Walk for Stmt {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_stmt(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Stmt { span, kind, semi } = self;
        span.visit_in(v);
        kind.visit_in(v);
        semi.visit_in(v);
    }
}
impl Walk for StmtKind {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_stmt_kind(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        match self {
            StmtKind::Empty => {}
            StmtKind::Item(value) => value.visit_in(v),
            StmtKind::Expr(value) => value.visit_in(v),
        }
    }
}
impl Walk for Semi {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_semi(self);
    }
}
impl Walk for Expr {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_expr(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Expr { span, kind } = self;
        span.visit_in(v);
        kind.visit_in(v);
    }
}
impl Walk for ExprKind {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_expr_kind(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        match self {
            ExprKind::Empty => {}
            ExprKind::Quote(value) => value.visit_in(v),
            ExprKind::Let(value) => value.visit_in(v),
            ExprKind::Match(value) => value.visit_in(v),
            ExprKind::Assign(value) => value.visit_in(v),
            ExprKind::Modify(value) => value.visit_in(v),
            ExprKind::Binary(value) => value.visit_in(v),
            ExprKind::Unary(value) => value.visit_in(v),
            ExprKind::Cast(value) => value.visit_in(v),
            ExprKind::Member(value) => value.visit_in(v),
            ExprKind::Index(value) => value.visit_in(v),
            ExprKind::Structor(value) => value.visit_in(v),
            ExprKind::Path(value) => value.visit_in(v),
            ExprKind::Literal(value) => value.visit_in(v),
            ExprKind::Array(value) => value.visit_in(v),
            ExprKind::ArrayRep(value) => value.visit_in(v),
            ExprKind::AddrOf(value) => value.visit_in(v),
            ExprKind::Block(value) => value.visit_in(v),
            ExprKind::Group(value) => value.visit_in(v),
            ExprKind::Tuple(value) => value.visit_in(v),
            ExprKind::While(value) => value.visit_in(v),
            ExprKind::If(value) => value.visit_in(v),
            ExprKind::For(value) => value.visit_in(v),
            ExprKind::Break(value) => value.visit_in(v),
            ExprKind::Return(value) => value.visit_in(v),
            ExprKind::Continue => v.visit_continue(),
        }
    }
}

impl Walk for Tuple {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_tuple(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Tuple { exprs } = self;
        exprs.visit_in(v);
    }
}
impl Walk for Structor {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_structor(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Structor { to, init } = self;
        to.visit_in(v);
        init.visit_in(v);
    }
}
impl Walk for Fielder {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_fielder(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Fielder { name, init } = self;
        name.visit_in(v);
        init.visit_in(v);
    }
}
impl Walk for Array {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_array(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Array { values } = self;
        values.visit_in(v);
    }
}
impl Walk for ArrayRep {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_array_rep(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let ArrayRep { value, repeat: _ } = self;
        value.visit_in(v);
        // repeat.visit_in(v) // TODO
    }
}
impl Walk for AddrOf {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_addrof(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let AddrOf { mutable, expr } = self;
        mutable.visit_in(v);
        expr.visit_in(v);
    }
}
impl Walk for Cast {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_cast(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Cast { head, ty } = self;
        head.visit_in(v);
        ty.visit_in(v);
    }
}
impl Walk for Quote {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_quote(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Quote { quote } = self;
        quote.visit_in(v);
    }
}
impl Walk for Group {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_group(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Group { expr } = self;
        expr.visit_in(v);
    }
}
impl Walk for Block {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_block(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Block { stmts } = self;
        stmts.visit_in(v);
    }
}
impl Walk for Assign {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_assign(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Assign { parts } = self;
        parts.visit_in(v);
    }
}
impl Walk for Modify {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_modify(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Modify { kind, parts } = self;
        kind.visit_in(v);
        parts.visit_in(v);
    }
}
impl Walk for ModifyKind {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_modify_kind(self);
    }
}
impl Walk for Binary {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_binary(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Binary { kind, parts } = self;
        kind.visit_in(v);
        parts.visit_in(v);
    }
}
impl Walk for BinaryKind {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_binary_kind(self);
    }
}
impl Walk for Unary {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_unary(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Unary { kind, tail } = self;
        kind.visit_in(v);
        tail.visit_in(v);
    }
}
impl Walk for UnaryKind {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_unary_kind(self);
    }
}
impl Walk for Member {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_member(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Member { head, kind } = self;
        head.visit_in(v);
        kind.visit_in(v);
    }
}
impl Walk for MemberKind {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_member_kind(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        match self {
            MemberKind::Call(sym, tuple) => {
                sym.visit_in(v);
                tuple.visit_in(v);
            }
            MemberKind::Struct(sym) => sym.visit_in(v),
            MemberKind::Tuple(literal) => literal.visit_in(v),
        }
    }
}
impl Walk for Index {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_index(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Index { head, indices } = self;
        head.visit_in(v);
        indices.visit_in(v);
    }
}
impl Walk for Let {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_let(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Let { mutable, name, ty, init } = self;
        mutable.visit_in(v);
        name.visit_in(v);
        ty.visit_in(v);
        init.visit_in(v);
    }
}
impl Walk for Match {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_match(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Match { scrutinee, arms } = self;
        scrutinee.visit_in(v);
        arms.visit_in(v);
    }
}
impl Walk for MatchArm {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_match_arm(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let MatchArm(pat, expr) = self;
        pat.visit_in(v);
        expr.visit_in(v);
    }
}
impl Walk for Pattern {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_pattern(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        match self {
            Pattern::Name(sym) => sym.visit_in(v),
            Pattern::Literal(literal) => literal.visit_in(v),
            Pattern::Rest(pattern) => pattern.visit_in(v),
            Pattern::Ref(mutability, pattern) => {
                mutability.visit_in(v);
                pattern.visit_in(v);
            }
            Pattern::RangeExc(from, to) => {
                from.visit_in(v);
                to.visit_in(v);
            }
            Pattern::RangeInc(from, to) => {
                from.visit_in(v);
                to.visit_in(v);
            }
            Pattern::Tuple(patterns) => patterns.visit_in(v),
            Pattern::Array(patterns) => patterns.visit_in(v),
            Pattern::Struct(path, items) => {
                path.visit_in(v);
                items.visit_in(v);
            }
            Pattern::TupleStruct(path, patterns) => {
                path.visit_in(v);
                patterns.visit_in(v);
            }
        }
    }
}
impl Walk for While {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_while(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let While { cond, pass, fail } = self;
        cond.visit_in(v);
        pass.visit_in(v);
        fail.visit_in(v);
    }
}
impl Walk for If {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_if(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let If { cond, pass, fail } = self;
        cond.visit_in(v);
        pass.visit_in(v);
        fail.visit_in(v);
    }
}
impl Walk for For {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_for(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let For { bind, cond, pass, fail } = self;
        bind.visit_in(v);
        cond.visit_in(v);
        pass.visit_in(v);
        fail.visit_in(v);
    }
}
impl Walk for Else {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_else(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Else { body } = self;
        body.visit_in(v);
    }
}
impl Walk for Break {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_break(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Break { body } = self;
        body.visit_in(v);
    }
}
impl Walk for Return {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_return(self);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let Return { body } = self;
        body.visit_in(v);
    }
}

// --- BLANKET IMPLEMENTATIONS

impl<T: Walk> Walk for [T] {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        self.iter().for_each(|value| value.visit_in(v));
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        self.iter().for_each(|value| value.children(v));
    }
}

impl<T: Walk> Walk for Vec<T> {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        self.as_slice().visit_in(v);
    }

    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        self.as_slice().children(v);
    }
}

impl<A: Walk, B: Walk> Walk for (A, B) {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let (a, b) = self;
        a.visit_in(v);
        b.visit_in(v);
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        let (a, b) = self;
        a.children(v);
        b.children(v);
    }
}

impl<T: Walk> Walk for Option<T> {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        if let Some(value) = self.as_ref() {
            value.visit_in(v)
        }
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        if let Some(value) = self {
            value.children(v)
        }
    }
}

impl<T: Walk> Walk for Box<T> {
    #[inline]
    fn visit_in<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        self.as_ref().visit_in(v)
    }
    fn children<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        self.as_ref().children(v)
    }
}
