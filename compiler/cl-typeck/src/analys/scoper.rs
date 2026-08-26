//! Calculates Conlang's scoping rules

use super::*;

/// Calculates the language's scoping rules
/// based on a recursive traversal
#[derive(Debug)]
pub struct Scoper<'t> {
    table: &'t mut Scopes,
    stack: Vec<ScopeIndex>,
    state: ScoperState,
}

use ScoperState::*;
#[derive(Clone, Copy, Debug)]
pub enum ScoperState {
    /// Binding arbitrary names
    Binding,
    /// Outside an enum declaration, searching for TypePrefixed
    ///
    /// Transitions to OutsideStruct when one is found
    OutsideEnum,
    /// Outside a struct declaration, searching for TypePrefixed
    ///
    /// Transitions to InsideRecord or InsideTuple when one is found
    OutsideStruct,
    /// Inside a struct, but we don't know what kind yet
    InsideStruct,
    /// Inside a record declaration, searching for Typed
    InsideRecord,
    /// Inside a tuple declaration
    InsideTuple,

    /// Outside a function declaration, looking for TypePrefixed, two-arg Fn, or Tuple
    OutsideFunction,
    /// Inside a function declaration, looking for Typed arguments
    InsideFunction,
    /// Ignoring all subpatterns
    Ignoring,
}

/**
 * [Scopes] and (not) Names
 *
 * Properties of [Scope]:
 * - contains names
 * - encloses [Expr]essions and [Pat]terns
 * - encloses other [Scopes]
 *
 * Properties of Name:
 * - can be const, static, or local
 * - can be pub or priv
 * - can be mut or not
 */
impl<'t> Scoper<'t> {
    /// Constructs a new Sniper
    pub fn new(table: &'t mut Scopes) -> Self {
        Self { stack: vec![table.root_scope()], table, state: Binding }
    }

    pub fn index(&self) -> ScopeIndex {
        match self.stack[..] {
            [] => panic!("Should not exit last scope!"),
            [.., last] => last,
        }
    }

    pub fn get(&self) -> &Scope {
        let scope = self.index();
        &self.table.scopes[scope]
    }

    pub fn get_mut(&mut self) -> &mut Scope {
        let scope = self.index();
        &mut self.table.scopes[scope]
    }

    /// Enters a new scope
    pub fn enter(&mut self, scope_kind: ScopeKind, from: &'static str) -> &mut Self {
        let parent = self.index();
        let idx = self.table.add_scope(parent, scope_kind, from);
        self.stack.push(idx);
        self
    }

    /// Exits the closest non-[`let`] [Scope].
    ///
    /// [Inherited][`let`] scopes, which bind [`let`]-bound
    /// variables, cannot be individually exited, and will
    /// accumulate until the end of their surrounding scope.
    ///
    /// [`let`]: ScopeKind::Inherited
    pub fn exit(&mut self) -> &mut Self {
        use ScopeKind::*;
        while self.table.scopes[self.stack.pop().expect("enclosing scope")].is(Inherited) {}
        self
    }

    /// Binds a symbol in the current scope
    ///
    /// Returns whether the symbol is unique in the scope
    pub fn bind(&mut self, name: Symbol) -> bool {
        let scope = self.index();
        let Scopes { scopes, names } = self.table;
        if scopes[scope].bindings.contains_key(&name) {
            return false;
        }
        let bound = names.len();
        names.push(name);
        scopes[scope].bindings.insert(name, bound).is_some()
    }

    /// Imports a path in the current scope
    ///
    /// Returns the existing import, if one already existed
    pub fn import(&mut self, name: Symbol, path: Path) -> Option<Path> {
        self.get_mut().imports.insert(name, path)
    }

    /// Glob-imports a path in the current scope
    ///
    /// Returns whether the import is unique in the scope
    pub fn glob(&mut self, path: Path) -> bool {
        self.get_mut().globs.insert(path)
    }

    /// Calls the closure on `self`
    pub fn then<T>(&mut self, block: impl FnOnce(&mut Scoper<'_>) -> T) -> T {
        block(self)
    }

    /// Opens a new "block scope" with the given [ScopeKind]
    pub fn block<R>(
        &mut self,
        kind: ScopeKind,
        from: &'static str,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let out = f(self.enter(kind, from));
        if kind != ScopeKind::Inherited {
            self.exit();
        }
        out
    }
}

impl<'t> fold::Fold<DefaultTypes, ScopedAst> for Scoper<'t> {
    type Error = ();

    impl_fold! {
        in Fold<DefaultTypes, ScopedAst>
        fn fold_annotation(self, span: Annotation) =
            ScopedSpan { span, scope: self.index() };
        fn fold_macro_id(self, from: MacroId) = from;
        fn fold_symbol(self, from: Symbol) = from;
        fn fold_path(self, from: Path) = from;
        fn fold_literal(self, from: Literal) = from;
    }

