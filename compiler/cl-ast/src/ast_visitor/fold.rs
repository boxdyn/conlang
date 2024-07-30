//! A folder (implementer of the [Fold] trait) maps ASTs to ASTs

use crate::ast::*;
use cl_structures::span::Span;

/// Deconstructs the entire AST, and reconstructs it from scratch.
///
/// Each method acts as a customization point.
///
/// There are a set of default implementations for enums
/// under the name [`or_fold_`*](or_fold_expr_kind),
/// provided for ease of use.
///
/// For all other nodes, traversal is *explicit*.
pub trait Fold {
    fn fold_span(&mut self, extents: Span) -> Span {
        extents
    }
    fn fold_mutability(&mut self, mutability: Mutability) -> Mutability {
        mutability
    }
    fn fold_visibility(&mut self, visibility: Visibility) -> Visibility {
        visibility
    }
    fn fold_sym(&mut self, ident: Sym) -> Sym {
        ident
    }
    fn fold_literal(&mut self, lit: Literal) -> Literal {
        or_fold_literal(self, lit)
    }
    fn fold_bool(&mut self, b: bool) -> bool {
        b
    }
    fn fold_char(&mut self, c: char) -> char {
        c
    }
    fn fold_int(&mut self, i: u128) -> u128 {
        i
    }
    fn fold_string(&mut self, s: String) -> String {
        s
    }
    fn fold_file(&mut self, f: File) -> File {
        let File { items } = f;
        File { items: items.into_iter().map(|i| self.fold_item(i)).collect() }
    }
    fn fold_attrs(&mut self, a: Attrs) -> Attrs {
        let Attrs { meta } = a;
        Attrs { meta: meta.into_iter().map(|m| self.fold_meta(m)).collect() }
    }
    fn fold_meta(&mut self, m: Meta) -> Meta {
        let Meta { name, kind } = m;
        Meta { name: self.fold_sym(name), kind: self.fold_meta_kind(kind) }
    }
    fn fold_meta_kind(&mut self, kind: MetaKind) -> MetaKind {
        or_fold_meta_kind(self, kind)
    }
    fn fold_item(&mut self, i: Item) -> Item {
        let Item { extents, attrs, vis, kind } = i;
        Item {
            extents: self.fold_span(extents),
            attrs: self.fold_attrs(attrs),
            vis: self.fold_visibility(vis),
            kind: self.fold_item_kind(kind),
        }
    }
    fn fold_item_kind(&mut self, kind: ItemKind) -> ItemKind {
        or_fold_item_kind(self, kind)
    }
    fn fold_alias(&mut self, a: Alias) -> Alias {
        let Alias { to, from } = a;
        Alias { to: self.fold_sym(to), from: from.map(|from| Box::new(self.fold_ty(*from))) }
    }
    fn fold_const(&mut self, c: Const) -> Const {
        let Const { name, ty, init } = c;
        Const {
            name: self.fold_sym(name),
            ty: Box::new(self.fold_ty(*ty)),
            init: Box::new(self.fold_expr(*init)),
        }
    }
    fn fold_static(&mut self, s: Static) -> Static {
        let Static { mutable, name, ty, init } = s;
        Static {
            mutable: self.fold_mutability(mutable),
            name: self.fold_sym(name),
            ty: Box::new(self.fold_ty(*ty)),
            init: Box::new(self.fold_expr(*init)),
        }
    }
    fn fold_module(&mut self, m: Module) -> Module {
        let Module { name, kind } = m;
        Module { name: self.fold_sym(name), kind: self.fold_module_kind(kind) }
    }
    fn fold_module_kind(&mut self, m: ModuleKind) -> ModuleKind {
        match m {
            ModuleKind::Inline(f) => ModuleKind::Inline(self.fold_file(f)),
            ModuleKind::Outline => ModuleKind::Outline,
        }
    }
    fn fold_function(&mut self, f: Function) -> Function {
        let Function { name, sign, bind, body } = f;
        Function {
            name: self.fold_sym(name),
            sign: self.fold_ty_fn(sign),
            bind: bind.into_iter().map(|p| self.fold_param(p)).collect(),
            body: body.map(|b| self.fold_block(b)),
        }
    }
    fn fold_param(&mut self, p: Param) -> Param {
        let Param { mutability, name } = p;
        Param { mutability: self.fold_mutability(mutability), name: self.fold_sym(name) }
    }
    fn fold_struct(&mut self, s: Struct) -> Struct {
        let Struct { name, kind } = s;
        Struct { name: self.fold_sym(name), kind: self.fold_struct_kind(kind) }
    }
    fn fold_struct_kind(&mut self, kind: StructKind) -> StructKind {
        match kind {
            StructKind::Empty => StructKind::Empty,
            StructKind::Tuple(tys) => {
                StructKind::Tuple(tys.into_iter().map(|t| self.fold_ty(t)).collect())
            }
            StructKind::Struct(mem) => StructKind::Struct(
                mem.into_iter()
                    .map(|m| self.fold_struct_member(m))
                    .collect(),
            ),
        }
    }
    fn fold_struct_member(&mut self, m: StructMember) -> StructMember {
        let StructMember { vis, name, ty } = m;
        StructMember {
            vis: self.fold_visibility(vis),
            name: self.fold_sym(name),
            ty: self.fold_ty(ty),
        }
    }
    fn fold_enum(&mut self, e: Enum) -> Enum {
        let Enum { name, kind } = e;
        Enum { name: self.fold_sym(name), kind: self.fold_enum_kind(kind) }
    }
    fn fold_enum_kind(&mut self, kind: EnumKind) -> EnumKind {
        or_fold_enum_kind(self, kind)
    }
    fn fold_variant(&mut self, v: Variant) -> Variant {
        let Variant { name, kind } = v;

        Variant { name: self.fold_sym(name), kind: self.fold_variant_kind(kind) }
    }
    fn fold_variant_kind(&mut self, kind: VariantKind) -> VariantKind {
        or_fold_variant_kind(self, kind)
    }
    fn fold_impl(&mut self, i: Impl) -> Impl {
        let Impl { target, body } = i;
        Impl { target: self.fold_impl_kind(target), body: self.fold_file(body) }
    }
    fn fold_impl_kind(&mut self, kind: ImplKind) -> ImplKind {
        or_fold_impl_kind(self, kind)
    }
    fn fold_use(&mut self, u: Use) -> Use {
        let Use { absolute, tree } = u;
        Use { absolute, tree: self.fold_use_tree(tree) }
    }
    fn fold_use_tree(&mut self, tree: UseTree) -> UseTree {
        or_fold_use_tree(self, tree)
    }
    fn fold_ty(&mut self, t: Ty) -> Ty {
        let Ty { extents, kind } = t;
        Ty { extents: self.fold_span(extents), kind: self.fold_ty_kind(kind) }
    }
    fn fold_ty_kind(&mut self, kind: TyKind) -> TyKind {
        or_fold_ty_kind(self, kind)
    }
    fn fold_ty_array(&mut self, a: TyArray) -> TyArray {
        let TyArray { ty, count } = a;
        TyArray { ty: Box::new(self.fold_ty_kind(*ty)), count }
    }
    fn fold_ty_slice(&mut self, s: TySlice) -> TySlice {
        let TySlice { ty } = s;
        TySlice { ty: Box::new(self.fold_ty_kind(*ty)) }
    }
    fn fold_ty_tuple(&mut self, t: TyTuple) -> TyTuple {
        let TyTuple { types } = t;
        TyTuple {
            types: types
                .into_iter()
                .map(|kind| self.fold_ty_kind(kind))
                .collect(),
        }
    }
    fn fold_ty_ref(&mut self, t: TyRef) -> TyRef {
        let TyRef { mutable, count, to } = t;
        TyRef { mutable: self.fold_mutability(mutable), count, to: self.fold_path(to) }
    }
    fn fold_ty_fn(&mut self, t: TyFn) -> TyFn {
        let TyFn { args, rety } = t;
        TyFn {
            args: Box::new(self.fold_ty_kind(*args)),
            rety: rety.map(|t| Box::new(self.fold_ty(*t))),
        }
    }
    fn fold_path(&mut self, p: Path) -> Path {
        let Path { absolute, parts } = p;
        Path { absolute, parts: parts.into_iter().map(|p| self.fold_path_part(p)).collect() }
    }
    fn fold_path_part(&mut self, p: PathPart) -> PathPart {
        match p {
            PathPart::SuperKw => PathPart::SuperKw,
            PathPart::SelfKw => PathPart::SelfKw,
            PathPart::SelfTy => PathPart::SelfTy,
            PathPart::Ident(i) => PathPart::Ident(self.fold_sym(i)),
        }
    }
    fn fold_stmt(&mut self, s: Stmt) -> Stmt {
        let Stmt { extents, kind, semi } = s;
        Stmt {
            extents: self.fold_span(extents),
            kind: self.fold_stmt_kind(kind),
            semi: self.fold_semi(semi),
        }
    }
    fn fold_stmt_kind(&mut self, kind: StmtKind) -> StmtKind {
        or_fold_stmt_kind(self, kind)
    }
    fn fold_semi(&mut self, s: Semi) -> Semi {
        s
    }
    fn fold_let(&mut self, l: Let) -> Let {
        let Let { mutable, name, ty, init, tail } = l;
        Let {
            mutable: self.fold_mutability(mutable),
            name: self.fold_sym(name),
            ty: ty.map(|t| Box::new(self.fold_ty(*t))),
            init: init.map(|e| Box::new(self.fold_expr(*e))),
            tail: tail.map(|e| Box::new(self.fold_expr(*e))),
        }
    }
    fn fold_expr(&mut self, e: Expr) -> Expr {
        let Expr { extents, kind } = e;
        Expr { extents: self.fold_span(extents), kind: self.fold_expr_kind(kind) }
    }
    fn fold_expr_kind(&mut self, kind: ExprKind) -> ExprKind {
        or_fold_expr_kind(self, kind)
    }
    fn fold_assign(&mut self, a: Assign) -> Assign {
        let Assign { parts } = a;
        let (head, tail) = *parts;
        Assign { parts: Box::new((self.fold_expr_kind(head), self.fold_expr_kind(tail))) }
    }
    fn fold_modify(&mut self, m: Modify) -> Modify {
        let Modify { kind, parts } = m;
        let (head, tail) = *parts;
        Modify {
            kind: self.fold_modify_kind(kind),
            parts: Box::new((self.fold_expr_kind(head), self.fold_expr_kind(tail))),
        }
    }
    fn fold_modify_kind(&mut self, kind: ModifyKind) -> ModifyKind {
        kind
    }
    fn fold_binary(&mut self, b: Binary) -> Binary {
        let Binary { kind, parts } = b;
        let (head, tail) = *parts;
        Binary {
            kind: self.fold_binary_kind(kind),
            parts: Box::new((self.fold_expr_kind(head), self.fold_expr_kind(tail))),
        }
    }
    fn fold_binary_kind(&mut self, kind: BinaryKind) -> BinaryKind {
        kind
    }
    fn fold_unary(&mut self, u: Unary) -> Unary {
        let Unary { kind, tail } = u;
        Unary { kind: self.fold_unary_kind(kind), tail: Box::new(self.fold_expr_kind(*tail)) }
    }
    fn fold_unary_kind(&mut self, kind: UnaryKind) -> UnaryKind {
        kind
    }
    fn fold_cast(&mut self, cast: Cast) -> Cast {
        let Cast { head, ty } = cast;
        Cast { head: Box::new(self.fold_expr_kind(*head)), ty: self.fold_ty(ty) }
    }
    fn fold_member(&mut self, m: Member) -> Member {
        let Member { head, kind } = m;
        Member { head: Box::new(self.fold_expr_kind(*head)), kind: self.fold_member_kind(kind) }
    }
    fn fold_member_kind(&mut self, kind: MemberKind) -> MemberKind {
        or_fold_member_kind(self, kind)
    }
    fn fold_index(&mut self, i: Index) -> Index {
        let Index { head, indices } = i;
        Index {
            head: Box::new(self.fold_expr_kind(*head)),
            indices: indices.into_iter().map(|e| self.fold_expr(e)).collect(),
        }
    }

