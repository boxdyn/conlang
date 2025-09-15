//! The [Inference] trait is the heart of cl-typeck's type inference.
//!
//! Each syntax structure must describe how to unify its types.

use std::iter;

use super::{engine::InferenceEngine, error::InferenceError};
use crate::{
    handle::Handle,
    type_expression::TypeExpression,
    type_kind::{Adt, TypeKind},
};
use cl_ast::*;

// TODO: "Infer" the types of Items

type IfResult = Result<Handle, InferenceError>;

pub trait Inference<'a> {
    /// Performs type inference
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult;
}

impl<'a> Inference<'a> for File {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Self { name: _, items } = self;
        for item in items {
            item.infer(e)?;
        }
        Ok(e.empty())
    }
}

impl<'a> Inference<'a> for Item {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Self { span: _, attrs: _, vis: _, kind } = self;
        kind.infer(e)
    }
}

impl<'a> Inference<'a> for ItemKind {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        match self {
            ItemKind::Module(v) => v.infer(e),
            ItemKind::Alias(v) => v.infer(e),
            ItemKind::Enum(v) => v.infer(e),
            ItemKind::Struct(v) => v.infer(e),
            ItemKind::Const(v) => v.infer(e),
            ItemKind::Static(v) => v.infer(e),
            ItemKind::Function(v) => v.infer(e),
            ItemKind::Impl(v) => v.infer(e),
            ItemKind::Use(_v) => Ok(e.empty()),
        }
    }
}

impl<'a> Inference<'a> for Generics {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        // bind names
        for name in &self.vars {
            let ty = e.new_var();
            e.table.add_child(e.at, *name, ty);
        }
        Ok(e.empty())
    }
}

impl<'a> Inference<'a> for Module {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Self { name, file } = self;
        let Some(file) = file else {
            return Err(InferenceError::NotFound((*name).into()));
        };
        let module = e.by_name(name)?;
        e.at(module).infer(file)
    }
}

impl<'a> Inference<'a> for Alias {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Self { name: _, from } = self;
        // let this = e.by_name(name)?;
        let alias = if let Some(from) = from {
            TypeKind::Instance(e.infer(from)?)
        } else {
            TypeKind::Tuple(vec![])
        };

        // This node may be a lang item referring to a primitive.
        let mut entry = e.at.to_entry_mut(e.table);
        if entry.ty().is_some() {
            return Ok(e.empty());
        }
        entry.set_ty(alias);

        Ok(entry.id())
    }
}

impl<'a> Inference<'a> for Const {
    #[allow(unused)]
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Self { name: _, ty, init } = self;
        // Same as static
        let node = e.at; //.by_name(name)?;
        let ty = e.infer(ty)?;
        let mut scope = e.at(node);
        // infer body
        let body = scope.infer(init)?;
        // unify with ty
        e.unify(body, ty)?;

        Ok(node)
    }
}

impl<'a> Inference<'a> for Static {
    #[allow(unused)]
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Static { mutable, name, ty, init } = self;
        let node = e.at; //e.by_name(name)?;
        let ty = e.infer(ty)?;
        let mut scope = e.at(node);
        // infer body
        let body = scope.infer(init)?;
        // unify with ty
        e.unify(body, ty)?;

        Ok(node)
    }
}

impl<'a> Inference<'a> for Function {
    #[allow(unused)]
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Self { name, gens, sign, bind, body } = self;
        // bind name to signature
        let node = e.at; // e.by_name(name)?;
        let node = e.deep_clone(node);
        let fnty = e.by_name(sign)?;
        e.unify(node, fnty)?;

        // bind gens to new variables at function scope
        let mut scope = e.at(node);
        scope.infer(gens)?;

        // bind binds to args
        let pat = scope.infer(bind)?;
        let arg = scope.by_name(sign.args.as_ref())?;
        scope.unify(pat, arg);

        let mut retscope = scope.open_rset();

        // infer body
        let bodty = retscope.infer(body)?;
        let rety = sign.rety.infer(&mut retscope)?;
        // unify body with rety
        retscope.unify(bodty, rety)?;
        // unify rset with rety
        if let Some(rset) = retscope.rset.get() {
            scope.unify(rset, rety)?;
        }
        Ok(node)
    }
}

