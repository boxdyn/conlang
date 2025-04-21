//! The [Inference] trait is the heart of cl-typeck's type inference.
//!
//! Each syntax structure must describe how to unify its types.

use std::iter;

use super::{engine::InferenceEngine, error::InferenceError};
use crate::{handle::Handle, type_expression::TypeExpression, type_kind::TypeKind};
use cl_ast::*;

pub trait Inference<'a> {
    /// Performs type inference
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> Result<Handle, InferenceError>;
}

impl<'a> Inference<'a> for cl_ast::Expr {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> Result<Handle, InferenceError> {
        self.kind.infer(e)
    }
}

impl<'a> Inference<'a> for cl_ast::ExprKind {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> Result<Handle, InferenceError> {
        match self {
            ExprKind::Empty => Ok(e.from_type_kind(TypeKind::Empty)),
            ExprKind::Quote(quote) => todo!("Quote: {quote}"),
            ExprKind::Let(l) => {
                let Let { mutable: _, name, ty, init } = l;
                // Infer the pattern
                let patty = name.infer(e)?;
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
                // Unify the pattern and the ty
                e.unify(ty, patty)?;
                // Infer the initializer
                if let Some(init) = init {
                    // Unify the initializer and the ty
                    let initty = init.infer(e)?;
                    e.unify(ty, initty)?;
                }
                Ok(ty)
            }
            ExprKind::Match(m) => {
                let Match { scrutinee, arms } = m;
                // Infer the scrutinee
                let scrutinee = scrutinee.infer(e)?;

                let scope = e.new_var();
                for arm in arms {
                    let _ty = arm.infer(e)?;
                }
                // For each pattern:
                //   Infer the pattern
                //   Unify it with the scrutinee
                //   Infer the Expr
                //   Unify the expr with the out variable
                // Return out
                todo!("Match: {m} {scrutinee} {scope}")
            }
            ExprKind::Assign(assign) => {
                let Assign { parts } = assign;
                let (head, tail) = parts.as_ref();
                // Infer the tail expression
                let tail = tail.infer(e)?;
                // Infer the head expression
                let head = head.infer(e)?;
                // Unify head and tail
                e.unify(head, tail)?;
                // Return Empty
                Ok(e.from_type_kind(TypeKind::Empty))
            }
            ExprKind::Modify(modify) => {
                let Modify { kind: _, parts } = modify;
                let (head, tail) = parts.as_ref();
                // Infer the tail expression
                let tail = tail.infer(e)?;
                // Infer the head expression
                let head = head.infer(e)?;
                // TODO: Search within the head type for `(op)_assign`
                e.unify(head, tail)?;
                // TODO: Typecheck `op_assign(&mut head, tail)`
                Ok(e.from_type_kind(TypeKind::Empty))
            }
            ExprKind::Binary(binary) => {
                let Binary { kind, parts } = binary;
                let (head, tail) = parts.as_ref();
                // Infer the tail expression
                let tail = tail.infer(e)?;
                // Infer the head expression
                let head = head.infer(e)?;
                // TODO: Search within the head type for `(op)`
                if let BinaryKind::Call = kind {
                    let rety = e.new_var();
                    let tail = e.from_type_kind(TypeKind::FnSig { args: tail, rety });
                    e.unify(head, tail)?;
                    Ok(rety)
                } else {
                    // Typecheck op(head, tail)
                    e.unify(head, tail)?;
                    Ok(head)
                }
            }
            ExprKind::Unary(Unary { kind: UnaryKind::Loop, tail }) => {
                let mut e = e.block_scope();
                // Enter a new breakset
                let mut e = e.open_bset();

                // Infer the fail branch
                let tail = tail.infer(&mut e)?;
                // Unify the pass branch with Empty
                let empt = e.from_type_kind(TypeKind::Empty);
                e.unify(tail, empt)?;

                // Return breakset
                Ok(e.bset)
            }
            ExprKind::Unary(unary) => {
                let Unary { kind: _, tail } = unary;
                // Infer the tail expression
                let tail = tail.infer(e)?;
                // TODO: Search within the tail type for `(op)`
                // Typecheck `(op)(tail)`
                Ok(tail)
            }
            ExprKind::Cast(cast) => {
                let Cast { head, ty } = cast;
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
            ExprKind::Member(member) => {
                let Member { head, kind } = member;
                // Infer the head expression
                let head = head.infer(e)?;
                // Get the type of head
                let ty = e.entry(e.de_inst(head));
                // Search within the head type for the memberkind
                match kind {
                    MemberKind::Call(name, tuple) => match ty.nav(&[PathPart::Ident(*name)]) {
                        Some(ty) => match e.entry(e.de_inst(ty.id())).ty() {
                            Some(&TypeKind::FnSig { args, rety }) => {
                                let values = iter::once(Ok(ty.id()))
                                    .chain(
                                        tuple
                                            .exprs
                                            .iter()
                                            // Infer each member
                                            .map(|expr| expr.infer(e)),
                                    )
                                    // Construct tuple
                                    .collect::<Result<Vec<_>, InferenceError>>()
                                    // Return tuple
                                    .map(|tys| e.from_type_kind(TypeKind::Tuple(tys)))?;
                                e.unify(args, values)?;
                                Ok(rety)
                            }
                            other => todo!("member-call {other:?}"),
                        },
                        None => Err(InferenceError::NotFound(Path::from(*name))),
                    },
                    MemberKind::Struct(name) => match ty.nav(&[PathPart::Ident(*name)]) {
                        Some(ty) => Ok(ty.id()),
                        None => Err(InferenceError::NotFound(Path::from(*name))),
                    },
                    MemberKind::Tuple(Literal::Int(idx)) => match ty.ty() {
                        Some(TypeKind::Tuple(tys)) => Ok(tys[*idx as usize]),
                        _ => Err(InferenceError::Mismatch(ty.id(), e.table.root())),
                    },
                    _ => Err(InferenceError::Mismatch(ty.id(), ty.root())),
                }
                // Type is required to be inferred at this point.
            }
            ExprKind::Index(index) => {
                // Infer the head expression
                // For each index expression:
                //   Infer the index type
                //   Decide whether the head can be indexed by that type
                //   head = result of indexing head
                todo!("Index {index}")
            }
            ExprKind::Structor(structor) => {
                // Evaluate the path in the current context
                // Typecheck the fielders against the fields
                todo!("Structor {structor}")
            }
            ExprKind::Path(path) => e
                .by_name(path)
                .map_err(|_| InferenceError::NotFound(path.clone())),
            ExprKind::Literal(literal) => literal.infer(e),
            ExprKind::Array(array) => {
                let Array { values } = array;
                let out = e.new_var();
                for value in values {
                    let ty = value.infer(e)?;
                    e.unify(out, ty)?;
                }
                Ok(out)
            }
            ExprKind::ArrayRep(array_rep) => {
                let ArrayRep { value, repeat } = array_rep;
                let ty = value.infer(e)?;
                Ok(e.from_type_kind(TypeKind::Array(ty, *repeat)))
            }
            ExprKind::AddrOf(addr_of) => {
                let AddrOf { mutable: _, expr } = addr_of;
                // TODO: mut ref
                let ty = expr.infer(e)?;
                Ok(e.from_type_kind(TypeKind::Ref(ty)))
            }
            ExprKind::Block(block) => block.infer(e),
            ExprKind::Group(group) => {
                let Group { expr } = group;
                expr.infer(e)
            }
            ExprKind::Tuple(tuple) => tuple.infer(e),

            ExprKind::While(w) => {
                let While { cond, pass, fail } = w;
                // Infer the condition
                let cond = cond.infer(e)?;
                // Unify the condition with bool
                let boule = e
                    .primitive("bool".into())
                    .expect("Primitive bool should exist!");
                e.unify(boule, cond)?;
                // Enter a new breakset
                let mut e = e.open_bset();

                // Infer the fail branch
                let fail = fail.infer(&mut e)?;
                // Unify the fail branch with breakset
                e.bset = fail;

                // Infer the pass branch
                let pass = pass.infer(&mut e)?;
                // Unify the pass branch with Empty
                let empt = e.from_type_kind(TypeKind::Empty);
                e.unify(pass, empt)?;

                // Return breakset
                Ok(e.bset)
            }
            ExprKind::If(i) => {
                let If { cond, pass, fail } = i;
                // Do inference on the condition'
                let cond = cond.infer(e)?;
                // Unify the condition with bool
                let boule = e
                    .primitive("bool".into())
                    .expect("Primitive bool should exist!");
                e.unify(boule, cond)?;
                // Do inference on the pass branch
                let pass = pass.infer(e)?;
                // Do inference on the fail branch
                let fail = fail.infer(e)?;
                // Unify pass and fail
                e.unify(pass, fail)?;
                // Return the result
                Ok(pass)
            }
            ExprKind::For(f) => todo!("For {f}"),
            ExprKind::Break(b) => {
                let Break { body } = b;
                // Infer the body of the break
                let ty = body.infer(e)?;
                // Unify it with the breakset of the loop
                e.unify(ty, e.bset)?;
                // Return never
                Ok(e.from_type_kind(TypeKind::Never))
            }
            ExprKind::Return(r) => {
                let Return { body } = r;
                // Infer the body of the return
                let ty = body.infer(e)?;
                // Unify it with the return-set of the function
                e.unify(ty, e.rset)?;
                // Return never
                Ok(e.from_type_kind(TypeKind::Never))
            }
            ExprKind::Continue => Ok(e.from_type_kind(TypeKind::Never)),
        }
    }
}

impl<'a> Inference<'a> for MatchArm {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> Result<Handle, InferenceError> {
        let MatchArm(pat, expr) = self;
        let table = &mut e.table;
        let scope = table.new_entry(e.at, crate::table::NodeKind::Local);
        let mut e = e.at(scope);