    fn fold_structor(&mut self, s: Structor) -> Structor {
        let Structor { to, init } = s;
        Structor {
            to: self.fold_path(to),
            init: init.into_iter().map(|f| self.fold_fielder(f)).collect(),
        }
    }

    fn fold_fielder(&mut self, f: Fielder) -> Fielder {
        let Fielder { name, init } = f;
        Fielder { name: self.fold_sym(name), init: init.map(|e| Box::new(self.fold_expr(*e))) }
    }
    fn fold_array(&mut self, a: Array) -> Array {
        let Array { values } = a;
        Array { values: values.into_iter().map(|e| self.fold_expr(e)).collect() }
    }
    fn fold_array_rep(&mut self, a: ArrayRep) -> ArrayRep {
        let ArrayRep { value, repeat } = a;
        ArrayRep {
            value: Box::new(self.fold_expr_kind(*value)),
            repeat: Box::new(self.fold_expr_kind(*repeat)),
        }
    }
    fn fold_addrof(&mut self, a: AddrOf) -> AddrOf {
        let AddrOf { count, mutable, expr } = a;
        AddrOf {
            count,
            mutable: self.fold_mutability(mutable),
            expr: Box::new(self.fold_expr_kind(*expr)),
        }
    }
    fn fold_block(&mut self, b: Block) -> Block {
        let Block { stmts } = b;
        Block { stmts: stmts.into_iter().map(|s| self.fold_stmt(s)).collect() }
    }
    fn fold_group(&mut self, g: Group) -> Group {
        let Group { expr } = g;
        Group { expr: Box::new(self.fold_expr_kind(*expr)) }
    }
    fn fold_tuple(&mut self, t: Tuple) -> Tuple {
        let Tuple { exprs } = t;
        Tuple { exprs: exprs.into_iter().map(|e| self.fold_expr(e)).collect() }
    }
    fn fold_while(&mut self, w: While) -> While {
        let While { cond, pass, fail } = w;
        While {
            cond: Box::new(self.fold_expr(*cond)),
            pass: Box::new(self.fold_block(*pass)),
            fail: self.fold_else(fail),
        }
    }
    fn fold_if(&mut self, i: If) -> If {
        let If { cond, pass, fail } = i;
        If {
            cond: Box::new(self.fold_expr(*cond)),
            pass: Box::new(self.fold_block(*pass)),
            fail: self.fold_else(fail),
        }
    }
    fn fold_for(&mut self, f: For) -> For {
        let For { bind, cond, pass, fail } = f;
        For {
            bind: self.fold_sym(bind),
            cond: Box::new(self.fold_expr(*cond)),
            pass: Box::new(self.fold_block(*pass)),
            fail: self.fold_else(fail),
        }
    }
    fn fold_else(&mut self, e: Else) -> Else {
        let Else { body } = e;
        Else { body: body.map(|e| Box::new(self.fold_expr(*e))) }
    }
    fn fold_break(&mut self, b: Break) -> Break {
        let Break { body } = b;
        Break { body: body.map(|e| Box::new(self.fold_expr(*e))) }
    }
    fn fold_return(&mut self, r: Return) -> Return {
        let Return { body } = r;
        Return { body: body.map(|e| Box::new(self.fold_expr(*e))) }
    }
    fn fold_continue(&mut self, c: Continue) -> Continue {
        let Continue = c;
        Continue
    }
}