// TODO: do we need type inference/checking in struct definitions?
// there are no bodies

impl<'a> Inference<'a> for Enum {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Self { name: _, gens, variants } = self;
        let node = e.at; //e.by_name(name)?;
        let mut scope = e.at(node);

        scope.infer(gens)?;
        for variant in variants {
            println!("Inferring {variant}");
            let var_ty = scope.infer(variant)?;
            scope.unify(node, var_ty)?;
        }
        Ok(node)
    }
}

impl<'a> Inference<'a> for Variant {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Self { name, kind: _, body } = self;
        let node = e.by_name(name)?;

        // TODO: this doesn't work when some variants have bodies and some don't
        if e.table.ty(node).is_some() {
            println!("{node} has ty!");
            return Ok(node);
        }

        match body {
            Some(body) => {
                let mut e = e.at(node);
                let value = e.infer(body)?;
                e.unify(node, value)?;
            }
            _ => {
                e.table.entry_mut(node).set_ty(TypeKind::Inferred);
            }
        };

        Ok(node)
    }
}

impl<'a> Inference<'a> for Struct {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Self { name, gens, kind: _ } = self;
        let node = e.by_name(name)?;
        let mut e = e.at(node);
        e.infer(gens)?;

        Ok(node)
    }
}

impl<'a> Inference<'a> for Impl {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Self { gens, target, body } = self;
        // TODO: match gens to target gens
        gens.infer(e)?;
        let instance = target.infer(e)?;
        let instance = e.def_usage(instance);
        let mut scope = e.at(instance);
        scope.infer(body)
    }
}

impl<'a> Inference<'a> for ImplKind {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        match self {
            ImplKind::Type(ty) => ty.infer(e),
            ImplKind::Trait { impl_trait: _, for_type } => for_type.infer(e),
        }
    }
}

impl<'a> Inference<'a> for Ty {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        Ok(e.by_name(self)?)
    }
}

impl<'a> Inference<'a> for cl_ast::Stmt {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Self { span: _, kind, semi } = self;
        let out = kind.infer(e)?;
        Ok(match semi {
            Semi::Terminated => e.empty(),
            Semi::Unterminated => out,
        })
    }
}

impl<'a> Inference<'a> for cl_ast::StmtKind {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        match self {
            StmtKind::Empty => Ok(e.empty()),
            StmtKind::Item(item) => item.infer(e),
            StmtKind::Expr(expr) => expr.infer(e),
        }
    }
}

impl<'a> Inference<'a> for cl_ast::Expr {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let out = self.kind.infer(e)?;
        println!("expr ({self}) -> {}", e.entry(out));
        Ok(out)
    }
}

impl<'a> Inference<'a> for cl_ast::ExprKind {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        match self {
            ExprKind::Empty => Ok(e.empty()),
            ExprKind::Closure(closure) => closure.infer(e),
            ExprKind::Tuple(tuple) => tuple.infer(e),
            ExprKind::Structor(structor) => structor.infer(e),
            ExprKind::Array(array) => array.infer(e),
            ExprKind::ArrayRep(array_rep) => array_rep.infer(e),
            ExprKind::AddrOf(addr_of) => addr_of.infer(e),
            ExprKind::Quote(quote) => quote.infer(e),
            ExprKind::Literal(literal) => literal.infer(e),

            ExprKind::Group(group) => group.infer(e),
            ExprKind::Block(block) => block.infer(e),

            ExprKind::Assign(assign) => assign.infer(e),
            ExprKind::Modify(modify) => modify.infer(e),
            ExprKind::Binary(binary) => binary.infer(e),
            ExprKind::Unary(unary) => unary.infer(e),
            ExprKind::Member(member) => member.infer(e),
            ExprKind::Index(index) => index.infer(e),
            ExprKind::Path(path) => path.infer(e),
            ExprKind::Cast(cast) => cast.infer(e),

