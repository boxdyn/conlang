//! A [visitor](Visit) (implementer of the [Visit] trait) walks the immutable AST, mutating itself.

use crate::ast::*;
use cl_structures::span::Span;

/// Immutably walks the entire AST
///
/// Each method acts as a customization point.
///
/// There are a set of default implementations for enums
/// under the name [`or_visit_`*](or_visit_expr_kind),
/// provided for ease of use.
///
/// For all other nodes, traversal is *explicit*.
pub trait Visit<'a>: Sized {
    fn visit_span(&mut self, _span: &'a Span) {}
    fn visit_mutability(&mut self, _mutable: &'a Mutability) {}
    fn visit_visibility(&mut self, _vis: &'a Visibility) {}
    fn visit_sym(&mut self, _name: &'a Sym) {}
    fn visit_literal(&mut self, l: &'a Literal) {
        or_visit_literal(self, l)
    }
    fn visit_bool(&mut self, _b: &'a bool) {}
    fn visit_char(&mut self, _c: &'a char) {}
    fn visit_int(&mut self, _i: &'a u128) {}
    fn visit_smuggled_float(&mut self, _f: &'a u64) {}
    fn visit_string(&mut self, _s: &'a str) {}
    fn visit_file(&mut self, f: &'a File) {
        let File { items } = f;
        items.iter().for_each(|i| self.visit_item(i));
    }
    fn visit_attrs(&mut self, a: &'a Attrs) {
        let Attrs { meta } = a;
        meta.iter().for_each(|m| self.visit_meta(m));
    }
    fn visit_meta(&mut self, m: &'a Meta) {
        let Meta { name, kind } = m;
        self.visit_sym(name);
        self.visit_meta_kind(kind);
    }
    fn visit_meta_kind(&mut self, kind: &'a MetaKind) {
        or_visit_meta_kind(self, kind)
    }
    fn visit_item(&mut self, i: &'a Item) {
        let Item { span, attrs, vis, kind } = i;
        self.visit_span(span);
        self.visit_attrs(attrs);
        self.visit_visibility(vis);
        self.visit_item_kind(kind);
    }
    fn visit_item_kind(&mut self, kind: &'a ItemKind) {
        or_visit_item_kind(self, kind)
    }
    fn visit_alias(&mut self, a: &'a Alias) {
        let Alias { name, from } = a;
        self.visit_sym(name);
        if let Some(t) = from {
            self.visit_ty(t)
        }
    }
    fn visit_const(&mut self, c: &'a Const) {
        let Const { name, ty, init } = c;
        self.visit_sym(name);
        self.visit_ty(ty);
        self.visit_expr(init);
    }
    fn visit_static(&mut self, s: &'a Static) {
        let Static { mutable, name, ty, init } = s;
        self.visit_mutability(mutable);
        self.visit_sym(name);
        self.visit_ty(ty);
        self.visit_expr(init);
    }
    fn visit_module(&mut self, m: &'a Module) {
        let Module { name, file } = m;
        self.visit_sym(name);
        if let Some(f) = file {
            self.visit_file(f)
        }
    }
    fn visit_function(&mut self, f: &'a Function) {
        let Function { name, sign, bind, body } = f;
        self.visit_sym(name);
        self.visit_ty_fn(sign);
        bind.iter().for_each(|p| self.visit_pattern(p));
        if let Some(b) = body {
            self.visit_expr(b)
        }
    }
    fn visit_struct(&mut self, s: &'a Struct) {
        let Struct { name, kind } = s;
        self.visit_sym(name);
        self.visit_struct_kind(kind);
    }
    fn visit_struct_kind(&mut self, kind: &'a StructKind) {
        or_visit_struct_kind(self, kind)
    }
    fn visit_struct_member(&mut self, m: &'a StructMember) {
        let StructMember { vis, name, ty } = m;
        self.visit_visibility(vis);
        self.visit_sym(name);
        self.visit_ty(ty);
    }
    fn visit_enum(&mut self, e: &'a Enum) {
        let Enum { name, variants: kind } = e;
        self.visit_sym(name);
        if let Some(variants) = kind {
            variants.iter().for_each(|v| self.visit_variant(v))
        }
    }
    fn visit_variant(&mut self, v: &'a Variant) {
        let Variant { name, kind } = v;
        self.visit_sym(name);
        self.visit_variant_kind(kind);
    }
    fn visit_variant_kind(&mut self, kind: &'a VariantKind) {
        or_visit_variant_kind(self, kind)
    }
    fn visit_impl(&mut self, i: &'a Impl) {
        let Impl { target, body } = i;
        self.visit_impl_kind(target);
        self.visit_file(body);
    }
    fn visit_impl_kind(&mut self, target: &'a ImplKind) {
        or_visit_impl_kind(self, target)
    }
    fn visit_use(&mut self, u: &'a Use) {
        let Use { absolute: _, tree } = u;
        self.visit_use_tree(tree);
    }
    fn visit_use_tree(&mut self, tree: &'a UseTree) {
        or_visit_use_tree(self, tree)
    }
    fn visit_ty(&mut self, t: &'a Ty) {
        let Ty { span, kind } = t;
        self.visit_span(span);
        self.visit_ty_kind(kind);
    }
    fn visit_ty_kind(&mut self, kind: &'a TyKind) {
        or_visit_ty_kind(self, kind)
    }
    fn visit_ty_array(&mut self, a: &'a TyArray) {
        let TyArray { ty, count: _ } = a;
        self.visit_ty_kind(ty);
    }
    fn visit_ty_slice(&mut self, s: &'a TySlice) {
        let TySlice { ty } = s;
        self.visit_ty_kind(ty)
    }
    fn visit_ty_tuple(&mut self, t: &'a TyTuple) {
        let TyTuple { types } = t;
        types.iter().for_each(|kind| self.visit_ty_kind(kind))
    }
    fn visit_ty_ref(&mut self, t: &'a TyRef) {
        let TyRef { mutable, count: _, to } = t;
        self.visit_mutability(mutable);
        self.visit_path(to);
    }
    fn visit_ty_fn(&mut self, t: &'a TyFn) {
        let TyFn { args, rety } = t;
        self.visit_ty_kind(args);
        if let Some(rety) = rety {
            self.visit_ty(rety);
        }
    }
    fn visit_path(&mut self, p: &'a Path) {
        let Path { absolute: _, parts } = p;
        parts.iter().for_each(|p| self.visit_path_part(p))
    }
    fn visit_path_part(&mut self, p: &'a PathPart) {
        match p {
            PathPart::SuperKw => {}
            PathPart::SelfKw => {}
            PathPart::SelfTy => {}
            PathPart::Ident(i) => self.visit_sym(i),
        }
    }
    fn visit_stmt(&mut self, s: &'a Stmt) {
        let Stmt { span, kind, semi } = s;
        self.visit_span(span);
        self.visit_stmt_kind(kind);
        self.visit_semi(semi);
    }
    fn visit_stmt_kind(&mut self, kind: &'a StmtKind) {
        or_visit_stmt_kind(self, kind)
    }
    fn visit_semi(&mut self, _s: &'a Semi) {}
    fn visit_expr(&mut self, e: &'a Expr) {
        let Expr { span, kind } = e;
        self.visit_span(span);
        self.visit_expr_kind(kind)
    }
    fn visit_expr_kind(&mut self, e: &'a ExprKind) {
        or_visit_expr_kind(self, e)
    }
    fn visit_let(&mut self, l: &'a Let) {
        let Let { mutable, name, ty, init } = l;
        self.visit_mutability(mutable);
        self.visit_pattern(name);
        if let Some(ty) = ty {
            self.visit_ty(ty);
        }
        if let Some(init) = init {
            self.visit_expr(init)
        }
    }

    fn visit_pattern(&mut self, p: &'a Pattern) {
        match p {
            Pattern::Name(name) => self.visit_sym(name),
            Pattern::Literal(literal) => self.visit_literal(literal),
            Pattern::Rest(Some(name)) => self.visit_pattern(name),
            Pattern::Rest(None) => {}
            Pattern::Ref(mutability, pattern) => {
                self.visit_mutability(mutability);
                self.visit_pattern(pattern);
            }
            Pattern::Tuple(patterns) => {
                patterns.iter().for_each(|p| self.visit_pattern(p));
            }
            Pattern::Array(patterns) => {
                patterns.iter().for_each(|p| self.visit_pattern(p));
            }
            Pattern::Struct(path, items) => {
                self.visit_path(path);
                items.iter().for_each(|(_name, bind)| {
                    bind.as_ref().inspect(|bind| {
                        self.visit_pattern(bind);
                    });
                });
            }
            Pattern::TupleStruct(path, items) => {
                self.visit_path(path);
                items.iter().for_each(|bind| self.visit_pattern(bind));
            }
        }
    }

    fn visit_match(&mut self, m: &'a Match) {
        let Match { scrutinee, arms } = m;
        self.visit_expr(scrutinee);
        arms.iter().for_each(|arm| self.visit_match_arm(arm));
    }

    fn visit_match_arm(&mut self, a: &'a MatchArm) {
        let MatchArm(pat, expr) = a;
        self.visit_pattern(pat);
        self.visit_expr(expr);
    }

    fn visit_assign(&mut self, a: &'a Assign) {
        let Assign { parts } = a;
        let (head, tail) = parts.as_ref();
        self.visit_expr(head);
        self.visit_expr(tail);
    }
    fn visit_modify(&mut self, m: &'a Modify) {
        let Modify { kind, parts } = m;
        let (head, tail) = parts.as_ref();
        self.visit_modify_kind(kind);
        self.visit_expr(head);
        self.visit_expr(tail);
    }
    fn visit_modify_kind(&mut self, _kind: &'a ModifyKind) {}
    fn visit_binary(&mut self, b: &'a Binary) {
        let Binary { kind, parts } = b;
        let (head, tail) = parts.as_ref();
        self.visit_binary_kind(kind);
        self.visit_expr(head);
        self.visit_expr(tail);
    }
    fn visit_binary_kind(&mut self, _kind: &'a BinaryKind) {}
    fn visit_unary(&mut self, u: &'a Unary) {
        let Unary { kind, tail } = u;
        self.visit_unary_kind(kind);
        self.visit_expr(tail);
    }
    fn visit_unary_kind(&mut self, _kind: &'a UnaryKind) {}
    fn visit_cast(&mut self, cast: &'a Cast) {
        let Cast { head, ty } = cast;
        self.visit_expr(head);
        self.visit_ty(ty);
    }
    fn visit_member(&mut self, m: &'a Member) {
        let Member { head, kind } = m;
        self.visit_expr(head);
        self.visit_member_kind(kind);
    }
    fn visit_member_kind(&mut self, kind: &'a MemberKind) {
        or_visit_member_kind(self, kind)
    }
    fn visit_index(&mut self, i: &'a Index) {
        let Index { head, indices } = i;
        self.visit_expr(head);
        indices.iter().for_each(|e| self.visit_expr(e));
    }
    fn visit_structor(&mut self, s: &'a Structor) {
        let Structor { to, init } = s;
        self.visit_path(to);
        init.iter().for_each(|e| self.visit_fielder(e))
    }
    fn visit_fielder(&mut self, f: &'a Fielder) {
        let Fielder { name, init } = f;
        self.visit_sym(name);
        if let Some(init) = init {
            self.visit_expr(init);
        }
    }
    fn visit_array(&mut self, a: &'a Array) {
        let Array { values } = a;
        values.iter().for_each(|e| self.visit_expr(e))
    }
    fn visit_array_rep(&mut self, a: &'a ArrayRep) {
        let ArrayRep { value, repeat } = a;
        self.visit_expr(value);
        self.visit_expr(repeat);
    }
    fn visit_addrof(&mut self, a: &'a AddrOf) {
        let AddrOf { mutable, expr } = a;
        self.visit_mutability(mutable);
        self.visit_expr(expr);
    }
    fn visit_block(&mut self, b: &'a Block) {
        let Block { stmts } = b;
        stmts.iter().for_each(|s| self.visit_stmt(s));
    }
    fn visit_group(&mut self, g: &'a Group) {
        let Group { expr } = g;
        self.visit_expr(expr)
    }
    fn visit_tuple(&mut self, t: &'a Tuple) {
        let Tuple { exprs } = t;
        exprs.iter().for_each(|e| self.visit_expr(e))
    }
    fn visit_while(&mut self, w: &'a While) {
        let While { cond, pass, fail } = w;
        self.visit_expr(cond);
        self.visit_block(pass);
        self.visit_else(fail);
    }
    fn visit_if(&mut self, i: &'a If) {
        let If { cond, pass, fail } = i;
        self.visit_expr(cond);
        self.visit_block(pass);
        self.visit_else(fail);
    }
    fn visit_for(&mut self, f: &'a For) {
        let For { bind, cond, pass, fail } = f;
        self.visit_pattern(bind);
        self.visit_expr(cond);
        self.visit_block(pass);
        self.visit_else(fail);
    }
    fn visit_else(&mut self, e: &'a Else) {
        let Else { body } = e;
        if let Some(body) = body {
            self.visit_expr(body)
        }
    }
    fn visit_break(&mut self, b: &'a Break) {
        let Break { body } = b;
        if let Some(body) = body {
            self.visit_expr(body)
        }
    }
    fn visit_return(&mut self, r: &'a Return) {
        let Return { body } = r;
        if let Some(body) = body {
            self.visit_expr(body)
        }
    }
    fn visit_continue(&mut self) {}
}

pub fn or_visit_literal<'a, V: Visit<'a>>(visitor: &mut V, l: &'a Literal) {
    match l {
        Literal::Bool(b) => visitor.visit_bool(b),
        Literal::Char(c) => visitor.visit_char(c),
        Literal::Int(i) => visitor.visit_int(i),
        Literal::Float(f) => visitor.visit_smuggled_float(f),
        Literal::String(s) => visitor.visit_string(s),
    }
}

pub fn or_visit_meta_kind<'a, V: Visit<'a>>(visitor: &mut V, kind: &'a MetaKind) {
    match kind {
        MetaKind::Plain => {}
        MetaKind::Equals(l) => visitor.visit_literal(l),
        MetaKind::Func(lits) => lits.iter().for_each(|l| visitor.visit_literal(l)),
    }
}

pub fn or_visit_item_kind<'a, V: Visit<'a>>(visitor: &mut V, kind: &'a ItemKind) {
    match kind {
        ItemKind::Module(m) => visitor.visit_module(m),
        ItemKind::Alias(a) => visitor.visit_alias(a),
        ItemKind::Enum(e) => visitor.visit_enum(e),
        ItemKind::Struct(s) => visitor.visit_struct(s),
        ItemKind::Const(c) => visitor.visit_const(c),
        ItemKind::Static(s) => visitor.visit_static(s),
        ItemKind::Function(f) => visitor.visit_function(f),
        ItemKind::Impl(i) => visitor.visit_impl(i),
        ItemKind::Use(u) => visitor.visit_use(u),
    }
}

pub fn or_visit_struct_kind<'a, V: Visit<'a>>(visitor: &mut V, kind: &'a StructKind) {
    match kind {
        StructKind::Empty => {}
        StructKind::Tuple(ty) => ty.iter().for_each(|t| visitor.visit_ty(t)),
        StructKind::Struct(m) => m.iter().for_each(|m| visitor.visit_struct_member(m)),
    }
}

pub fn or_visit_variant_kind<'a, V: Visit<'a>>(visitor: &mut V, kind: &'a VariantKind) {
    match kind {
        VariantKind::Plain => {}
        VariantKind::CLike(_) => {}
        VariantKind::Tuple(t) => visitor.visit_ty(t),
        VariantKind::Struct(m) => m.iter().for_each(|m| visitor.visit_struct_member(m)),
    }
}

pub fn or_visit_impl_kind<'a, V: Visit<'a>>(visitor: &mut V, target: &'a ImplKind) {
    match target {
        ImplKind::Type(t) => visitor.visit_ty(t),
        ImplKind::Trait { impl_trait, for_type } => {
            visitor.visit_path(impl_trait);
            visitor.visit_ty(for_type)
        }
    }
}

pub fn or_visit_use_tree<'a, V: Visit<'a>>(visitor: &mut V, tree: &'a UseTree) {
    match tree {
        UseTree::Tree(tree) => {
            tree.iter().for_each(|tree| visitor.visit_use_tree(tree));
        }
        UseTree::Path(path, rest) => {
            visitor.visit_path_part(path);
            visitor.visit_use_tree(rest)
        }
        UseTree::Alias(path, name) => {
            visitor.visit_sym(path);
            visitor.visit_sym(name);
        }
        UseTree::Name(name) => {
            visitor.visit_sym(name);
        }
        UseTree::Glob => {}
    }
}

pub fn or_visit_ty_kind<'a, V: Visit<'a>>(visitor: &mut V, kind: &'a TyKind) {
    match kind {
        TyKind::Never => {}
        TyKind::Empty => {}
        TyKind::Infer => {}
        TyKind::Path(p) => visitor.visit_path(p),
        TyKind::Array(t) => visitor.visit_ty_array(t),
        TyKind::Slice(t) => visitor.visit_ty_slice(t),
        TyKind::Tuple(t) => visitor.visit_ty_tuple(t),
        TyKind::Ref(t) => visitor.visit_ty_ref(t),
        TyKind::Fn(t) => visitor.visit_ty_fn(t),
    }
}

pub fn or_visit_stmt_kind<'a, V: Visit<'a>>(visitor: &mut V, kind: &'a StmtKind) {
    match kind {
        StmtKind::Empty => {}
        StmtKind::Item(i) => visitor.visit_item(i),
        StmtKind::Expr(e) => visitor.visit_expr(e),
    }
}

pub fn or_visit_expr_kind<'a, V: Visit<'a>>(visitor: &mut V, e: &'a ExprKind) {
    match e {
        ExprKind::Empty => {}
        ExprKind::Quote(_q) => {} // Quoted expressions are left unvisited
        ExprKind::Let(l) => visitor.visit_let(l),
        ExprKind::Match(m) => visitor.visit_match(m),
        ExprKind::Assign(a) => visitor.visit_assign(a),
        ExprKind::Modify(m) => visitor.visit_modify(m),
        ExprKind::Binary(b) => visitor.visit_binary(b),
        ExprKind::Unary(u) => visitor.visit_unary(u),
        ExprKind::Cast(c) => visitor.visit_cast(c),
        ExprKind::Member(m) => visitor.visit_member(m),
        ExprKind::Index(i) => visitor.visit_index(i),
        ExprKind::Structor(s) => visitor.visit_structor(s),
        ExprKind::Path(p) => visitor.visit_path(p),
        ExprKind::Literal(l) => visitor.visit_literal(l),
        ExprKind::Array(a) => visitor.visit_array(a),
        ExprKind::ArrayRep(a) => visitor.visit_array_rep(a),
        ExprKind::AddrOf(a) => visitor.visit_addrof(a),
        ExprKind::Block(b) => visitor.visit_block(b),
        ExprKind::Group(g) => visitor.visit_group(g),
        ExprKind::Tuple(t) => visitor.visit_tuple(t),
        ExprKind::While(w) => visitor.visit_while(w),
        ExprKind::If(i) => visitor.visit_if(i),
        ExprKind::For(f) => visitor.visit_for(f),
        ExprKind::Break(b) => visitor.visit_break(b),
        ExprKind::Return(r) => visitor.visit_return(r),
        ExprKind::Continue => visitor.visit_continue(),
    }
}
pub fn or_visit_member_kind<'a, V: Visit<'a>>(visitor: &mut V, kind: &'a MemberKind) {
    match kind {
        MemberKind::Call(field, args) => {
            visitor.visit_sym(field);
            visitor.visit_tuple(args);
        }
        MemberKind::Struct(field) => visitor.visit_sym(field),
        MemberKind::Tuple(field) => visitor.visit_literal(field),
    }
}