#[inline]
/// Folds a [Literal] in the default way
pub fn or_fold_literal<F: Fold + ?Sized>(folder: &mut F, lit: Literal) -> Literal {
    match lit {
        Literal::Bool(b) => Literal::Bool(folder.fold_bool(b)),
        Literal::Char(c) => Literal::Char(folder.fold_char(c)),
        Literal::Int(i) => Literal::Int(folder.fold_int(i)),
        Literal::String(s) => Literal::String(folder.fold_string(s)),
    }
}

#[inline]
/// Folds a [MetaKind] in the default way
pub fn or_fold_meta_kind<F: Fold + ?Sized>(folder: &mut F, kind: MetaKind) -> MetaKind {
    match kind {
        MetaKind::Plain => MetaKind::Plain,
        MetaKind::Equals(l) => MetaKind::Equals(folder.fold_literal(l)),
        MetaKind::Func(lits) => {
            MetaKind::Func(lits.into_iter().map(|l| folder.fold_literal(l)).collect())
        }
    }
}

#[inline]
/// Folds an [ItemKind] in the default way
pub fn or_fold_item_kind<F: Fold + ?Sized>(folder: &mut F, kind: ItemKind) -> ItemKind {
    match kind {
        ItemKind::Module(m) => ItemKind::Module(folder.fold_module(m)),
        ItemKind::Alias(a) => ItemKind::Alias(folder.fold_alias(a)),
        ItemKind::Enum(e) => ItemKind::Enum(folder.fold_enum(e)),
        ItemKind::Struct(s) => ItemKind::Struct(folder.fold_struct(s)),
        ItemKind::Const(c) => ItemKind::Const(folder.fold_const(c)),
        ItemKind::Static(s) => ItemKind::Static(folder.fold_static(s)),
        ItemKind::Function(f) => ItemKind::Function(folder.fold_function(f)),
        ItemKind::Impl(i) => ItemKind::Impl(folder.fold_impl(i)),
        ItemKind::Use(u) => ItemKind::Use(folder.fold_use(u)),
    }
}