            ExprKind::Let(l) => l.infer(e),
            ExprKind::Match(m) => m.infer(e),
            ExprKind::While(w) => w.infer(e),
            ExprKind::If(i) => i.infer(e),
            ExprKind::For(f) => f.infer(e),
            ExprKind::Break(b) => b.infer(e),
            ExprKind::Return(r) => r.infer(e),
            ExprKind::Continue => Ok(e.never()),
        }
    }
}

impl<'a> Inference<'a> for Closure {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Self { arg, body } = self;
        let args = arg.infer(e)?;

        let mut scope = e.block_scope();
        let mut scope = scope.open_rset();
        let rety = scope.infer(body)?;

        if let Some(rset) = scope.rset.get() {
            e.unify(rety, rset)?;
        }

        Ok(e.table.anon_type(TypeKind::FnSig { args, rety }))
    }
}

impl<'a> Inference<'a> for Tuple {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Tuple { exprs } = self;
        exprs
            .iter()
            // Infer each member
            .map(|expr| expr.infer(e))
            // Construct tuple
            .collect::<Result<Vec<_>, InferenceError>>()
            // Return tuple
            .map(|tys| e.new_tuple(tys))
    }
}

impl<'a> Inference<'a> for Structor {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Structor { to, init } = self;
        // Evaluate the path in the current context
        let to = to.infer(e)?;
        match e.entry(to).ty() {
            // Typecheck the fielders against the fields
            Some(TypeKind::Adt(Adt::Struct(fields))) => {
                if init.len() != fields.len() {
                    return Err(InferenceError::FieldCount(to, fields.len(), init.len()));
                }
                let fields = fields.clone(); // todo: fix this somehow.
                let mut field_inits: std::collections::HashMap<_, _> = init
                    .iter()
                    .map(|Fielder { name, init }| (name, init))
                    .collect();
                // Unify fields with fielders
                for (name, _vis, ty) in fields {
                    match field_inits.remove(&name) {
                        Some(Some(field)) => {
                            let init_ty = field.infer(e)?;
                            e.unify(init_ty, ty)?;
                        }
                        Some(None) => {
                            // Get name in scope
                            let init_ty = e
                                .table
                                .get_by_sym(e.at, &name)
                                .ok_or_else(|| InferenceError::NotFound(Path::from(name)))?;
                            e.unify(init_ty, ty)?;
                        }
                        None => Err(InferenceError::NotFound(Path::from(name)))?,
                    }
                }
                Ok(to)
            }
            _ => Err(InferenceError::NotFound(self.to.clone())),
        }
    }
}

impl<'a> Inference<'a> for Array {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Array { values } = self;
        let out = e.new_inferred();
        for value in values {
            let ty = value.infer(e)?;
            e.unify(out, ty)?;
        }
        Ok(e.new_array(out, values.len()))
    }
}

impl<'a> Inference<'a> for ArrayRep {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let ArrayRep { value, repeat } = self;
        let ty = value.infer(e)?;
        let rep = repeat.infer(e)?;
        let usize_ty = e.usize();
        e.unify(rep, usize_ty)?;
        match &repeat.kind {
            ExprKind::Literal(Literal::Int(repeat)) => Ok(e.new_array(ty, *repeat as usize)),
            _ => {
                todo!("TODO: constant folding before type checking?");
            }
        }
    }
}

impl<'a> Inference<'a> for AddrOf {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let AddrOf { mutable: _, expr } = self;
        // TODO: mut ref
        let ty = expr.infer(e)?;
        Ok(e.new_ref(ty))
    }
}

impl<'a> Inference<'a> for Quote {
    fn infer(&'a self, _e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        todo!("Quote: {self}")
    }
}

impl<'a> Inference<'a> for Literal {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let ty = match self {
            Literal::Bool(_) => e.bool(),
            Literal::Char(_) => e.char(),
            Literal::Int(_) => e.integer_literal(),
            Literal::Float(_) => e.float_literal(),
            Literal::String(_) => {
                let str_ty = e.str();
                e.new_ref(str_ty)
            }
        };
        Ok(e.new_inst(ty))
    }
}

impl<'a> Inference<'a> for Group {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Group { expr } = self;
        expr.infer(e)
    }
}