        let pat_ty = pat.infer(&mut e)?;
        // TODO: bind pattern variables in scope
        let expr_ty = expr.infer(&mut e)?;
        todo!("Finish pattern-matching: {pat_ty}, {expr_ty}")
    }
}

impl<'a> Inference<'a> for Pattern {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> Result<Handle, InferenceError> {
        match self {
            Pattern::Name(name) => {
                let child = e.new_var();
                e.table.add_child(e.at, *name, child);
                Ok(child)
            }
            Pattern::Literal(literal) => literal.infer(e),
            Pattern::Rest(_) => todo!("Infer rest-patterns"),
            Pattern::Ref(_, pattern) => {
                let ty = pattern.infer(e)?;
                Ok(e.from_type_kind(TypeKind::Ref(ty)))
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
                Ok(e.from_type_kind(TypeKind::Tuple(tys)))
            }
            Pattern::Array(patterns) => match patterns.as_slice() {
                [one, rest @ ..] => {
                    let ty = one.infer(e)?;
                    for rest in rest {
                        let ty2 = rest.infer(e)?;
                        e.unify(ty, ty2)?;
                    }
                    Ok(e.from_type_kind(TypeKind::Slice(ty)))
                }
                [] => {
                    let ty = e.new_var();
                    Ok(e.from_type_kind(TypeKind::Slice(ty)))
                }
            },
            Pattern::Struct(_path, _items) => todo!(),
            Pattern::TupleStruct(_path, _patterns) => todo!(),
        }
    }
}

impl<'a> Inference<'a> for Tuple {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> Result<Handle, InferenceError> {
        let Tuple { exprs } = self;
        exprs
            .iter()
            // Infer each member
            .map(|expr| expr.infer(e))
            // Construct tuple
            .collect::<Result<Vec<_>, InferenceError>>()
            // Return tuple
            .map(|tys| e.from_type_kind(TypeKind::Tuple(tys)))
    }
}

impl<'a> Inference<'a> for Block {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> Result<Handle, InferenceError> {
        let Block { stmts } = self;
        let mut e = e.block_scope();
        let empty = e.from_type_kind(TypeKind::Empty);
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

impl<'a> Inference<'a> for Else {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> Result<Handle, InferenceError> {
        self.body.infer(e)
    }
}

impl<'a, I: Inference<'a>> Inference<'a> for Option<I> {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> Result<Handle, InferenceError> {
        match self {
            Some(expr) => expr.infer(e),
            None => Ok(e.from_type_kind(TypeKind::Empty)),
        }
    }
}
impl<'a, I: Inference<'a>> Inference<'a> for Box<I> {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> Result<Handle, InferenceError> {
        self.as_ref().infer(e)
    }
}

impl<'a> Inference<'a> for Literal {
    fn infer(&'a self, e: &mut InferenceEngine<'_, 'a>) -> Result<Handle, InferenceError> {
        let ty = match self {
            Literal::Bool(_) => Ok(e
                .primitive("bool".into())
                .expect("Primitive bool should exist!")),
            Literal::Char(_) => Ok(e
                .primitive("char".into())
                .expect("Primitive char should exist!")),
            Literal::Int(_) => Ok(e
                .primitive("isize".into())
                .expect("Primitive isize should exist!")),
            Literal::Float(_) => Ok(e
                .primitive("f64".into())
                .expect("Primitive f64 should exist!")),
            Literal::String(_) => Ok(e
                .primitive("str".into())
                .expect("Primitive str should exist!")),
        }?;
        Ok(e.new_inst(ty))
    }
}