    fn fold_at_expr(
        &mut self,
        expr: At<Expr<DefaultTypes>>,
    ) -> Result<At<Expr<ScopedAst>, ScopedAst>, ()> {
        let At(expr, span) = expr;
        let span = self.fold_annotation(span)?;
        let expr = self.fold(expr)?;
        Ok(At(expr, span))
    }

    fn fold_expr(&mut self, expr: Expr<DefaultTypes>) -> Result<Expr<ScopedAst>, Self::Error> {
        match expr {
            Expr::Op(Op::Block, exprs) if !self.get().is(ScopeKind::Outer) => {
                self.block(ScopeKind::Inner, "block", |block| {
                    Ok(Expr::Op(Op::Block, block.fold(exprs)?))
                })
            }
            Expr::Op(op @ (Op::Loop | Op::Defer | Op::Break | Op::Return), exprs) => {
                self.block(ScopeKind::Inner, "control-flow", |block| {
                    Ok(Expr::Op(op, block.fold(exprs)?))
                })
            }
            Expr::Op(op @ (Op::If | Op::While), mut exprs) => {
                assert_eq!(exprs.len(), 3);
                let [cond, pass, fail] = exprs.into_chunks().pop().ok_or(())?;
                let (cond, pass) = self.block(ScopeKind::Inner, "cond-pass", |block| {
                    let cond = block.fold(cond)?;
                    let pass = block.fold(pass)?;
                    Ok((cond, pass))
                })?;
                let fail = self.block(ScopeKind::Inner, "else", |block| {
                    let fail = block.fold(fail)?;
                    Ok(fail)
                })?;
                Ok(Expr::Op(op, vec![cond, pass, fail]))
            }
            other => other.children(self),
        }
    }

    fn fold_bind(&mut self, bind: Bind<DefaultTypes>) -> Result<Bind<ScopedAst>, Self::Error> {
        use ScopeKind::*;
        let Bind(op, pat, mut exprs) = bind;
        match op {
            BindOp::Let => {
                let exprs = exprs
                    .into_iter()
                    .map(|e| self.block(Inner, "let body", |block| block.fold(e)))
                    .collect::<Result<_, _>>()?;
                let bind = |scope: &mut Scoper| {
                    let pat = scope.fold(pat)?;
                    Ok(Bind(BindOp::Let, pat, exprs))
                };
                // if outside body, bind in scope
                match self.get().kind {
                    Outer => bind(self),
                    _ => self.block(Inherited, "let", bind),
                }
            }
            // TODO: bind function names outside
            BindOp::Fn => self.block(Outer, "fn", |item| {
                let pat = item.fold(pat)?;
                let exprs = item.block(Inner, "fn body", |body| body.fold(exprs))?;
                Ok(Bind(op, pat, exprs))
            }),
            BindOp::Mod => Ok(Bind(
                BindOp::Mod,
                self.fold(pat)?,
                self.block(Outer, "mod", |block| block.fold(exprs))?,
            )),
            BindOp::Enum => {
                let pat = Binder::new(self, BinderState::OutsideEnum).fold(pat)?;
                let exprs = self.block(Inner, "enum body?", |block| block.fold(exprs))?;
                Ok(Bind(BindOp::Enum, pat, exprs))
            }
            BindOp::Struct => {
                let pat = Binder::new(self, BinderState::OutsideStruct).fold(pat)?;
                let exprs = self.block(Inner, "enum body?", |block| block.fold(exprs))?;
                Ok(Bind(BindOp::Enum, pat, exprs))
            }
            op @ (BindOp::Type) => self.block(Outer, "type", |block| {
                let pat = block.fold(pat)?;
                let exprs = block.block(Inner, "type body", |block| block.fold(exprs))?;
                Ok(Bind(op, pat, exprs))
            }),
            BindOp::Impl => todo!("Scope `impl`"),
            BindOp::For => {
                assert_eq!(exprs.len(), 3);
                let [cond, pass, fail] = exprs.into_chunks().pop().ok_or(())?;
                let cond = self.block(Inner, "iter", |block| block.fold(cond))?;
                let fail = self.block(Inner, "fail", |block| block.fold(fail))?;
                let (pat, pass) = self.block(Inner, "pass", |block| {
                    Ok((block.fold(pat)?, block.fold(pass)?))
                })?;
                let exprs = vec![cond, pass, fail];
                Ok(Bind(BindOp::For, pat, exprs))
            }
        }
    }