impl<'a> Inference<'a> for Block {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Block { stmts } = self;
        let mut e = e.block_scope();
        let empty = e.empty();
        if let [stmts @ .., ret] = stmts.as_slice() {
            for stmt in stmts {
                match (&stmt.kind, &stmt.semi) {
                    (StmtKind::Expr(expr), Semi::Terminated) => {
                        expr.infer(&mut e)?;
                    }
                    (StmtKind::Expr(expr), Semi::Unterminated) => {
                        let ty = expr.infer(&mut e)?;
                        e.unify(ty, empty)?;
                    }
                    _ => {}
                }
            }
            let out = if let StmtKind::Expr(expr) = &ret.kind {
                expr.infer(&mut e)?
            } else {
                empty
            };
            if Semi::Unterminated == ret.semi {
                return Ok(out);
            }
        }
        Ok(empty)
    }
}

impl<'a> Inference<'a> for Assign {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Assign { parts } = self;
        let (head, tail) = parts.as_ref();
        // Infer the tail expression
        let tail = tail.infer(e)?;
        // Infer the head expression
        let head = head.infer(e)?;
        // Unify head and tail
        e.unify(head, tail)?;
        // Return Empty
        Ok(e.empty())
    }
}

impl<'a> Inference<'a> for Modify {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Modify { kind: _, parts } = self;
        let (head, tail) = parts.as_ref();
        // Infer the tail expression
        let tail = tail.infer(e)?;
        // Infer the head expression
        let head = head.infer(e)?;
        // TODO: Search within the head type for `(op)_assign`
        e.unify(head, tail)?;
        // TODO: Typecheck `op_assign(&mut head, tail)`
        Ok(e.empty())
    }
}

impl<'a> Inference<'a> for Binary {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        use BinaryKind as Bk;
        let Binary { kind, parts } = self;
        let (head, tail) = parts.as_ref();
        // Infer the tail expression
        let tail = tail.infer(e)?;
        // Infer the head expression
        let head = head.infer(e)?;
        let head = e.prune(head);
        // TODO: Search within the head type for `(op)`
        match kind {
            BinaryKind::Call => match e.entry(head).ty() {
                Some(TypeKind::Adt(Adt::TupleStruct(types))) => {
                    let Some(TypeKind::Tuple(values)) = e.entry(tail).ty() else {
                        Err(InferenceError::Mismatch(head, tail))?
                    };
                    if types.len() != values.len() {
                        Err(InferenceError::FieldCount(head, types.len(), values.len()))?
                    }
                    let pairs = types
                        .iter()
                        .zip(values.iter())
                        .map(|(&(_vis, ty), &value)| (ty, value))
                        .collect::<Vec<_>>();
                    for (ty, value) in pairs {
                        e.unify(ty, value)?;
                    }
                    Ok(head)
                }
                Some(&TypeKind::FnSig { args, rety }) => {
                    e.unify(tail, args)?;
                    Ok(rety)
                }
                _ => Err(InferenceError::Mismatch(head, tail))?,
            },
            Bk::Lt | Bk::LtEq | Bk::Equal | Bk::NotEq | Bk::GtEq | Bk::Gt => {
                e.unify(head, tail)?;
                Ok(e.bool())
            }
            Bk::LogAnd | Bk::LogOr | Bk::LogXor => {
                let bool = e.bool();
                e.unify(head, bool)?;
                e.unify(tail, bool)?;
                Ok(bool)
            }
            // TODO: Don't return the generic form wholesale.
            Bk::RangeExc => Ok(e.table.get_lang_item("range_exc")),
            Bk::RangeInc => Ok(e.table.get_lang_item("range_exc")),
            Bk::Shl | Bk::Shr => {
                let shift_amount = e.u32();
                e.unify(tail, shift_amount)?;
                Ok(head)
            }
            Bk::BitAnd
            | Bk::BitOr
            | Bk::BitXor
            | Bk::Add
            | Bk::Sub
            | Bk::Mul
            | Bk::Div
            | Bk::Rem => {
                // Typecheck op(head, tail)
                e.unify(head, tail)?;
                Ok(head)
            }
        }
    }
}