#[inline]
/// Folds a [ModuleKind] in the default way
pub fn or_fold_module_kind<F: Fold + ?Sized>(folder: &mut F, kind: ModuleKind) -> ModuleKind {
    match kind {
        ModuleKind::Inline(f) => ModuleKind::Inline(folder.fold_file(f)),
        ModuleKind::Outline => ModuleKind::Outline,
    }
}

#[inline]
/// Folds a [StructKind] in the default way
pub fn or_fold_struct_kind<F: Fold + ?Sized>(folder: &mut F, kind: StructKind) -> StructKind {
    match kind {
        StructKind::Empty => StructKind::Empty,
        StructKind::Tuple(tys) => {
            StructKind::Tuple(tys.into_iter().map(|t| folder.fold_ty(t)).collect())
        }
        StructKind::Struct(mem) => StructKind::Struct(
            mem.into_iter()
                .map(|m| folder.fold_struct_member(m))
                .collect(),
        ),
    }
}

#[inline]
/// Folds an [EnumKind] in the default way
pub fn or_fold_enum_kind<F: Fold + ?Sized>(folder: &mut F, kind: EnumKind) -> EnumKind {
    match kind {
        EnumKind::NoVariants => EnumKind::NoVariants,
        EnumKind::Variants(v) => {
            EnumKind::Variants(v.into_iter().map(|v| folder.fold_variant(v)).collect())
        }
    }
}

