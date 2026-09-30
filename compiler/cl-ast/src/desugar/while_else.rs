//! Desugars `while {...} else` expressions
//! into `loop if {...} else break` expressions

use crate::{
    ast::*,
    fold::{Fold, Foldable, impl_default_fold},
};

/// Adds `'loop`, `'while`, and `'for` labels to their corresponding loops
pub struct LoopLabelDesugar<A: AstTypes> {
    /// The [AstTypes::Symbol] for a `loop`'s label
    pub label_loop: A::Symbol,
    /// The [AstTypes::Symbol] for a `while` loop's label
    pub label_while: A::Symbol,
    /// The [AstTypes::Symbol] for a `for` loop's label
    pub label_for: A::Symbol,
}

impl LoopLabelDesugar<DefaultTypes> {
    pub fn new() -> Self {
        LoopLabelDesugar {
            label_for: "for".into(),
            label_while: "while".into(),
            label_loop: "loop".into(),
        }
    }
}

impl<A: AstTypes> Fold<A, A> for LoopLabelDesugar<A> {
    type Error = A::Annotation;
    impl_default_fold!(A, A);

    fn fold_at_expr(&mut self, expr: At<Expr<A>, A>) -> Result<At<Expr<A>, A>, Self::Error> {
        let At(expr, span) = expr.children(self)?;
        let label = match &expr {
            Expr::Op(Op::Loop, _) => self.label_loop,
            Expr::Op(Op::While, _) => self.label_while,
            Expr::Bind(bind) if let Bind(BindOp::For, ..) = bind.as_ref() => self.label_for,
            _ => return Ok(expr.at(span)),
        };
        Ok(Expr::Label(Label(label, expr.at(span)).into()).at(span))
    }
}

/// Desugars while-else expressions
/// into loop-if-else-break expressions
pub struct WhileElseDesugar;

impl<A: AstTypes> Fold<A, A> for WhileElseDesugar {
    type Error = A::Annotation;
    impl_default_fold!(A, A);

    fn fold_at_expr(&mut self, expr: At<Expr<A>, A>) -> Result<At<Expr<A>, A>, Self::Error> {
        let expr = expr.children(self)?;
        let At(Expr::Op(Op::While, mut parts), span) = expr else {
            return Ok(expr);
        };
        if parts.len() != 3 {
            std::hint::cold_path();
            Err(span)?
        }
        let fail = parts.pop().unwrap();
        let pass = parts.pop().unwrap();
        let cond = parts.pop().unwrap();
        let fail_span = fail.1;
        let fail = Expr::Op(Op::Break, vec![fail]).at(fail_span);
        let body = Expr::Op(Op::If, vec![cond, pass, fail]).at(span);
        let expr = Expr::Op(Op::Loop, vec![body]).at(span);

        Ok(expr)
    }
}

/// Desugars `for-else` expressions into `loop if let` expressions.
///
/// To do this for arbitrary [`AstTypes`], you are required to provide:
/// - option_some: The [`AstTypes::Path`] to [`Option::Some`]
/// - option_none: The [`AstTypes::Path`] to [`Option::None`]
/// - into_iter: The [`AstTypes::Path`] to [`IntoIterator::into_iter`]
/// - next: The [`AstTypes::Path`] to [`Iterator::next`]
/// - object_symbol: The [`AstTypes::Symbol`] to store the result of `into_iter`
/// - object_path: The [`AstTypes::Path`] to retrieve the result of `into_iter`
///
/// ```rust,ignore
/// // input:
/// for Pat in Iter Pass else Fail
///
/// // output:
/// match Iter.into_iter() {
///     $iter => 'for loop
///         if let Some(Pat) = $iter.next()
///             Pass
///         else
///             break Fail
/// ```
pub struct ForElseDesugar<A: AstTypes> {
    /// The [`AstTypes::Path`] to [`Option::Some`]
    pub option_some: A::Path,
    /// The [`AstTypes::Path`] to [`Option::None`]
    pub option_none: A::Path,
    /// The [`AstTypes::Path`] to [`IntoIterator::into_iter`]
    pub into_iter: A::Path,
    /// The [`AstTypes::Path`] to [`Iterator::next`]
    pub next: A::Path,
    /// A(n) [`AstTypes::Symbol`] to store the result of `into_iter`
    pub object_symbol: A::Symbol,
    /// A(n) [`AstTypes::Path`] to retrieve the result of `into_iter`
    pub object_path: A::Path,
}