impl<'a> Inference<'a> for Unary {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Unary { kind, tail } = self;
        match kind {
            UnaryKind::Deref => {
                let tail = tail.infer(e)?;
                let tail = e.def_usage(tail);
                // TODO: get the base type
                match e.entry(tail).ty() {
                    Some(&TypeKind::Ref(h)) => Ok(h),
                    _ => todo!("Deref {}", e.entry(tail)),
                }
            }
            UnaryKind::Loop => {
                let mut e = e.block_scope();
                // Enter a new breakset
                let mut e = e.open_bset();

                // Infer the fail branch
                let tail = tail.infer(&mut e)?;
                // Unify the pass branch with Empty
                let empt = e.empty();
                e.unify(tail, empt)?;

                // Return breakset
                match e.bset.get() {
                    Some(bset) => Ok(bset),
                    None => Ok(e.never()),
                }
            }
            _op => {
                // Infer the tail expression
                let tail = tail.infer(e)?;
                // TODO: Search within the tail type for `(op)`
                Ok(tail)
            }
        }
    }
}

impl<'a> Inference<'a> for Member {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Member { head, kind } = self;
        // Infer the head expression
        let head = head.infer(e)?;
        // Get the type of head
        let head = e.prune(head);
        let ty = e.entry(head);
        // Search within the head type for the memberkind
        match kind {
            MemberKind::Call(name, tuple) => {
                if let Some((args, rety)) = e.get_fn(ty.id(), *name) {
                    let values = iter::once(Ok(e.new_ref(head)))
                        // infer for each member-
                        .chain(tuple.exprs.iter().map(|expr| expr.infer(e)))
                        // Construct tuple
                        .collect::<Result<Vec<_>, InferenceError>>()
                        // Return tuple
                        .map(|tys| e.new_tuple(tys))?;
                    e.unify(args, values)?;
                    Ok(rety)
                } else {
                    Err(InferenceError::NotFound(Path::from(*name)))
                }
            }
            MemberKind::Struct(name) => match ty.nav(&[PathPart::Ident(*name)]) {
                Some(ty) => Ok(ty.id()),
                None => Err(InferenceError::NotFound(Path::from(*name))),
            },
            MemberKind::Tuple(Literal::Int(idx)) => match ty.ty() {
                Some(TypeKind::Tuple(tys)) => tys
                    .get(*idx as usize)
                    .copied()
                    .ok_or(InferenceError::FieldCount(head, tys.len(), *idx as usize)),
                Some(TypeKind::Adt(Adt::TupleStruct(tys))) => tys
                    .get(*idx as usize)
                    .map(|(_vis, ty)| *ty)
                    .ok_or(InferenceError::FieldCount(head, tys.len(), *idx as usize)),
                _ => Err(InferenceError::Mismatch(ty.id(), e.table.root())),
            },
            _ => Err(InferenceError::Mismatch(ty.id(), ty.root())),
        }
        // Type is required to be inferred at this point.
    }
}

impl<'a> Inference<'a> for Index {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Index { head, indices } = self;
        let usize = e.usize();
        // Infer the head expression
        let head = head.infer(e)?;
        let mut head = e.prune(head);
        // For each index expression:
        for index in indices {
            //   Infer the index type
            let index = index.infer(e)?;
            if let Some((args, rety)) = e.get_fn(head, "index".into()) {
                // Unify args and tuple (&head, index)
                let selfty = e.new_ref(head);
                let tupty = e.new_tuple(vec![selfty, index]);
                e.unify(args, tupty)?;
                head = e.prune(rety);
                continue;
            }
            //   Decide whether the head can be indexed by that type
            //   TODO: check for a `.index` method on the type
            match e.entry(head).ty().unwrap() {
                &TypeKind::Slice(handle) | &TypeKind::Array(handle, _) => {
                    e.unify(usize, index)?;
                    head = e.prune(handle);
                }
                other => todo!("Indexing on type {other}"),
            }
            //   head = result of indexing head
        }
        Ok(head)
    }
}

impl<'a> Inference<'a> for Cast {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Cast { head, ty } = self;
        // Infer the head expression
        let _head = head.infer(e)?;
        // Evaluate the type
        let ty = ty
            .evaluate(e.table, e.at)
            .map_err(InferenceError::AnnotationEval)?;
        // Decide whether the type is castable
        // TODO: not deciding is absolutely unsound!!!
        // Return the type
        Ok(ty)
    }
}

