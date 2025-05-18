//! A [visitor](Visit) (implementer of the [Visit] trait) walks the immutable AST, mutating itself.

use crate::ast::*;
use cl_structures::span::Span;

use super::walk::Walk;

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
    /// Visits a [Walker](Walk)
    #[inline]
    fn visit<W: Walk>(&mut self, walker: &'a W) {
        walker.visit_in(self)
    }
    /// Visits the children of a [Walker](Walk)
    fn visit_children<W: Walk>(&mut self, walker: &'a W) {
        walker.children(self)
    }

    fn visit_span(&mut self, value: &'a Span) {
        value.children(self)
    }
    fn visit_mutability(&mut self, value: &'a Mutability) {
        value.children(self)
    }
    fn visit_visibility(&mut self, value: &'a Visibility) {
        value.children(self)
    }
    fn visit_sym(&mut self, value: &'a Sym) {
        value.children(self)
    }
    fn visit_literal(&mut self, value: &'a Literal) {
        value.children(self)
    }
    fn visit_bool(&mut self, value: &'a bool) {
        value.children(self)
    }
    fn visit_char(&mut self, value: &'a char) {
        value.children(self)
    }
    fn visit_int(&mut self, value: &'a u128) {
        value.children(self)
    }
    fn visit_smuggled_float(&mut self, value: &'a u64) {
        value.children(self)
    }
    fn visit_string(&mut self, value: &'a str) {
        value.children(self)
    }
    fn visit_file(&mut self, value: &'a File) {
        value.children(self)
    }
    fn visit_attrs(&mut self, value: &'a Attrs) {
        value.children(self)
    }
    fn visit_meta(&mut self, value: &'a Meta) {
        value.children(self)
    }
    fn visit_meta_kind(&mut self, value: &'a MetaKind) {
        value.children(self)
    }
    fn visit_item(&mut self, value: &'a Item) {
        value.children(self)
    }
    fn visit_item_kind(&mut self, value: &'a ItemKind) {
        value.children(self)
    }
    fn visit_generics(&mut self, value: &'a Generics) {
        value.children(self)
    }
    fn visit_alias(&mut self, value: &'a Alias) {
        value.children(self)
    }
    fn visit_const(&mut self, value: &'a Const) {
        value.children(self)
    }
    fn visit_static(&mut self, value: &'a Static) {
        value.children(self)
    }
    fn visit_module(&mut self, value: &'a Module) {
        value.children(self)
    }
    fn visit_function(&mut self, value: &'a Function) {
        value.children(self)
    }
    fn visit_struct(&mut self, value: &'a Struct) {
        value.children(self)
    }
    fn visit_struct_kind(&mut self, value: &'a StructKind) {
        value.children(self)
    }
    fn visit_struct_member(&mut self, value: &'a StructMember) {
        value.children(self)
    }
    fn visit_enum(&mut self, value: &'a Enum) {
        value.children(self)
    }
    fn visit_variant(&mut self, value: &'a Variant) {
        value.children(self)
    }
    fn visit_impl(&mut self, value: &'a Impl) {
        value.children(self)
    }
    fn visit_impl_kind(&mut self, value: &'a ImplKind) {
        value.children(self)
    }
    fn visit_use(&mut self, value: &'a Use) {
        value.children(self)
    }
    fn visit_use_tree(&mut self, value: &'a UseTree) {
        value.children(self)
    }
    fn visit_ty(&mut self, value: &'a Ty) {
        value.children(self)
    }
    fn visit_ty_kind(&mut self, value: &'a TyKind) {
        value.children(self)
    }
    fn visit_ty_array(&mut self, value: &'a TyArray) {
        value.children(self)
    }
    fn visit_ty_slice(&mut self, value: &'a TySlice) {
        value.children(self)
    }
    fn visit_ty_tuple(&mut self, value: &'a TyTuple) {
        value.children(self)
    }
    fn visit_ty_ref(&mut self, value: &'a TyRef) {
        value.children(self)
    }
    fn visit_ty_fn(&mut self, value: &'a TyFn) {
        value.children(self)
    }
    fn visit_path(&mut self, value: &'a Path) {
        value.children(self)
    }
    fn visit_path_part(&mut self, value: &'a PathPart) {
        value.children(self)
    }
    fn visit_stmt(&mut self, value: &'a Stmt) {
        value.children(self)
    }
    fn visit_stmt_kind(&mut self, value: &'a StmtKind) {
        value.children(self)
    }
    fn visit_semi(&mut self, value: &'a Semi) {
        value.children(self)
    }
    fn visit_expr(&mut self, value: &'a Expr) {
        value.children(self)
    }
    fn visit_expr_kind(&mut self, value: &'a ExprKind) {
        value.children(self)
    }
    fn visit_quote(&mut self, value: &'a Quote) {
        value.children(self)
    }
    fn visit_let(&mut self, value: &'a Let) {
        value.children(self)
    }

    fn visit_pattern(&mut self, value: &'a Pattern) {
        value.children(self)
    }

    fn visit_match(&mut self, value: &'a Match) {
        value.children(self)
    }

    fn visit_match_arm(&mut self, value: &'a MatchArm) {
        value.children(self)
    }

    fn visit_assign(&mut self, value: &'a Assign) {
        value.children(self)
    }
    fn visit_modify(&mut self, value: &'a Modify) {
        value.children(self)
    }
    fn visit_modify_kind(&mut self, value: &'a ModifyKind) {
        value.children(self)
    }
    fn visit_binary(&mut self, value: &'a Binary) {
        value.children(self)
    }
    fn visit_binary_kind(&mut self, value: &'a BinaryKind) {
        value.children(self)
    }
    fn visit_unary(&mut self, value: &'a Unary) {
        value.children(self)
    }
    fn visit_unary_kind(&mut self, value: &'a UnaryKind) {
        value.children(self)
    }

    fn visit_cast(&mut self, value: &'a Cast) {
        value.children(self)
    }
    fn visit_member(&mut self, value: &'a Member) {
        value.children(self)
    }
    fn visit_member_kind(&mut self, value: &'a MemberKind) {
        value.children(self)
    }
    fn visit_index(&mut self, value: &'a Index) {
        value.children(self)
    }
    fn visit_structor(&mut self, value: &'a Structor) {
        value.children(self)
    }
    fn visit_fielder(&mut self, value: &'a Fielder) {
        value.children(self)
    }
    fn visit_array(&mut self, value: &'a Array) {
        value.children(self)
    }
    fn visit_array_rep(&mut self, value: &'a ArrayRep) {
        value.children(self)
    }
    fn visit_addrof(&mut self, value: &'a AddrOf) {
        value.children(self)
    }
    fn visit_block(&mut self, value: &'a Block) {
        value.children(self)
    }
    fn visit_group(&mut self, value: &'a Group) {
        value.children(self)
    }
    fn visit_tuple(&mut self, value: &'a Tuple) {
        value.children(self)
    }
    fn visit_while(&mut self, value: &'a While) {
        value.children(self)
    }
    fn visit_if(&mut self, value: &'a If) {
        value.children(self)
    }
    fn visit_for(&mut self, value: &'a For) {
        value.children(self)
    }
    fn visit_else(&mut self, value: &'a Else) {
        value.children(self)
    }
    fn visit_break(&mut self, value: &'a Break) {
        value.children(self)
    }
    fn visit_return(&mut self, value: &'a Return) {
        value.children(self)
    }
    fn visit_continue(&mut self) {}
}