    fn fold_pat(&mut self, pat: Pat<DefaultTypes>) -> Result<Pat<ScopedAst>, Self::Error> {
        use ScopeKind::*;
        // FIXME: This doesn't fully encapsulate scoping semantics
        match pat {
            Pat::Name(name) => {
                self.bind(name);
                Ok(Pat::Name(name))
            }
            Pat::Op(PatOp::TypePrefixed, mut ats) if self.get().is(Outer) => {
                assert_eq!(ats.len(), 2);
                let [pfx, ty] = ats.into_chunks().pop().expect("should contain 2 patterns");
                let pfx = pfx.fold_in(self)?;
                let ty = self.block(Inherited, "args", |block| ty.fold_in(block))?;
                Ok(Pat::Op(PatOp::TypePrefixed, vec![pfx, ty]))
            }
            _ => pat.children(self),
        }
    }

    fn fold_make(&mut self, make: Make<DefaultTypes>) -> Result<Make<ScopedAst>, Self::Error> {
        self.block(ScopeKind::Inner, "make", |block| make.children(block))
    }

    fn fold_match(&mut self, mtch: Match<DefaultTypes>) -> Result<Match<ScopedAst>, Self::Error> {
        // The scrutinee and arms of a match expression exist in a shared scope
        self.block(ScopeKind::Inner, "match", |block| mtch.children(block))
    }

    fn fold_matcharm(
        &mut self,
        arm: MatchArm<DefaultTypes>,
    ) -> Result<MatchArm<ScopedAst>, Self::Error> {
        // Each match arm is wrapped in an implicit {block},
        self.block(ScopeKind::Inner, "match arm", |block| arm.children(block))
    }
}

macro_rules! fold_with {
    (
        with $self: ident = $body: expr;
        $(fn $f: ident ($ty: ty) -> $rety: ty;)*
    ) => {$(
        fn $f(&mut $self, param: $ty) -> Result<$rety, Self::Error> {
            $body.$f(param)
        }
    )*};
}

#[derive(Clone, Copy, Debug)]
pub enum BinderState {
    /// Outside an enum declaration, searching for TypePrefixed
    ///
    /// Transitions to OutsideStruct when one is found
    OutsideEnum,
    /// Outside a struct declaration, searching for TypePrefixed
    ///
    /// Transitions to InsideRecord or InsideTuple when one is found
    OutsideStruct,
    /// Inside a struct, but we don't know what kind yet
    InsideStruct,
    /// Inside a record declaration, searching for Typed
    InsideRecord,
    /// Inside a tuple declaration
    InsideTuple,

    /// Outside a function declaration, looking for TypePrefixed, two-arg Fn, or Tuple
    OutsideFunction,
    /// Inside a function declaration, looking for Typed arguments
    InsideFunction,
    /// Ignoring all subpatterns
    Ignoring,
}

#[derive(Debug)]
pub struct Binder<'p, 't> {
    scoper: &'p mut Scoper<'t>,
    state: BinderState,
}

impl<'p, 't> Binder<'p, 't> {
    pub fn new(scoper: &'p mut Scoper<'t>, state: BinderState) -> Self {
        Self { scoper, state }
    }
    pub fn then<T>(&mut self, block: impl FnOnce(&mut Binder<'_, '_>) -> T) -> T {
        block(self)
    }
    pub fn state<'a>(&'a mut self, state: BinderState) -> Binder<'a, 't> {
        Binder { scoper: self.scoper, state }
    }
    pub fn block<'f, T: 'f>(
        &mut self,
        state: BinderState,
        kind: ScopeKind,
        from: &'static str,
        block: impl FnOnce(&mut Binder<'_, '_>) -> T,
    ) -> T {
        {
            self.scoper.enter(kind, from);
            let out = block(&mut Binder::new(self.scoper, state));
            if kind != ScopeKind::Inherited {
                self.scoper.exit();
            }
            out
        }
    }
}

impl fold::Fold<DefaultTypes, ScopedAst> for Binder<'_, '_> {
    type Error = ();

    impl_fold! {
        in Fold<DefaultTypes, ScopedAst>
        fn fold_annotation(self, span: Annotation) =
            self.scoper.fold_annotation(span)?;
        fn fold_macro_id(self, from: MacroId) = from;
        fn fold_symbol(self, from: Symbol) = from;
        fn fold_path(self, from: Path) = from;
        fn fold_literal(self, from: Literal) = from;
    }

    fold_with! {
        with self = self.scoper;
        fn fold_at_expr(At<Expr>) -> At<Expr<ScopedAst>, ScopedAst>;
        fn fold_expr(Expr) -> Expr<ScopedAst>;
        fn fold_label(Label) -> Label<ScopedAst>;
        fn fold_use(Use) -> Use<ScopedAst>;
        fn fold_bind(Bind) -> Bind<ScopedAst>;
        fn fold_make(Make) -> Make<ScopedAst>;
        fn fold_makearm(MakeArm) -> MakeArm<ScopedAst>;
        fn fold_match(Match) -> Match<ScopedAst>;
        fn fold_matcharm(MatchArm) -> MatchArm<ScopedAst>;
        // We want to traverse nested patterns!
        // ! fn fold_at_pat(At<Pat>) -> At<Pat<ScopedAst>, ScopedAst>;
    }

