//! The [Inference] trait is the heart of cl-typeck's type inference.
//!
//! Each syntax structure must describe how to unify its types.

use super::{engine::InferenceEngine, error::InferenceError};
use crate::{consteval::ConstEval, table::Scope, type_expression::TypeExpression};
use cl_ast::{types::Literal, *};

// TODO: "Infer" the types of Items

type IfResult = Result<Scope, InferenceError>;

pub trait Inference {
    /// Performs type inference
    fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult;
}

impl<T: AstNode + Inference, A: AstTypes> Inference for At<T, A> {
    fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
        self.0.infer(e)
    }
}

impl Inference for Expr {
    fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
        let out = match self {
            Self::Omitted => Ok(e.unit()),
            Self::Id(v) => v
                .evaluate(e.table, e.at)
                .map_err(InferenceError::AnnotationEval),
            Self::MetId(_) => todo!("Cannot perform type inference on macro identifier {self}"),
            Self::Lit(literal) => literal.infer(e),
            Self::Use(_) => Ok(e.unit()),
            Self::Bind(bind) => bind.infer(e),
            Self::Make(make) => make.infer(e),
            Self::Match(mtch) => mtch.infer(e),
            Self::Label(labl) => labl.infer(e),
            Self::Op(op, exprs) => infer_expr_op(*op, exprs, e),
        }?;
        println!("Inferred {self}: {}", e.entry(out));
        Ok(out)
    }
}