#[inline]
/// Folds a [VariantKind] in the default way
pub fn or_fold_variant_kind<F: Fold + ?Sized>(folder: &mut F, kind: VariantKind) -> VariantKind {
    match kind {
        VariantKind::Plain => VariantKind::Plain,
        VariantKind::CLike(n) => VariantKind::CLike(n),
        VariantKind::Tuple(t) => VariantKind::Tuple(folder.fold_ty(t)),
        VariantKind::Struct(mem) => VariantKind::Struct(
            mem.into_iter()
                .map(|m| folder.fold_struct_member(m))
                .collect(),
        ),
    }
}

#[inline]
/// Folds an [ImplKind] in the default way
pub fn or_fold_impl_kind<F: Fold + ?Sized>(folder: &mut F, kind: ImplKind) -> ImplKind {
    match kind {
        ImplKind::Type(t) => ImplKind::Type(folder.fold_ty(t)),
        ImplKind::Trait { impl_trait, for_type } => ImplKind::Trait {
            impl_trait: folder.fold_path(impl_trait),
            for_type: Box::new(folder.fold_ty(*for_type)),
        },
    }
}

#[inline]
pub fn or_fold_use_tree<F: Fold + ?Sized>(folder: &mut F, tree: UseTree) -> UseTree {
    match tree {
        UseTree::Tree(tree) => UseTree::Tree(
            tree.into_iter()
                .map(|tree| folder.fold_use_tree(tree))
                .collect(),
        ),
        UseTree::Path(path, rest) => UseTree::Path(
            folder.fold_path_part(path),
            Box::new(folder.fold_use_tree(*rest)),
        ),
        UseTree::Alias(path, name) => UseTree::Alias(folder.fold_sym(path), folder.fold_sym(name)),
        UseTree::Name(name) => UseTree::Name(folder.fold_sym(name)),
        UseTree::Glob => UseTree::Glob,
    }
}