impl<'a> Inference<'a> for Path {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        e.by_name(self)
            .map_err(|_| InferenceError::NotFound(self.clone()))
    }
}

impl<'a> Inference<'a> for Let {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Let { mutable: _, name, ty, init } = self;
        let ty = match ty {
            Some(ty) => ty
                .evaluate(e.table, e.at)
                .map_err(InferenceError::AnnotationEval)?,
            None => e.new_inferred(),
        };
        // Infer the initializer
        if let Some(init) = init {
            // Unify the initializer and the ty
            let initty = init.infer(e)?;
            e.unify(ty, initty)?;
        }
        // Deep copy the ty, if it exists
        let ty = e.deep_clone(ty);
        // Infer the pattern
        let patty = name.infer(e)?;
        // Unify the pattern and the ty
        e.unify(ty, patty)?;
        // `if let` returns whether the pattern succeeded or not
        Ok(e.bool())
    }
}

impl<'a> Inference<'a> for Match {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Match { scrutinee, arms } = self;
        // Infer the scrutinee
        let scrutinee = scrutinee.infer(e)?;

        let mut out = None;
        // For each pattern:
        for MatchArm(pat, expr) in arms {
            let mut scope = e.block_scope();
            // Infer the pattern
            let pat = pat.infer(&mut scope)?;
            // Unify it with the scrutinee
            scope.unify(scrutinee, pat)?;
            // Infer the Expr
            let expr = expr.infer(&mut scope)?;
            // Unify the expr with the out variable
            match out {
                Some(ty) => e.unify(ty, expr)?,
                None => out = Some(expr),
            }
        }
        // Return out. If there are no arms, assume Never.
        match out {
            Some(ty) => Ok(ty),
            None => Ok(e.never()),
        }
    }
}

impl<'a> Inference<'a> for Pattern {
    // TODO: This is the wrong way to typeck pattern matching.
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        match self {
            Pattern::Name(name) => {
                // Evaluating a pattern creates and enters a new scope.
                // Surely this will cause zero problems.
                e.local_scope(*name);
                e.table.set_ty(e.at, TypeKind::Inferred);
                Ok(e.at)
            }
            Pattern::Path(path) => {
                // Evaluating a path pattern puts type constraints on the scrutinee
                path.evaluate(e.table, e.at)
                    .map_err(|_| InferenceError::NotFound(path.clone()))
            }
            Pattern::Literal(literal) => literal.infer(e),
            Pattern::Rest(Some(pat)) => {
                eprintln!("TODO: Rest patterns in tuples?");
                let ty = pat.infer(e)?;
                Ok(e.new_slice(ty))
            }
            Pattern::Rest(_) => Ok(e.new_inferred()),
            Pattern::Ref(_, pattern) => {
                let ty = pattern.infer(e)?;
                Ok(e.new_ref(ty))
            }
            Pattern::RangeExc(pat1, pat2) => {
                let ty1 = pat1.infer(e)?;
                let ty2 = pat2.infer(e)?;
                e.unify(ty1, ty2)?;
                Ok(ty1)
            }
            Pattern::RangeInc(pat1, pat2) => {
                let ty1 = pat1.infer(e)?;
                let ty2 = pat2.infer(e)?;
                e.unify(ty1, ty2)?;
                Ok(ty1)
            }
            Pattern::Tuple(patterns) => {
                let tys = patterns
                    .iter()
                    .map(|pat| pat.infer(e))
                    .collect::<Result<Vec<Handle>, InferenceError>>()?;
                Ok(e.new_tuple(tys))
            }
            Pattern::Array(patterns) => match patterns.as_slice() {
                // TODO: rest patterns here
                [one, rest @ ..] => {
                    let ty = one.infer(e)?;
                    for rest in rest {
                        let ty2 = rest.infer(e)?;
                        e.unify(ty, ty2)?;
                    }
                    Ok(e.new_slice(ty))
                }
                [] => {
                    let ty = e.new_inferred();
                    Ok(e.new_slice(ty))
                }
            },
            Pattern::Struct(_path, _items) => {
                eprintln!("TODO: struct patterns: {self}");
                Ok(e.empty())
            }
            Pattern::TupleStruct(path, patterns) => {
                eprintln!("TODO: tuple struct patterns: {self}");
                let struc = e.by_name(path)?;
                let Some(TypeKind::Adt(Adt::TupleStruct(ts))) = e.entry(struc).ty() else {
                    Err(InferenceError::Mismatch(struc, e.never()))?
                };
                let ts: Vec<_> = ts.iter().map(|(_v, h)| *h).collect();
                let tys = patterns
                    .iter()
                    .map(|pat| pat.infer(e))
                    .collect::<Result<Vec<Handle>, InferenceError>>()?;
                let ts = e.new_tuple(ts);
                let tup = e.new_tuple(tys);
                e.unify(ts, tup)?;
                Ok(struc)
            }
        }
    }
}