    fn fold_pat(&mut self, pat: Pat) -> Result<Pat<ScopedAst>, Self::Error> {
        use BinderState::*;
        use ScopeKind::*;

        match (self.state, pat) {
            (OutsideEnum, Pat::Op(PatOp::TypePrefixed, pats)) if pats.len() == 2 => {
                // found enum definition
                let [name, ty] = pats.into_chunks().pop().expect("should contain 2 patterns");
                println!("Found enum definition {name}");
                // fold outer in parent
                let pfx = self.fold(name)?;
                // fold inner in new scope, with state OutsideStruct
                let ty = self.block(OutsideStruct, Inner, "enum", |block| block.fold(ty))?;
                Ok(Pat::Op(PatOp::TypePrefixed, vec![pfx, ty]))
            }
            // found enum definition
            (OutsideStruct, Pat::Op(PatOp::TypePrefixed, pats)) if pats.len() == 2 => {
                let [name, ty] = pats.into_chunks().pop().expect("should contain 2 patterns");
                println!("Found struct definition {name}");
                // fold outer in parent
                let pfx = self.fold(name)?;
                // fold inner in new scope, with state InsideStruct
                let ty = self.block(InsideStruct, Inner, "struct", |block| block.fold(ty))?;
                Ok(Pat::Op(PatOp::TypePrefixed, vec![pfx, ty]))
            }
            (InsideStruct, Pat::Op(PatOp::Record, pats)) => {
                let pats = self.block(InsideRecord, Inner, "members", |block| block.fold(pats))?;
                Ok(Pat::Op(PatOp::Record, pats))
            }
            (InsideStruct, Pat::Op(PatOp::Tuple, pats)) => {
                let pats = self.block(InsideTuple, Inner, "tuple", |block| block.fold(pats))?;
                Ok(Pat::Op(PatOp::Tuple, pats))
            }
            // found record field
            (InsideRecord | InsideFunction, Pat::Op(PatOp::Typed, pats)) if pats.len() == 2 => {
                let [name, ty] = pats.into_chunks().pop().expect("should contain 2 patterns");
                println!("Found record or function argument member {name} /*: {ty} */");
                let name = self.fold(name)?;
                let ty = self.state(Ignoring).fold(ty)?;
                Ok(Pat::Op(PatOp::Typed, vec![name, ty]))
            }
            (InsideTuple, pat) => self.state(Ignoring).fold(pat),
            // found named function declaration
            (OutsideFunction, Pat::Op(PatOp::TypePrefixed, pats)) if pats.len() == 2 => {
                let [name, ty] = pats.into_chunks().pop().expect("should contain 2 patterns");
                println!("Found function definition {name}");
                let name = self.fold(name)?;
                let ty = self.block(InsideFunction, Inherited, "args", |block| block.fold(ty))?;
                Ok(Pat::Op(PatOp::TypePrefixed, vec![name, ty]))
            }
            // found anonymous function declaration
            (OutsideFunction | InsideFunction, Pat::Op(PatOp::Fn, pats)) if pats.len() == 2 => {
                let [args, rety] = pats.into_chunks().pop().expect("should contain 2 patterns");
                println!("Found function arguments {args} /* -> {rety} */");
                let args = self.block(InsideFunction, Inherited, "args", |b| b.fold(args))?;
                let rety = self.state(Ignoring).fold(rety)?;
                Ok(Pat::Op(PatOp::TypePrefixed, vec![args, rety]))
            }
            (Ignoring, pat) => pat.children(self),
            (_, Pat::Name(name)) => {
                self.scoper.bind(name);
                Ok(Pat::Name(self.fold_symbol(name)?))
            }
            (_, pat) => pat.children(self),
        }

        // match pat {
        //     Pat::Op(PatOp::Generic, _) => {
        //         self.scoper.enter(Inherited, "generics");
        //         pat.children(self)
        //     }
        //     Pat::Op(PatOp::TypePrefixed, mut ats) => {
        //         assert_eq!(ats.len(), 2);
        //         let [pfx, ty] = ats.into_chunks().pop().expect("should contain 2 patterns");
        //         let pfx = pfx.fold_in(self)?;
        //         let ty = self
        //             .scoper
        //             .block(Inherited, "args", |block| ty.fold_in(block))?;
        //         Ok(Pat::Op(PatOp::TypePrefixed, vec![pfx, ty]))
        //     }
        //     other => other.children(self),
        // }
    }
}