impl ForElseDesugar<DefaultTypes> {
    pub fn new() -> Self {
        let object_symbol: types::Symbol = "$iter".into();
        ForElseDesugar {
            option_some: types::Path { parts: vec!["Option".into(), "Some".into()] },
            option_none: types::Path { parts: vec!["Option".into(), "None".into()] },
            into_iter: types::Path { parts: vec!["into_iter".into()] },
            next: types::Path { parts: vec!["next".into()] },
            object_symbol,
            object_path: types::Path { parts: vec![object_symbol] },
        }
    }
}

impl<A: AstTypes> Fold<A, A> for ForElseDesugar<A> {
    type Error = A::Annotation;
    impl_default_fold!(A, A);

    fn fold_at_expr(&mut self, expr: At<Expr<A>, A>) -> Result<At<Expr<A>, A>, Self::Error> {
        let expr = expr.children(self)?;
        let At(Expr::Bind(bind), span) = expr else {
            return Ok(expr);
        };
        let Bind(BindOp::For, pat, mut parts) = *bind else {
            return Ok(Expr::Bind(bind).at(span));
        };
        if parts.len() != 3 {
            std::hint::cold_path();
            return Err(span);
        }
        let fail = parts.pop().unwrap();
        let pass = parts.pop().unwrap();
        let iter = parts.pop().unwrap();
        let pat_span = pat.1;
        let fail_span = fail.1;
        let iter_span = iter.1;

        // into_iter := iter.into_iter()
        let into_iter = Expr::Id::<A>(self.into_iter.clone()).at(iter_span);
        let iter_args = Expr::Op(Op::Tuple, vec![]).at(iter_span);
        let call_iter = Expr::Op(Op::Call, vec![into_iter, iter_args]).at(iter_span);
        let into_iter = Expr::Op(Op::Dot, vec![iter, call_iter]).at(iter_span);

        // iter_next := $iter.next()
        let iter = Expr::Id::<A>(self.object_path.clone()).at(iter_span);
        let next = Expr::Id::<A>(self.next.clone()).at(iter_span);
        let next_args = Expr::Op(Op::Tuple, vec![]).at(iter_span);
        let call_next = Expr::Op(Op::Call, vec![next, next_args]).at(iter_span);
        let iter_next = Expr::Op(Op::Dot, vec![iter, call_next]).at(iter_span);

        // some_pat := Some(pat)
        let some = Expr::Id::<A>(self.option_some.clone()).at(pat_span);
        let some = Pat::Value(Box::new(some)).at(pat_span);
        let some_pat = Pat::Op(PatOp::Tuple, vec![pat]).at(pat_span);
        let some_pat = Pat::Op(PatOp::TypePrefixed, vec![some, some_pat]).at(pat_span);

        // cond := let Some(pat) = $iter.next()
        let cond = Bind(BindOp::Let, some_pat, vec![iter_next]);
        let cond = Expr::Bind(cond.into()).at(span);

        // fail := break fail
        let fail = Expr::Op(Op::Break, vec![fail]).at(fail_span);

        // body := if let Some(pat) = $iter.next() Pass else break Fail
        let body = Expr::Op(Op::If, vec![cond, pass, fail]).at(span);
        // body := loop (if let Some(pat) = $iter.next() Pass else break Fail)
        let body = Expr::Op(Op::Loop, vec![body]).at(span);

        // mtch := match iter.into_iter() { $iter => (loop if let ...); }
        let iter_pat = Pat::Name::<A>(self.object_symbol).at(iter_span);
        let mtch = Match(into_iter, vec![MatchArm(iter_pat, body)]);

        Ok(Expr::Match(mtch.into()).at(span))
    }
}
