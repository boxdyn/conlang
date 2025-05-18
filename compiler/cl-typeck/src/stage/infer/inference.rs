//! The [Inference] trait is the heart of cl-typeck's type inference.
//!
//! Each syntax structure must describe how to unify its types.

use std::iter;

use super::{engine::InferenceEngine, error::InferenceError};
use crate::{
    handle::Handle,
    table::NodeKind,
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

impl<'a> Inference<'a> for cl_ast::Expr {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        self.kind.infer(e)
    }
}

impl<'a> Inference<'a> for cl_ast::ExprKind {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> IfResult {
        match self {
            ExprKind::Empty => Ok(e.empty()),
            ExprKind::Closure(_) => todo!("Infer the type of a closure"),
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
        let out = e.new_var();
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
        Ok(e.new_array(ty, *repeat))
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
            match (&ret.kind, &ret.semi) {
                (StmtKind::Expr(expr), Semi::Terminated) => {
                    expr.infer(&mut e)?;
                }
                (StmtKind::Expr(expr), Semi::Unterminated) => {
                    return expr.infer(&mut e);
                }
                _ => {}
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
            Bk::RangeExc => todo!("Ranges in the type checker"),
            Bk::RangeInc => todo!("Ranges in the type checker"),
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
                // TODO: get the base type
                match e.entry(tail).ty() {
                    Some(&TypeKind::Ref(h)) => Ok(h),
                    other => todo!("Deref {other:?}"),
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
                Ok(e.bset)
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
        // Deep copy the ty, if it exists
        let ty = match ty {
            Some(ty) => {
                let ty = ty
                    .evaluate(e.table, e.at)
                    .map_err(InferenceError::AnnotationEval)?;
                e.deep_clone(ty)
            }
            None => e.new_var(),
        };
        // Infer the initializer
        if let Some(init) = init {
            // Unify the initializer and the ty
            let initty = init.infer(e)?;
            e.unify(ty, initty)?;
        }
        // Enter a local scope (modifies the current scope)
        e.local_scope();
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
                let node = e.table.new_entry(e.at, NodeKind::Local);
                e.table.set_ty(node, TypeKind::Variable);
                e.table.add_child(e.at, *name, node);
                e.at = node;
                Ok(node)
            }
            Pattern::Path(path) => {
                // Evaluating a path pattern puts type constraints on the scrutinee
                path.evaluate(e.table, e.at)
                    .map_err(|_| InferenceError::NotFound(path.clone()))
            }
            Pattern::Literal(literal) => literal.infer(e),
            Pattern::Rest(Some(pat)) => pat.infer(e), // <-- glaring soundness holes
            Pattern::Rest(_) => todo!("Fix glaring soundness holes in pattern"),
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
                    let ty = e.new_var();
                    Ok(e.new_slice(ty))
                }
            },
            Pattern::Struct(_path, _items) => todo!("Struct patterns"),
            Pattern::TupleStruct(_path, _patterns) => todo!("Tuple struct patterns"),
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
        let mut e = InferenceEngine { bset: fail, ..e.scoped() };

        // Infer the pass branch
        let pass = pass.infer(&mut e)?;
        // Unify the pass branch with Empty
        let empt = e.empty();
        e.unify(pass, empt)?;

        // Return breakset
        Ok(e.bset)
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
        // Unify the fail branch with breakset
        let mut e = InferenceEngine { bset: fail, ..e.scoped() };
        e.bset = fail;

        // Infer the pass branch
        let pass = pass.infer(&mut e)?;
        // Unify the pass branch with Empty
        let empt = e.empty();
        e.unify(pass, empt)?;

        // Return breakset
        Ok(e.bset)
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
        e.unify(ty, e.bset)?;
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
        e.unify(ty, e.rset)?;
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