fn infer_expr_op(op: Op, exprs: &[At<Expr>], e: &mut InferenceEngine<'_, '_>) -> IfResult {
    match (op, exprs) {
        (Op::Do, []) => Ok(e.unit()),
        (Op::Do, [ignored @ .., returned]) => {
            for expr in ignored {
                expr.infer(e)?;
            }
            returned.infer(e)
        }
        (Op::As, [expr, ty]) => todo!("Infer {expr} as {ty}"),
        (Op::Macro, [..]) => todo!("Infer {}", op),
        (Op::Block, []) => Ok(e.unit()),
        (Op::Block, [body]) => body.infer(e),
        (Op::Array, items) => {
            let out = e.new_inferred();
            for item in items {
                let ty = item.infer(e)?;
                e.unify(out, ty)?;
            }
            Ok(e.new_array(out, items.len()))
        }
        (Op::ArRep, [value, rep]) => {
            let Some(size) = rep.const_eval().and_then(|v| v.uint()) else {
                Err(crate::type_expression::Error::ConstEval {
                    parent: e.at,
                    eval: Box::new(rep.clone()),
                })?
            };
            let ty = value.infer(e)?;
            Ok(e.new_array(ty, size as _))
        }
        (Op::Group, [value]) => value.infer(e),
        (Op::Tuple, []) => Ok(e.unit()),
        (Op::Tuple, exprs) => exprs
            .iter()
            .map(|expr| expr.infer(e))
            .collect::<Result<Vec<_>, InferenceError>>()
            .map(|tys| e.new_tuple(tys)),
        (Op::MetaInner | Op::MetaOuter, [_, body]) => body.infer(e),
        (Op::Try, [lhs]) => todo!("Infer {lhs}{op}"),
        (Op::Index, [lhs, rhs]) => todo!("Infer {lhs}[{rhs}]"),
        (Op::Call, [lhs, rhs]) => todo!("Infer {lhs}({rhs}) (generalize)"),
        (Op::Dot, [lhs, At(Expr::Op(Op::Call, exprs), _)])
            if let [rhs, args] = exprs.as_slice() =>
        {
            // infer lhs
            // in lhs scope, infer rhs
            // infer args
            // if rhs passes self-test, prepend lhs to inferred args
            // unify with rhs
            // TODO: how do we distinguish member access from function call?
            todo!("Dotcall: {lhs}.{rhs}{args} (generalize)")
        }
        (Op::Pub | Op::Const | Op::Static, [expr]) => expr.infer(e),
        (Op::Loop, [expr]) => {
            fn infer_loop(expr: &At<Expr>, e: &mut InferenceEngine<'_, '_>) -> IfResult {
                let (bset, mut scope) = e.open_bset("loop");
                let body = scope.infer(expr)?;
                let unit = scope.unit();
                scope.unify(body, unit)?;
                let never = scope.never();
                scope.unify(bset, never)?;
                Ok(bset)
            }
            infer_loop(expr, e)
        }
        (Op::If, [cond, pass, fail @ ..]) => {
            // Open a block scope so the condition doesn't escape
            let pass = {
                let mut scope = e.block_scope();
                // Do inference on the condition'
                let cond = cond.infer(&mut scope)?;
                // Unify the condition with bool
                let bool = scope.bool();
                scope.unify(bool, cond)?;
                // Do inference on the pass branch
                pass.infer(&mut scope)?
            };

            // Do inference on the fail branch
            let fail = if let [fail] = fail {
                fail.infer(&mut e.block_scope())?
            } else {
                e.unit()
            };

            // Unify pass and fail
            e.unify(pass, fail)?;
            // Return the result
            Ok(pass)
        }
        (Op::While, [cond, pass, fail @ ..]) => {
            // Open a block scope so the loop condition doesn't escape
            let bset = {
                let mut scope = e.block_scope();
                // Infer the condition
                let cond = cond.infer(&mut scope)?;
                // Unify the condition with bool
                let bool = scope.bool();
                scope.unify(bool, cond)?;

                // Open a breakset for the pass branch
                {
                    let (bset, mut body_scope) = scope.open_bset("while");
                    // Infer the pass branch
                    let pass = pass.infer(&mut body_scope)?;
                    // Unify the pass branch with Empty
                    let empt = body_scope.unit();
                    body_scope.unify(empt, pass)?;
                    bset
                }
            };

            // Infer the fail branch
            let fail = if let [fail] = fail { fail.infer(e)? } else { e.unit() };

            // Unify the fail branch with breakset
            println!("bset: {}", e.entry(bset));
            e.unify(bset, fail)?;
            Ok(fail)
        }
        (Op::Break, [body]) if let Expr::Label(label) = body.value() => {
            let Label(label, body) = label.as_ref();
            let ty = body.infer(e)?;
            // // Unify it with the breakset of the loop
            e.bset(Some(label.to_ref()), ty)?;
            // // Return never
            Ok(e.never())
        }
        (Op::Break, [body]) => {
            let ty = body.infer(e)?;
            // Unify it with the breakset of the loop
            e.bset(None, ty)?;
            // Return never
            Ok(e.never())
        }
        (Op::Return, [body]) => {
            let ty = body.infer(e)?;
            // Unify it with the returnset of the function
            e.rset(ty)?;
            // Return never
            Ok(e.never())
        }
        (Op::Continue, []) => Ok(e.never()),
        (Op::Dot, [lhs, rhs]) => todo!("Infer {lhs}{op}{rhs}"),
        (Op::RangeEx, [..]) => todo!("Infer {op}"),
        (Op::RangeIn, [..]) => todo!("Infer {op}"),
        (Op::Neg, [rhs]) => todo!("Infer {op}{rhs}"),
        (Op::Not, [rhs]) => todo!("Infer {op}{rhs}"),
        (Op::Identity, [rhs]) => rhs.infer(e),
        (Op::Refer, [rhs]) => {
            let ty = rhs.infer(e)?;
            Ok(e.new_ref(ty))
        }
        (Op::Deref, [rhs]) => todo!("Infer {op}{rhs}"),
        (Op::Mul | Op::Div | Op::Rem | Op::Add | Op::Sub, [lhs, rhs]) => {
            let lty = lhs.infer(e)?;
            let rty = rhs.infer(e)?;
            e.unify(lty, rty)?;
            Ok(lty)
            // TODO: Look up operator overloads!
        }
        (Op::Shl | Op::Shr, [lhs, rhs]) => {
            let lhs = lhs.infer(e)?;
            let rhs = rhs.infer(e)?;
            let i32 = e.primitive("i32");
            e.unify(rhs, i32)?;

            Ok(lhs)
        }
        (Op::And | Op::Xor | Op::Or, [lhs, rhs]) => {
            let lhs = lhs.infer(e)?;
            let rhs = rhs.infer(e)?;
            e.unify(lhs, rhs)?;

            Ok(lhs)
        }
        (Op::Lt | Op::Leq | Op::Eq | Op::Neq | Op::Geq | Op::Gt, [lhs, rhs]) => {
            let lhs = lhs.infer(e)?;
            let rhs = rhs.infer(e)?;
            e.unify(lhs, rhs)?;

            Ok(e.bool())
        }
        (Op::LogAnd | Op::LogXor | Op::LogOr, [lhs, rhs]) => {
            let lhs = lhs.infer(e)?;
            let rhs = rhs.infer(e)?;
            // TODO: clarify truthiness
            e.unify(lhs, rhs)?;
            Ok(lhs)
        }
        (Op::Set, [lhs, rhs]) => {
            let lhs = lhs.infer(e)?;
            let rhs = rhs.infer(e)?;
            e.unify(lhs, rhs)?;
            Ok(e.unit()) // TODO: decide semantics of assignment
        }

        // TODO: Look up operator overloads!
        (Op::MulSet, [lhs, rhs]) => todo!("Infer {lhs}{op}{rhs}"),
        (Op::DivSet, [lhs, rhs]) => todo!("Infer {lhs}{op}{rhs}"),
        (Op::RemSet, [lhs, rhs]) => todo!("Infer {lhs}{op}{rhs}"),
        (Op::AddSet, [lhs, rhs]) => todo!("Infer {lhs}{op}{rhs}"),
        (Op::SubSet, [lhs, rhs]) => todo!("Infer {lhs}{op}{rhs}"),
        (Op::ShlSet, [lhs, rhs]) => todo!("Infer {lhs}{op}{rhs}"),
        (Op::ShrSet, [lhs, rhs]) => todo!("Infer {lhs}{op}{rhs}"),
        (Op::AndSet, [lhs, rhs]) => todo!("Infer {lhs}{op}{rhs}"),
        (Op::XorSet, [lhs, rhs]) => todo!("Infer {lhs}{op}{rhs}"),
        (Op::OrSet, [lhs, rhs]) => todo!("Infer {lhs}{op}{rhs}"),
        (op, exprs) => unimplemented!("ICE: malformed expression {op:?} {exprs:?}"),
    }
}