#[inline]
/// Folds a [TyKind] in the default way
pub fn or_fold_ty_kind<F: Fold + ?Sized>(folder: &mut F, kind: TyKind) -> TyKind {
    match kind {
        TyKind::Never => TyKind::Never,
        TyKind::Empty => TyKind::Empty,
        TyKind::Path(p) => TyKind::Path(folder.fold_path(p)),
        TyKind::Array(a) => TyKind::Array(folder.fold_ty_array(a)),
        TyKind::Slice(s) => TyKind::Slice(folder.fold_ty_slice(s)),
        TyKind::Tuple(t) => TyKind::Tuple(folder.fold_ty_tuple(t)),
        TyKind::Ref(t) => TyKind::Ref(folder.fold_ty_ref(t)),
        TyKind::Fn(t) => TyKind::Fn(folder.fold_ty_fn(t)),
    }
}

#[inline]
/// Folds a [StmtKind] in the default way
pub fn or_fold_stmt_kind<F: Fold + ?Sized>(folder: &mut F, kind: StmtKind) -> StmtKind {
    match kind {
        StmtKind::Empty => StmtKind::Empty,
        StmtKind::Item(i) => StmtKind::Item(Box::new(folder.fold_item(*i))),
        StmtKind::Expr(e) => StmtKind::Expr(Box::new(folder.fold_expr(*e))),
    }
}
#[inline]
/// Folds an [ExprKind] in the default way
pub fn or_fold_expr_kind<F: Fold + ?Sized>(folder: &mut F, kind: ExprKind) -> ExprKind {
    match kind {
        ExprKind::Empty => ExprKind::Empty,
        ExprKind::Let(l) => ExprKind::Let(folder.fold_let(l)),
        ExprKind::Assign(a) => ExprKind::Assign(folder.fold_assign(a)),
        ExprKind::Modify(m) => ExprKind::Modify(folder.fold_modify(m)),
        ExprKind::Binary(b) => ExprKind::Binary(folder.fold_binary(b)),
        ExprKind::Unary(u) => ExprKind::Unary(folder.fold_unary(u)),
        ExprKind::Cast(c) => ExprKind::Cast(folder.fold_cast(c)),
        ExprKind::Member(m) => ExprKind::Member(folder.fold_member(m)),
        ExprKind::Index(i) => ExprKind::Index(folder.fold_index(i)),
        ExprKind::Structor(s) => ExprKind::Structor(folder.fold_structor(s)),
        ExprKind::Path(p) => ExprKind::Path(folder.fold_path(p)),
        ExprKind::Literal(l) => ExprKind::Literal(folder.fold_literal(l)),
        ExprKind::Array(a) => ExprKind::Array(folder.fold_array(a)),
        ExprKind::ArrayRep(a) => ExprKind::ArrayRep(folder.fold_array_rep(a)),
        ExprKind::AddrOf(a) => ExprKind::AddrOf(folder.fold_addrof(a)),
        ExprKind::Block(b) => ExprKind::Block(folder.fold_block(b)),
        ExprKind::Group(g) => ExprKind::Group(folder.fold_group(g)),
        ExprKind::Tuple(t) => ExprKind::Tuple(folder.fold_tuple(t)),
        ExprKind::While(w) => ExprKind::While(folder.fold_while(w)),
        ExprKind::If(i) => ExprKind::If(folder.fold_if(i)),
        ExprKind::For(f) => ExprKind::For(folder.fold_for(f)),
        ExprKind::Break(b) => ExprKind::Break(folder.fold_break(b)),
        ExprKind::Return(r) => ExprKind::Return(folder.fold_return(r)),
        ExprKind::Continue(c) => ExprKind::Continue(folder.fold_continue(c)),
    }
}
pub fn or_fold_member_kind<F: Fold + ?Sized>(folder: &mut F, kind: MemberKind) -> MemberKind {
    match kind {
        MemberKind::Call(name, args) => {
            MemberKind::Call(folder.fold_sym(name), folder.fold_tuple(args))
        }
        MemberKind::Struct(name) => MemberKind::Struct(folder.fold_sym(name)),
        MemberKind::Tuple(name) => MemberKind::Tuple(folder.fold_literal(name)),
    }
}