impl<'a> Inference<'a> for While {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let While { cond, pass, fail } = self;
        // Infer the condition
        let cond = cond.infer(e)?;
        // Unify the condition with bool
        let bool = e.bool();
        e.unify(bool, cond)?;

        // Infer the fail branch
        let fail = fail.infer(e)?;
        // Unify the fail branch with breakset
        let mut e = e.open_bset();

        // Infer the pass branch
        let pass = pass.infer(&mut e)?;
        // Unify the pass branch with Empty
        let empt = e.empty();
        e.unify(pass, empt)?;

        match e.bset.get() {
            None => Ok(e.empty()),
            Some(bset) => {
                e.unify(fail, bset)?;
                Ok(fail)
            }
        }
    }
}

impl<'a> Inference<'a> for If {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let If { cond, pass, fail } = self;
        // Do inference on the condition'
        let cond = cond.infer(e)?;
        // Unify the condition with bool
        let bool = e.bool();
        e.unify(bool, cond)?;
        // Do inference on the pass branch
        let pass = pass.infer(e)?;
        // Do inference on the fail branch
        let fail = fail.infer(e)?;
        // Unify pass and fail
        e.unify(pass, fail)?;
        // Return the result
        Ok(pass)
    }
}

impl<'a> Inference<'a> for For {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let For { bind, cond, pass, fail } = self;
        let mut e = e.block_scope();

        let bind = bind.infer(&mut e)?;

        // What does it mean to be iterable? Why, `next()`, of course!
        let cond = cond.infer(&mut e)?;
        let cond = e.prune(cond);
        if let Some((args, rety)) = e.get_fn(cond, "next".into()) {
            // Check that the args are correct
            let params = vec![e.new_ref(cond)];
            let params = e.new_tuple(params);
            e.unify(args, params)?;
            e.unify(rety, bind)?;
        }

        // Enter a new breakset
        let mut e = e.open_bset();

        // Infer the fail branch
        let fail = fail.infer(&mut e)?;
        // Open a breakset
        let mut e = e.open_bset();

        // Infer the pass branch
        let pass = pass.infer(&mut e)?;
        // Unify the pass branch with Empty
        let empt = e.empty();
        e.unify(pass, empt)?;

        // Return breakset
        if let Some(bset) = e.bset.get() {
            e.unify(fail, bset)?;
            Ok(fail)
        } else {
            Ok(e.empty())
        }
    }
}

impl<'a> Inference<'a> for Else {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        self.body.infer(e)
    }
}

impl<'a> Inference<'a> for Break {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Break { body } = self;
        // Infer the body of the break
        let ty = body.infer(e)?;
        // Unify it with the breakset of the loop
        e.bset(ty)?;
        // Return never
        Ok(e.never())
    }
}

impl<'a> Inference<'a> for Return {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        let Return { body } = self;
        // Infer the body of the return
        let ty = body.infer(e)?;
        // Unify it with the return-set of the function
        e.rset(ty)?;
        // Return never
        Ok(e.never())
    }
}

impl<'a, I: Inference<'a>> Inference<'a> for Option<I> {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        match self {
            Some(expr) => expr.infer(e),
            None => Ok(e.empty()),
        }
    }
}
impl<'a, I: Inference<'a>> Inference<'a> for Box<I> {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        self.as_ref().infer(e)
    }
}