impl Inference for Literal {
    fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
        Ok(match self {
            Self::Bool(_) => e.bool(),
            Self::Char(_) => e.char(),
            Self::Int(_, _) => e.integer_literal(),
            Self::Str(_) => e.str(),
        })
    }
}

impl Inference for Label {
    fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
        let Label(label, expr) = self;
        let (bset, mut scope) = e.open_bset(label.to_ref());
        let ty = expr.infer(&mut scope)?;
        e.unify(bset, ty)?;
        Ok(bset)
    }
}

impl Inference for Bind {
    fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
        let Bind(op, _gens, pat, exprs) = self;
        match (op, exprs.as_slice()) {
            (BindOp::Let, [bind]) => {
                //todo!("Unify {pat} with {bind}, return bool"),
                let bind = bind.infer(e)?;
                let pat = e.by_name(pat)?;
                e.unify(pat, bind)?;
                Ok(e.bool())
            }
            (BindOp::Let, [bind, fail]) => {
                //todo!("Unify {pat} with {bind} and {fail}, return unit"),'
                let bind = bind.infer(e)?;
                let fail = fail.infer(e)?;
                let pat = e.by_name(pat)?;
                e.unify(pat, bind)?;
                e.unify(pat, fail)?;
                Ok(e.unit())
            }
            (BindOp::Type, []) => Ok(e.by_name(pat)?),
            (BindOp::Type, [body]) => {
                println!("Bind names!");
                let ty = e.by_name(pat)?;
                let body = body.infer(e)?;
                e.unify(ty, body)?;
                Ok(ty)
            }
            (BindOp::Fn, [body]) => {
                let loc = e.by_name(pat)?;
                e.at(loc).infer(body)
            }
            (BindOp::Mod, [body]) => {
                if let Ok(loc) = e.by_name(pat) {
                    e.at(loc).infer(body)
                } else {
                    e.infer(body)
                }
            }
            (BindOp::Impl, [body]) => {
                let loc = e.by_name(pat)?;
                // TODO: Properly scope impl targets
                e.at(loc).infer(body)
            }
            (BindOp::Struct, []) => Ok(e.by_name(pat)?),
            (BindOp::Enum, []) => Ok(e.by_name(pat)?),
            (BindOp::For, [iter, pass, fail]) => todo!("Infer for {pat} in {iter} {pass} {fail}"),
            _ => unimplemented!("ICE: malformed Bind expression {self}"),
        }
    }
}

impl Inference for Pat {
    fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
        match self {
            Self::Ignore => Ok(e.new_inferred()),
            Self::Never => Ok(e.never()),
            Self::MetId(_) => todo!("Macro identifiers in the type checker??!"),
            Self::Name(name) => Ok(e.by_name(name)?),
            Self::Value(body) => body.infer(e),
            Self::Op(op, pats) => infer_pat_op(*op, pats, e),
        }
    }
}

fn infer_pat_op(op: PatOp, pats: &[At<Pat>], e: &mut InferenceEngine<'_, '_>) -> IfResult {
    match (op, pats) {
        (PatOp::Pub, [body]) => body.infer(e),
        (PatOp::Mut, [body]) => body.infer(e),
        (PatOp::Ref, [body]) => {
            let r = body.infer(e)?;
            Ok(e.new_ref(r))
        }
        (PatOp::Ptr, [..]) => todo!("Pointer decomposition"),
        (PatOp::Rest, [..]) => todo!("Partial decomposition or start-open range"),
        (PatOp::RangeEx, [..]) => todo!("Range Exclusive"),
        (PatOp::RangeIn, [..]) => todo!("Range Inclusive"),
        (PatOp::Record, [..]) => todo!("Record decomposition"),
        (PatOp::Tuple, args) => {
            let args = args
                .iter()
                .map(|arg| arg.infer(e))
                .collect::<Result<_, _>>()?;
            Ok(e.new_tuple(args))
        }
        (PatOp::Slice, [body]) => {
            let r = body.infer(e)?;
            Ok(e.new_slice(r))
        }
        (PatOp::ArRep, [value, rep]) => todo!("Array Repetition `[{value}; {rep}]`"),
        (PatOp::Typed, [body, ty]) => todo!("Type Annotation `{body}: {ty}`"),
        (PatOp::TypePrefixed, [pfx, body]) => todo!("Prefix Annotation {pfx} {body}"),
        (PatOp::Generic, [name, rest @ ..]) => todo!("Generic usage `{name}<{rest:?}>`"),
        (PatOp::Fn, [..]) => todo!("Function introduction"),
        (PatOp::Alt, [..]) => todo!("Alternates"),
        _ => panic!(""),
    }
}

impl Inference for Make {
    fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
        let Self(ty, arms) = self;
        // todo!("infer {self}")
        let ty = ty.infer(e)?;
        for arm in arms {
            e.at(ty).infer(arm)?;
        }
        Ok(ty)
        // Look up struct definition in scope
        // generalize definition
        // unify members by name?
    }
}

impl Inference for MakeArm {
    fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
        let Self(sym, expr) = self;
        let ty = e.by_name(sym)?;
        let ty = e.deep_clone(ty);
        let expr = expr.infer(e)?;
        e.unify(ty, expr)?;
        Ok(e.unit())
    }
}

impl Inference for Match {
    fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
        let Self(scrutinee, arms) = self;
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

// use crate::table::Map;

// impl Inference for Generics {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         // bind names
//         for name in &self.vars {
//             let ty = e.new_var();
//             e.table.add_child(e.at, *name, ty);
//         }
//         Ok(e.unit())
//     }
// }

// impl Inference for Module {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Self { name, file } = self;
//         let Some(file) = file else {
//             return Err(InferenceError::NotFound((*name).into()));
//         };
//         let module = e.by_name(name)?;
//         e.at(module).infer(file)
//     }
// }

// impl Inference for Alias {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Self { name: _, from } = self;
//         // let this = e.by_name(name)?;
//         let alias = if let Some(from) = from {
//             TypeKind::Instance(e.infer(from)?)
//         } else {
//             TypeKind::Tuple(vec![])
//         };

//         // This node may be a lang item referring to a primitive.
//         let mut entry = e.at.to_entry_mut(e.table);
//         if entry.ty().is_some() {
//             return Ok(e.unit());
//         }
//         entry.set_ty(alias);

//         Ok(entry.id())
//     }
// }

// impl Inference for Function {
//     #[allow(unused)]
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Self { name, gens, sign, bind, body } = self;
//         // bind name to signature
//         let node = e.at; // e.by_name(name)?;
//         let node = e.deep_clone(node);
//         let fnty = e.by_name(sign)?;
//         e.unify(node, fnty)?;

//         // bind gens to new variables at function scope
//         let mut scope = e.at(node);
//         scope.infer(gens)?;

//         // bind binds to args
//         let pat = scope.infer(bind)?;
//         let arg = scope.by_name(sign.args.as_ref())?;
//         scope.unify(pat, arg);

//         let mut rset = None;

//         let mut retscope = scope.open_rset(&mut rset);

//         // infer body
//         let bodty = retscope.infer(body)?;
//         let rety = sign.rety.infer(&mut retscope)?;
//         // unify body with rety
//         retscope.unify(bodty, rety)?;
//         // unify rset with rety

//         if let Some(rset) = rset {
//             scope.unify(rset, rety)?;
//         }
//         Ok(node)
//     }
// }
// impl Inference for Closure {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Self { arg, body } = self;
//         let args = arg.infer(e)?;

//         let mut scope = e.block_scope();

//         let mut rset = None;
//         let mut retscope = scope.open_rset(&mut rset);
//         let rety = retscope.infer(body)?;

//         if let Some(rset) = rset {
//             e.unify(rety, rset)?;
//         }

//         Ok(e.table.anon_type(TypeKind::FnSig { args, rety }))
//     }
// }

// // TODO: do we need type inference/checking in struct definitions?
// // there are no bodies

// impl Inference for Enum {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Self { name: _, gens, variants } = self;
//         let node = e.at; //e.by_name(name)?;
//         let mut scope = e.at(node);

//         scope.infer(gens)?;
//         for variant in variants {
//             println!("Inferring {variant}");
//             let var_ty = scope.infer(variant)?;
//             scope.unify(node, var_ty)?;
//         }
//         Ok(node)
//     }
// }

// impl Inference for Variant {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Self { name, kind: _, body } = self;
//         let node = e.by_name(name)?;

//         // TODO: this doesn't work when some variants have bodies and some don't
//         if e.table.ty(node).is_some() {
//             println!("{node} has ty!");
//             return Ok(node);
//         }

//         match body {
//             Some(body) => {
//                 let mut e = e.at(node);
//                 let value = e.infer(body)?;
//                 e.unify(node, value)?;
//             }
//             _ => {
//                 e.table.entry_mut(node).set_ty(TypeKind::Inferred);
//             }
//         };

//         Ok(node)
//     }
// }

// impl Inference for Struct {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Self { name, gens, kind: _ } = self;
//         let node = e.by_name(name)?;
//         let mut e = e.at(node);
//         e.infer(gens)?;

//         Ok(node)
//     }
// }

// impl Inference for Impl {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Self { gens, target, body } = self;
//         // TODO: match gens to target gens
//         gens.infer(e)?;
//         let instance = target.infer(e)?;
//         let instance = e.def_usage(instance);
//         let mut scope = e.at(instance);
//         scope.infer(body)
//     }
// }

// impl Inference for Tuple {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Tuple { exprs } = self;
//         exprs
//             .iter()
//             // Infer each member
//             .map(|expr| expr.infer(e))
//             // Construct tuple
//             .collect::<Result<Vec<_>, InferenceError>>()
//             // Return tuple
//             .map(|tys| e.new_tuple(tys))
//     }
// }

// impl Inference for Structor {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Structor { to, init } = self;
//         // Evaluate the path in the current context
//         let to = to.infer(e)?;
//         match e.entry(to).ty() {
//             // Typecheck the fielders against the fields
//             Some(TypeKind::Adt(Adt::Struct(fields))) => {
//                 if init.len() != fields.len() {
//                     return Err(InferenceError::FieldCount(to, fields.len(), init.len()));
//                 }
//                 let fields = fields.clone(); // todo: fix this somehow.
//                 let mut field_inits: std::collections::Map<_, _> = init
//                     .iter()
//                     .map(|Fielder { name, init }| (name, init))
//                     .collect();
//                 // Unify fields with fielders
//                 for (name, _vis, ty) in fields {
//                     match field_inits.remove(&name) {
//                         Some(Some(field)) => {
//                             let init_ty = field.infer(e)?;
//                             e.unify(init_ty, ty)?;
//                         }
//                         Some(None) => {
//                             // Get name in scope
//                             let init_ty = e
//                                 .table
//                                 .get_by_sym(e.at, &name)
//                                 .ok_or_else(|| InferenceError::NotFound(Path::from(name)))?;
//                             e.unify(init_ty, ty)?;
//                         }
//                         None => Err(InferenceError::NotFound(Path::from(name)))?,
//                     }
//                 }
//                 Ok(to)
//             }
//             _ => Err(InferenceError::NotFound(self.to.clone())),
//         }
//     }
// }

// impl Inference for Array {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Array { values } = self;
//         let out = e.new_inferred();
//         for value in values {
//             let ty = value.infer(e)?;
//             e.unify(out, ty)?;
//         }
//         Ok(e.new_array(out, values.len()))
//     }
// }

// impl Inference for ArrayRep {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let ArrayRep { value, repeat } = self;
//         let ty = value.infer(e)?;
//         let rep = repeat.infer(e)?;
//         let usize_ty = e.usize();
//         e.unify(rep, usize_ty)?;
//         match &repeat.kind {
//             ExprKind::Literal(Literal::Int(repeat)) => Ok(e.new_array(ty, *repeat as usize)),
//             _ => {
//                 todo!("TODO: constant folding before type checking?");
//             }
//         }
//     }
// }

// impl Inference for AddrOf {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let AddrOf { mutable: _, expr } = self;
//         // TODO: mut ref
//         let ty = expr.infer(e)?;
//         Ok(e.new_ref(ty))
//     }
// }

// impl Inference for Quote {
//     fn infer(&self, _e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         todo!("Quote: {self}")
//     }
// }

// impl Inference for Literal {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let ty = match self {
//             Literal::Bool(_) => e.bool(),
//             Literal::Char(_) => e.char(),
//             Literal::Int(_) => e.integer_literal(),
//             Literal::Float(_) => e.float_literal(),
//             Literal::String(_) => {
//                 let str_ty = e.str();
//                 e.new_ref(str_ty)
//             }
//         };
//         Ok(e.new_inst(ty))
//     }
// }

// impl Inference for Group {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Group { expr } = self;
//         expr.infer(e)
//     }
// }

// impl Inference for Block {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Block { stmts } = self;
//         let mut e = e.block_scope();
//         let empty = e.unit();
//         if let [stmts @ .., ret] = stmts.as_slice() {
//             for stmt in stmts {
//                 match (&stmt.kind, &stmt.semi) {
//                     (StmtKind::Expr(expr), Semi::Terminated) => {
//                         expr.infer(&mut e)?;
//                     }
//                     (StmtKind::Expr(expr), Semi::Unterminated) => {
//                         let ty = expr.infer(&mut e)?;
//                         e.unify(ty, empty)?;
//                     }
//                     _ => {}
//                 }
//             }
//             let out = if let StmtKind::Expr(expr) = &ret.kind {
//                 expr.infer(&mut e)?
//             } else {
//                 empty
//             };
//             if Semi::Unterminated == ret.semi {
//                 return Ok(out);
//             }
//         }
//         Ok(empty)
//     }
// }

// impl Inference for Assign {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Assign { parts } = self;
//         let (head, tail) = parts.as_ref();
//         // Infer the tail expression
//         let tail = tail.infer(e)?;
//         // Infer the head expression
//         let head = head.infer(e)?;
//         // Unify head and tail
//         e.unify(head, tail)?;
//         // Return Empty
//         Ok(e.unit())
//     }
// }

// impl Inference for Modify {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Modify { kind: _, parts } = self;
//         let (head, tail) = parts.as_ref();
//         // Infer the tail expression
//         let tail = tail.infer(e)?;
//         // Infer the head expression
//         let head = head.infer(e)?;
//         // TODO: Search within the head type for `(op)_assign`
//         e.unify(head, tail)?;
//         // TODO: Typecheck `op_assign(&mut head, tail)`
//         Ok(e.unit())
//     }
// }

// impl Inference for Binary {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         use BinaryKind as Bk;
//         let Binary { kind, parts } = self;
//         let (head, tail) = parts.as_ref();
//         // Infer the tail expression
//         let tail = tail.infer(e)?;
//         // Infer the head expression
//         let head = head.infer(e)?;
//         let head = e.prune(head);
//         // TODO: Search within the head type for `(op)`
//         match kind {
//             BinaryKind::Call => match e.entry(head).ty() {
//                 Some(TypeKind::Adt(Adt::TupleStruct(types))) => {
//                     let Some(TypeKind::Tuple(values)) = e.entry(tail).ty() else {
//                         Err(InferenceError::Mismatch(head, tail))?
//                     };
//                     if types.len() != values.len() {
//                         Err(InferenceError::FieldCount(head, types.len(), values.len()))?
//                     }
//                     let pairs = types
//                         .iter()
//                         .zip(values.iter())
//                         .map(|(&(_vis, ty), &value)| (ty, value))
//                         .collect::<Vec<_>>();
//                     for (ty, value) in pairs {
//                         e.unify(ty, value)?;
//                     }
//                     Ok(head)
//                 }
//                 Some(&TypeKind::FnSig { args, rety }) => {
//                     e.unify(tail, args)?;
//                     Ok(rety)
//                 }
//                 _ => Err(InferenceError::Mismatch(head, tail))?,
//             },
//             Bk::Lt | Bk::LtEq | Bk::Equal | Bk::NotEq | Bk::GtEq | Bk::Gt => {
//                 e.unify(head, tail)?;
//                 Ok(e.bool())
//             }
//             Bk::LogAnd | Bk::LogOr | Bk::LogXor => {
//                 let bool = e.bool();
//                 e.unify(head, bool)?;
//                 e.unify(tail, bool)?;
//                 Ok(bool)
//             }
//             // TODO: Don't return the generic form wholesale.
//             Bk::RangeExc => Ok(e.table.get_lang_item("range_exc")),
//             Bk::RangeInc => Ok(e.table.get_lang_item("range_exc")),
//             Bk::Shl | Bk::Shr => {
//                 let shift_amount = e.u32();
//                 e.unify(tail, shift_amount)?;
//                 Ok(head)
//             }
//             Bk::BitAnd
//             | Bk::BitOr
//             | Bk::BitXor
//             | Bk::Add
//             | Bk::Sub
//             | Bk::Mul
//             | Bk::Div
//             | Bk::Rem => {
//                 // Typecheck op(head, tail)
//                 e.unify(head, tail)?;
//                 Ok(head)
//             }
//         }
//     }
// }

// impl Inference for Unary {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Unary { kind, tail } = self;
//         match kind {
//             UnaryKind::Deref => {
//                 let tail = tail.infer(e)?;
//                 let tail = e.def_usage(tail);
//                 // TODO: get the base type
//                 match e.entry(tail).ty() {
//                     Some(&TypeKind::Ref(h)) => Ok(h),
//                     _ => todo!("Deref {}", e.entry(tail)),
//                 }
//             }
//             UnaryKind::Loop => {
//                 let mut scope = e.block_scope();
//                 // Enter a new breakset
//                 let mut bset = None;
//                 let mut bscope = scope.open_bset(&mut bset);
//                 // Infer the tail
//                 let body = tail.infer(&mut bscope)?;
//                 // Unify the loop body with `empty`
//                 let unit = bscope.unit();
//                 bscope.unify(unit, body)?;

//                 // Return breakset
//                 match bset {
//                     Some(bset) => Ok(bset),
//                     None => Ok(e.never()),
//                 }
//             }
//             _op => {
//                 // Infer the tail expression
//                 let tail = tail.infer(e)?;
//                 // TODO: Search within the tail type for `(op)`
//                 Ok(tail)
//             }
//         }
//     }
// }

// impl Inference for Member {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Member { head, kind } = self;
//         // Infer the head expression
//         let head = head.infer(e)?;
//         // Get the type of head
//         let head = e.prune(head);
//         let ty = e.entry(head);
//         // Search within the head type for the memberkind
//         match kind {
//             MemberKind::Call(name, tuple) => {
//                 if let Some((args, rety)) = e.get_fn(ty.id(), *name) {
//                     let values = iter::once(Ok(e.new_ref(head)))
//                         // infer for each member-
//                         .chain(tuple.exprs.iter().map(|expr| expr.infer(e)))
//                         // Construct tuple
//                         .collect::<Result<Vec<_>, InferenceError>>()
//                         // Return tuple
//                         .map(|tys| e.new_tuple(tys))?;
//                     e.unify(args, values)?;
//                     Ok(rety)
//                 } else {
//                     Err(InferenceError::NotFound(Path::from(*name)))
//                 }
//             }
//             MemberKind::Struct(name) => {
//                 if let Some(TypeKind::Adt(Adt::Struct(members))) = ty.ty() {
//                     for member in members {
//                         if member.0 == *name {
//                             return Ok(member.2);
//                         }
//                     }
//                 };
//                 match ty.nav(&[PathPart::Ident(*name)]) {
//                     Some(ty) => Ok(ty.id()),
//                     None => Err(InferenceError::NotFound(Path::from(*name))),
//                 }
//             }
//             MemberKind::Tuple(Literal::Int(idx)) => match ty.ty() {
//                 Some(TypeKind::Tuple(tys)) => tys
//                     .get(*idx as usize)
//                     .copied()
//                     .ok_or(InferenceError::FieldCount(head, tys.len(), *idx as usize)),
//                 Some(TypeKind::Adt(Adt::TupleStruct(tys))) => tys
//                     .get(*idx as usize)
//                     .map(|(_vis, ty)| *ty)
//                     .ok_or(InferenceError::FieldCount(head, tys.len(), *idx as usize)),
//                 _ => Err(InferenceError::Mismatch(ty.id(), e.table.root())),
//             },
//             _ => Err(InferenceError::Mismatch(ty.id(), ty.root())),
//         }
//         // Type is required to be inferred at this point.
//     }
// }

// impl Inference for Index {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Index { head, indices } = self;
//         let usize = e.usize();
//         // Infer the head expression
//         let head = head.infer(e)?;
//         let mut head = e.prune(head);
//         // For each index expression:
//         for index in indices {
//             //   Infer the index type
//             let index = index.infer(e)?;
//             if let Some((args, rety)) = e.get_fn(head, "index".into()) {
//                 // Unify args and tuple (&head, index)
//                 let selfty = e.new_ref(head);
//                 let tupty = e.new_tuple(vec![selfty, index]);
//                 e.unify(args, tupty)?;
//                 head = e.prune(rety);
//                 continue;
//             }
//             //   Decide whether the head can be indexed by that type
//             //   TODO: check for a `.index` method on the type
//             match e.entry(head).ty().unwrap() {
//                 &TypeKind::Slice(handle) | &TypeKind::Array(handle, _) => {
//                     e.unify(usize, index)?;
//                     head = e.prune(handle);
//                 }
//                 other => todo!("Indexing on type {other}"),
//             }
//             //   head = result of indexing head
//         }
//         Ok(head)
//     }
// }

// impl Inference for Cast {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Cast { head, ty } = self;
//         // Infer the head expression
//         let _head = head.infer(e)?;
//         // Evaluate the type
//         let ty = ty
//             .evaluate(e.table, e.at)
//             .map_err(InferenceError::AnnotationEval)?;
//         // Decide whether the type is castable
//         // TODO: not deciding is absolutely unsound!!!
//         // Return the type
//         Ok(ty)
//     }
// }

// impl Inference for Path {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         e.by_name(self)
//             .map_err(|_| InferenceError::NotFound(self.clone()))
//     }
// }

// impl Inference for Let {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Let { mutable: _, name, ty, init } = self;
//         let ty = match ty {
//             Some(ty) => ty
//                 .evaluate(e.table, e.at)
//                 .map_err(InferenceError::AnnotationEval)?,
//             None => e.new_inferred(),
//         };
//         // Infer the initializer
//         if let Some(init) = init {
//             // Unify the initializer and the ty
//             let initty = init.infer(e)?;
//             e.unify(ty, initty)?;
//         }
//         // Deep copy the ty, if it exists
//         let ty = e.deep_clone(ty);
//         // Infer the pattern
//         let patty = name.infer(e)?;
//         // Unify the pattern and the ty
//         e.unify(ty, patty)?;
//         // `if let` returns whether the pattern succeeded or not
//         Ok(e.bool())
//     }
// }

// impl Inference for Match {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let Match { scrutinee, arms } = self;
//         // Infer the scrutinee
//         let scrutinee = scrutinee.infer(e)?;

//         let mut out = None;
//         // For each pattern:
//         for MatchArm(pat, expr) in arms {
//             let mut scope = e.block_scope();
//             // Infer the pattern
//             let pat = pat.infer(&mut scope)?;
//             // Unify it with the scrutinee
//             scope.unify(scrutinee, pat)?;
//             // Infer the Expr
//             let expr = expr.infer(&mut scope)?;
//             // Unify the expr with the out variable
//             match out {
//                 Some(ty) => e.unify(ty, expr)?,
//                 None => out = Some(expr),
//             }
//         }
//         // Return out. If there are no arms, assume Never.
//         match out {
//             Some(ty) => Ok(ty),
//             None => Ok(e.never()),
//         }
//     }
// }

// impl Inference for Pattern {
//     // TODO: This is the wrong way to typeck pattern matching.
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         match self {
//             Pattern::Name(name) => {
//                 // Evaluating a pattern creates and enters a new scope.
//                 // Surely this will cause zero problems.
//                 e.local_scope(*name);
//                 e.table.set_ty(e.at, TypeKind::Inferred);
//                 Ok(e.at)
//             }
//             Pattern::Path(path) => {
//                 // Evaluating a path pattern puts type constraints on the scrutinee
//                 path.evaluate(e.table, e.at)
//                     .map_err(|_| InferenceError::NotFound(path.clone()))
//             }
//             Pattern::Literal(literal) => literal.infer(e),
//             Pattern::Rest(Some(pat)) => {
//                 eprintln!("TODO: Rest patterns in tuples?");
//                 let ty = pat.infer(e)?;
//                 Ok(e.new_slice(ty))
//             }
//             Pattern::Rest(_) => Ok(e.new_inferred()),
//             Pattern::Ref(_, pattern) => {
//                 let ty = pattern.infer(e)?;
//                 Ok(e.new_ref(ty))
//             }
//             Pattern::RangeExc(pat1, pat2) => {
//                 let ty1 = pat1.infer(e)?;
//                 let ty2 = pat2.infer(e)?;
//                 e.unify(ty1, ty2)?;
//                 Ok(ty1)
//             }
//             Pattern::RangeInc(pat1, pat2) => {
//                 let ty1 = pat1.infer(e)?;
//                 let ty2 = pat2.infer(e)?;
//                 e.unify(ty1, ty2)?;
//                 Ok(ty1)
//             }
//             Pattern::Tuple(patterns) => {
//                 let tys = patterns
//                     .iter()
//                     .map(|pat| pat.infer(e))
//                     .collect::<Result<Vec<Handle>, InferenceError>>()?;
//                 Ok(e.new_tuple(tys))
//             }
//             Pattern::Array(patterns) => match patterns.as_slice() {
//                 // TODO: rest patterns here
//                 [one, rest @ ..] => {
//                     let ty = one.infer(e)?;
//                     for rest in rest {
//                         let ty2 = rest.infer(e)?;
//                         e.unify(ty, ty2)?;
//                     }
//                     Ok(e.new_slice(ty))
//                 }
//                 [] => {
//                     let ty = e.new_inferred();
//                     Ok(e.new_slice(ty))
//                 }
//             },
//             Pattern::Struct(_path, _items) => {
//                 eprintln!("TODO: struct patterns: {self}");
//                 Ok(e.unit())
//             }
//             Pattern::TupleStruct(path, patterns) => {
//                 eprintln!("TODO: tuple struct patterns: {self}");
//                 let struc = e.by_name(path)?;
//                 let Some(TypeKind::Adt(Adt::TupleStruct(ts))) = e.entry(struc).ty() else {
//                     Err(InferenceError::Mismatch(struc, e.never()))?
//                 };
//                 let ts: Vec<_> = ts.iter().map(|(_v, h)| *h).collect();
//                 let tys = patterns
//                     .iter()
//                     .map(|pat| pat.infer(e))
//                     .collect::<Result<Vec<Handle>, InferenceError>>()?;
//                 let ts = e.new_tuple(ts);
//                 let tup = e.new_tuple(tys);
//                 e.unify(ts, tup)?;
//                 Ok(struc)
//             }
//         }
//     }
// }

// impl Inference for While {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let While { cond, pass, fail } = self;
//         let mut bset = None;

//         // Open a block scope so the loop condition doesn't escape
//         {
//             let mut scope = e.block_scope();
//             // Infer the condition
//             let cond = cond.infer(&mut scope)?;
//             // Unify the condition with bool
//             let bool = scope.bool();
//             scope.unify(bool, cond)?;

//             // Open a breakset for the pass branch
//             {
//                 let mut body_scope = scope.open_bset(&mut bset);
//                 // Infer the pass branch
//                 let pass = pass.infer(&mut body_scope)?;
//                 // Unify the pass branch with Empty
//                 let empt = body_scope.unit();
//                 body_scope.unify(empt, pass)?;
//             }
//         }

//         // Infer the fail branch
//         let fail = fail.infer(e)?;

//         // Unify the fail branch with breakset
//         if let Some(bset) = bset {
//             println!("bset: {}", e.entry(bset));
//             e.unify(bset, fail)?;
//         }
//         Ok(fail)
//     }
// }

// impl Inference for If {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let If { cond, pass, fail } = self;

//         // Open a block scope so the condition doesn't escape
//         let pass = {
//             let mut scope = e.block_scope();
//             // Do inference on the condition'
//             let cond = cond.infer(&mut scope)?;
//             // Unify the condition with bool
//             let bool = scope.bool();
//             scope.unify(bool, cond)?;
//             // Do inference on the pass branch
//             pass.infer(&mut scope)?
//         };

//         // Do inference on the fail branch
//         let fail = fail.infer(&mut e.block_scope())?;

//         // Unify pass and fail
//         e.unify(pass, fail)?;
//         // Return the result
//         Ok(pass)
//     }
// }

// impl Inference for For {
//     fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
//         let For { bind, cond, pass, fail } = self;
//         let mut scope = e.block_scope();

//         let bind = bind.infer(&mut scope)?;

//         // What does it mean to be iterable? Why, `next()`, of course!
//         let cond = cond.infer(&mut scope)?;
//         let cond = scope.prune(cond);
//         if let Some((args, rety)) = scope.get_fn(cond, "next".into()) {
//             // Check that the args are correct
//             let params = vec![scope.new_ref(cond)];
//             let params = scope.new_tuple(params);
//             scope.unify(args, params)?;
//             scope.unify(rety, bind)?;
//         }

//         // Open a breakset
//         let mut bset = None;
//         let mut bscope = scope.open_bset(&mut bset);

//         // Infer the pass branch
//         let pass = pass.infer(&mut bscope)?;
//         // Unify the pass branch with Empty
//         let empt = bscope.unit();
//         bscope.unify(pass, empt)?;

//         // Infer the fail branch
//         let fail = fail.infer(&mut e.block_scope())?;

//         // Unify the fail branch with the breakset
//         if let Some(bset) = bset {
//             e.unify(fail, bset)?;
//         }
//         Ok(fail)
//     }
// }

impl<I: Inference> Inference for Option<I> {
    fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
        match self {
            Some(expr) => expr.infer(e),
            None => Ok(e.unit()),
        }
    }
}
impl<I: Inference> Inference for Box<I> {
    fn infer(&self, e: &mut InferenceEngine<'_, '_>) -> IfResult {
        self.as_ref().infer(e)
    }
}
