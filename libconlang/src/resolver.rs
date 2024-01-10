//! Extremely early WIP of a static type-checker/resolver
//!
//! This will hopefully become a fully fledged static resolution pass in the future
use std::collections::HashMap;

use crate::ast::preamble::*;

use scopeguard::Scoped;
pub mod scopeguard {
    //! Implements a generic RAII scope-guard

    use std::ops::{Deref, DerefMut};

    pub trait Scoped: Sized {
        fn frame(&mut self) -> Guard<Self> {
            Guard::new(self)
        }
        ///
        fn enter_scope(&mut self);
        fn exit_scope(&mut self);
    }

    pub struct Guard<'scope, T: Scoped> {
        inner: &'scope mut T,
    }
    impl<'scope, T: Scoped> Guard<'scope, T> {
        pub fn new(inner: &'scope mut T) -> Self {
            inner.enter_scope();
            Self { inner }
        }
    }
    impl<'scope, T: Scoped> Deref for Guard<'scope, T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            self.inner
        }
    }
    impl<'scope, T: Scoped> DerefMut for Guard<'scope, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.inner
        }
    }
    impl<'scope, T: Scoped> Drop for Guard<'scope, T> {
        fn drop(&mut self) {
            self.inner.exit_scope()
        }
    }
}

/// Prints like [println] if `debug_assertions` are enabled
macro debugln($($t:tt)*) {
    if cfg!(debug_assertions) {
        println!($($t)*);
    }
}
macro debug($($t:tt)*) {
    if cfg!(debug_assertions) {
        print!($($t)*);
    }
}

use ty::Type;
pub mod ty {
    //! Describes the type of a [Variable](super::Variable)
    use std::fmt::Display;

    /// Describes the type of a [Variable](super::Variable)
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
    pub enum Type {
        #[default]
        Empty,
        Int,
        Bool,
        Char,
        String,
        Float,
        Fn {
            args: Vec<Type>,
            ret: Box<Type>,
        },
        Range(Box<Type>),
        Tuple(Vec<Type>),
        Never,
        /// [Inferred](Type::Inferred) is for error messages. DO NOT CONSTRUCT
        Inferred,
        Generic(String),
        /// A function with a single parameter of [Type::ManyInferred]
        /// is assumed to always be correct.
        ManyInferred,
    }
    impl Type {
        fn is_empty(&self) -> bool {
            self == &Type::Empty
        }
    }
    impl Display for Type {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Type::Empty => "Empty".fmt(f),
                Type::Int => "integer".fmt(f),
                Type::Bool => "bool".fmt(f),
                Type::Char => "char".fmt(f),
                Type::String => "String".fmt(f),
                Type::Float => "float".fmt(f),
                // TODO: clean this up
                Type::Fn { args, ret } => {
                    "fn (".fmt(f)?;
                    let mut args = args.iter();
                    if let Some(arg) = args.next() {
                        arg.fmt(f)?;
                    }
                    for arg in args {
                        write!(f, ", {arg}")?
                    }
                    ")".fmt(f)?;
                    if !ret.is_empty() {
                        write!(f, " -> {ret}")?;
                    }
                    Ok(())
                }
                Type::Range(t) => write!(f, "{t}..{t}"),
                Type::Tuple(t) => {
                    "(".fmt(f)?;
                    for (idx, ty) in t.iter().enumerate() {
                        if idx > 0 {
                            ", ".fmt(f)?;
                        }
                        ty.fmt(f)?;
                    }
                    ")".fmt(f)
                }
                Type::Never => "!".fmt(f),
                Type::Inferred => "_".fmt(f),
                Type::Generic(name) => write!(f, "<{name}>"),
                Type::ManyInferred => "..".fmt(f),
            }
        }
    }
}

/// Describes the life-cycle of a [Variable]: Whether it's bound, typed, or initialized
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    #[default]
    Bound,
    Uninitialized(Type),
    Initialized(Type),
}
impl Status {
    /// Performs type-checking for a [Variable] assignment
    pub fn assign(&mut self, ty: &Type) -> TyResult<()> {
        match self {
            // Variable uninitialized: initialize it
            Status::Bound => {
                *self = Status::Initialized(ty.clone());
                Ok(())
            }
            // Typecheck ok! Reuse the allocation for t
            Status::Uninitialized(t) if t == ty => {
                *self = Status::Initialized(std::mem::take(t));
                Ok(())
            }
            Status::Initialized(t) if t == ty => Ok(()),
            // Typecheck not ok.
            Status::Uninitialized(e) | Status::Initialized(e) => {
                Err(Error::TypeMismatch { want: ty.clone(), got: e.clone() })
            }
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Variable {
    /// The unique, global index of this variable
    pub index: usize,
    /// The [Status] of this variable
    pub status: Status,
    /// The mutability qualifier of this variable
    pub mutable: bool,
}
impl Variable {
    /// Constructs a new variable with the provided index and mutability
    pub fn new(index: usize, mutable: bool) -> Self {
        Self { index, mutable, ..Default::default() }
    }
    /// Performs a type-checked assignment on self
    pub fn assign(&mut self, name: &str, ty: &Type) -> TyResult<()> {
        let Variable { index, status, mutable } = self;
        debug!("Typecheck for {name} #{index}: ");
        let out = match (status, mutable) {
            // Variable uninitialized: initialize it
            (Status::Bound, _) => {
                self.status = Status::Initialized(ty.clone());
                Ok(())
            }
            // Typecheck ok! Reuse the allocation for t
            (Status::Uninitialized(t), _) if t == ty => {
                self.status = Status::Initialized(std::mem::take(t));
                Ok(())
            }
            // Reassignment of mutable variable is ok
            (Status::Initialized(t), true) if t == ty => Ok(()),
            // Reassignment of immutable variable is not ok
            (Status::Initialized(_), false) => Err(Error::ImmutableAssign(name.into(), *index)),
            // Typecheck not ok.
            (Status::Uninitialized(e) | Status::Initialized(e), _) => {
                Err(Error::TypeMismatch { want: ty.clone(), got: e.clone() })
            }
        };
        match &out {
            Ok(_) => debugln!("Ok! ({ty})"),
            Err(e) => debugln!("Error: {e:?}"),
        }
        out
    }
    /// Performs the type-checking for a modifying assignment
    pub fn modify_assign(&self, name: &str, ty: &Type) -> TyResult<()> {
        let Variable { index, status, mutable } = &self;
        match (status, mutable) {
            (Status::Initialized(t), true) if t == ty => Ok(()),
            (Status::Initialized(t), true) => {
                Err(Error::TypeMismatch { want: t.clone(), got: ty.clone() })
            }
            (Status::Initialized(_), false) => Err(Error::ImmutableAssign(name.into(), *index)),
            (..) => Err(Error::Uninitialized(name.into(), *index)),
        }
    }
}

/*
    THE BIG IDEA:
    - Each `let` statement binds a *different* variable.
      - Shadowing is a FEATURE
    - Traversing the tree before program launch allows the Resolver to assign
      an index to each variable usage in the scope-tree
    - These indices allow constant-time variable lookup in the interpreter!!!
    - The added type-checking means fewer type errors!

    REQUIREMENTS FOR FULL TYPE-CHECKING:
    - Meaningful type expressions in function declarations

    NECESSARY CONSIDERATIONS:
    - Variable binding happens AFTER the initialization expression is run.
      - If a variable is *entirely* unbound before being referenced,
        it'll still error.
      - This is *intentional*, and ALLOWS shadowing previous variables.
      - In my experience, this is almost NEVER an error :P
*/

#[derive(Clone, Debug, Default)]
pub struct Scope {
    /// A monotonically increasing counter of variable declarations
    count: usize,
    /// A dictionary keeping track of type and lifetime information
    vars: HashMap<String, Variable>,
}
impl Scope {
    /// Bind a [Variable] in Scope
    pub fn insert(&mut self, name: &str, index: usize, mutable: bool) {
        self.count += 1;
        self.vars
            .insert(name.to_string(), Variable::new(index, mutable));
    }
    /// Returns a reference to a [Variable], if `name` is bound
    pub fn get(&self, name: &str) -> Option<&Variable> {
        self.vars.get(name)
    }
    /// Returns a mutable reference to a [Variable], if `name` is bound
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Variable> {
        self.vars.get_mut(name)
    }
}
/// Implements a dynamically scoped namespace
#[derive(Clone, Debug, Default)]
pub struct Module {
    modules: HashMap<String, Module>,
    vars: HashMap<String, Variable>,
}
impl Module {
    pub fn insert_var(&mut self, name: &str, index: usize, mutable: bool) -> TyResult<()> {
        if self
            .vars
            .insert(name.into(), Variable::new(index, mutable))
            .is_some()
        {
            Err(Error::NonUniqueInModule(name.into()))?;
        }
        Ok(())
    }
    pub fn insert_module(&mut self, name: String, module: Module) -> TyResult<()> {
        if self.modules.insert(name.clone(), module).is_some() {
            Err(Error::NonUniqueInModule(name + "(module)"))?
        }
        Ok(())
    }
    /// Returns a reference to a [Variable] in this Module, if `name` is bound
    pub fn get(&self, name: &str) -> Option<&Variable> {
        self.vars.get(name)
    }
    /// Returns a mutable reference to a [Variable] in this Module, if `name` is bound
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Variable> {
        self.vars.get_mut(name)
    }

    pub fn resolve_get(&self, name: &str, path: &[String]) -> Option<&Variable> {
        if path.is_empty() {
            return self.get(name);
        }
        let module = self.modules.get(&path[0])?;
        module
            .resolve_get(name, &path[1..])
            .or_else(|| self.get(name))
    }
    // Returns a reference to the module at a specified path
    pub fn resolve(&self, path: &[String]) -> TyResult<&Module> {
        if path.is_empty() {
            return Ok(self);
        }
        let module = self
            .modules
            .get(&path[0])
            .ok_or_else(|| Error::Unbound(path[0].clone()))?;
        module.resolve(&path[1..])
    }
    /// Returns a mutable reference to a Module if one is bound
    pub fn resolve_mut(&mut self, path: &[String]) -> TyResult<&mut Module> {
        if path.is_empty() {
            return Ok(self);
        }
        let module = self
            .modules
            .get_mut(&path[0])
            .ok_or_else(|| Error::Unbound(path[0].clone()))?;
        module.resolve_mut(&path[1..])
    }
}

#[derive(Clone, Debug)]
pub struct Resolver {
    /// A monotonically increasing counter of variable declarations
    count: usize,
    /// A stack of nested scopes *inside* a function
    scopes: Vec<Scope>,
    /// A stack of nested scopes *outside* a function
    // TODO: Record the name of the module, and keep a stack of the current module
    // for name resolution
    modules: Module,
    /// Describes the current path
    module: Vec<String>,
    /// A stack of types
    types: Vec<Type>,
}

impl Scoped for Resolver {
    fn enter_scope(&mut self) {
        self.enter_scope();
    }
    fn exit_scope(&mut self) {
        self.exit_scope();
    }
}

impl Default for Resolver {
    fn default() -> Self {
        let mut new = Self {
            count: Default::default(),
            scopes: vec![Default::default()],
            modules: Default::default(),
            module: Default::default(),
            types: Default::default(),
        };
        new.register_builtin("print", &[], &[Type::ManyInferred], Type::Empty)
            .expect("print should not be bound in new Resolver");
        new
    }
}

impl Resolver {
    pub fn new() -> Self {
        Default::default()
    }
    /// Register a built-in function into the top-level module
    pub fn register_builtin(
        &mut self,
        name: &str,
        path: &[String],
        args: &[Type],
        ret: Type,
    ) -> TyResult<()> {
        let module = self.modules.resolve_mut(path)?;
        module.vars.insert(
            name.into(),
            Variable {
                index: 0,
                status: Status::Initialized(Type::Fn { args: args.into(), ret: ret.into() }),
                mutable: false,
            },
        );
        Ok(())
    }
    /// Enters a Module Scope
    pub fn enter_module(&mut self, name: &str) -> TyResult<()> {
        let module = self.modules.resolve_mut(&self.module)?;
        module.insert_module(name.into(), Default::default())?;
        self.module.push(name.into());
        Ok(())
    }
    /// Exits a Module Scope
    pub fn exit_module(&mut self) -> Option<String> {
        // Modules stay registered
        self.module.pop()
    }
    /// Enters a Block Scope
    pub fn enter_scope(&mut self) {
        self.scopes.push(Default::default());
    }
    /// Exits a Block Scope, returning the value
    pub fn exit_scope(&mut self) -> Option<usize> {
        self.scopes.pop().map(|scope| scope.count)
    }
    /// Resolves a name to a [Variable]
    pub fn get(&self, name: &str) -> TyResult<&Variable> {
        if let Some(var) = self.scopes.iter().rev().find_map(|s| s.get(name)) {
            return Ok(var);
        }
        self.modules
            .resolve_get(name, &self.module)
            .ok_or_else(|| Error::Unbound(name.into()))
    }
    /// Mutably resolves a name to a [Variable]
    pub fn get_mut(&mut self, name: &str) -> TyResult<&mut Variable> {
        if let Some(var) = self.scopes.iter_mut().rev().find_map(|s| s.get_mut(name)) {
            return Ok(var);
        }
        self.modules
            .resolve_mut(&self.module)?
            .get_mut(name)
            .ok_or_else(|| Error::Unbound(name.into()))
    }
    /// Binds a name in the current lexical scope
    pub fn insert_scope(&mut self, name: &str, mutable: bool) -> TyResult<usize> {
        self.count += 1;
        self.scopes
            .last_mut()
            .ok_or_else(|| panic!("Stack underflow in resolver?"))?
            .insert(name, self.count, mutable);
        Ok(self.count)
    }
    /// Binds a name in the current module
    pub fn insert_module(&mut self, name: &str, mutable: bool) -> TyResult<usize> {
        self.count += 1;
        self.modules
            .resolve_mut(&self.module)?
            .insert_var(name, self.count, mutable)?;
        Ok(self.count)
    }
    /// Performs scoped type-checking of variables
    pub fn assign(&mut self, name: &str, ty: &Type) -> TyResult<()> {
        self.get_mut(name)?.assign(name, ty)
    }
}
/// Manages a module scope
/// ```rust,ignore
/// macro module(self, name: &str, inner: {...}) -> Result<_, Error>
/// ```
macro module($self:ident, $name:tt, $inner:tt) {{
    $self.enter_module($name)?;
    #[allow(clippy::redundant_closure_call)]
    let scope = (|| $inner)(); // This is pretty gross, but hey, try {} syntax is unstable too
    $self.exit_module();
    scope
}}

impl Resolver {
    pub fn visit_empty(&mut self) -> TyResult<()> {
        debugln!("Got Empty");
        self.types.push(Type::Empty);
        Ok(())
    }
}

pub trait Resolve {
    /// Performs variable resolution on self, and returns the type of self.
    ///
    /// For expressions, this is the type of the expression.
    ///
    /// For declarations, this is Empty.
    fn resolve(&mut self, _resolver: &mut Resolver) -> TyResult<Type> {
        Ok(Type::Empty)
    }
}

impl Resolve for Start {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        let Self(program) = self;
        program.resolve(resolver)
    }
}
impl Resolve for Program {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        let Self(module) = self;
        for decl in module {
            decl.resolve(resolver)?;
        }
        // TODO: record the number of module-level assignments into the AST
        Ok(Type::Empty)
    }
}
impl Resolve for Stmt {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        match self {
            Stmt::Let(value) => value.resolve(resolver),
            Stmt::Fn(value) => value.resolve(resolver),
            Stmt::Expr(value) => value.resolve(resolver),
        }
    }
}
impl Resolve for Let {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        let Let { name: Name { symbol: Identifier { name, index }, mutable, ty: _ }, init } = self;
        debugln!("ty> let {name} ...");
        if let Some(init) = init {
            let ty = init.resolve(resolver)?;
            *index = Some(resolver.insert_scope(name, *mutable)?);
            resolver.get_mut(name)?.assign(name, &ty)?;
        } else {
            resolver.insert_scope(name, *mutable)?;
        }
        Ok(Type::Empty)
    }
}
impl Resolve for FnDecl {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        let FnDecl { name: Name { symbol: Identifier { name, index }, .. }, args, body } = self;
        debugln!("ty> fn {name} ...");
        // register the name at module scope
        *index = Some(resolver.insert_module(name, false)?);
        // create a new lexical scope
        let scopes = std::mem::take(&mut resolver.scopes);
        // type-check the function body
        let out = {
            let mut resolver = resolver.frame();
            let mut evaluated_args = vec![];
            for arg in args {
                evaluated_args.push(arg.resolve(&mut resolver)?)
            }
            let fn_decl = Type::Fn { args: evaluated_args.clone(), ret: Box::new(Type::Empty) };
            resolver.get_mut(name)?.assign(name, &fn_decl)?;
            module!(resolver, name, { body.resolve(&mut resolver) })
        };
        let _ = std::mem::replace(&mut resolver.scopes, scopes);
        out
    }
}
impl Resolve for Name {
    fn resolve(&mut self, _resolver: &mut Resolver) -> TyResult<Type> {
        Ok(Type::Empty)
    }
}
impl Resolve for Block {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        let Block { let_count: _, statements, expr } = self;
        let mut resolver = resolver.frame();
        for stmt in statements {
            stmt.resolve(&mut resolver)?;
        }
        expr.resolve(&mut resolver)
    }
}
impl Resolve for Expr {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        let Expr(expr) = self;
        expr.resolve(resolver)
    }
}

impl Resolve for Operation {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        match self {
            Operation::Assign(value) => value.resolve(resolver),
            Operation::Binary(value) => value.resolve(resolver),
            Operation::Unary(value) => value.resolve(resolver),
            Operation::Call(value) => value.resolve(resolver),
        }
    }
}
impl Resolve for Assign {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        let Assign { target, operator, init } = self;
        // Evaluate the initializer expression
        let ty = init.resolve(resolver)?;
        // Resolve the variable
        match (operator, resolver.get_mut(&target.name)?) {
            (
                operator::Assign::Assign,
                Variable { status: Status::Initialized(_), mutable: false, index },
            ) => Err(Error::ImmutableAssign(target.name.clone(), *index)),
            // TODO: make typing more expressive for modifying assignment
            (_, variable) => variable
                .modify_assign(&target.name, &ty)
                .map(|_| Type::Empty),
        }
    }
}

impl Resolve for Binary {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        let Binary { first, other } = self;

        let mut first = first.resolve(resolver)?;
        for (op, other) in other {
            let other = other.resolve(resolver)?;
            first = resolver.resolve_binary_operator(first, other, op)?;
        }
        Ok(first)
    }
}
impl Resolve for Unary {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        let Unary { operators, operand } = self;
        let mut operand = operand.resolve(resolver)?;
        for op in operators {
            operand = resolver.resolve_unary_operator(operand, op)?;
        }
        Ok(operand)
    }
}
/// Resolve [operator]s
impl Resolver {
    fn resolve_binary_operator(
        &mut self,
        first: Type,
        other: Type,
        op: &operator::Binary,
    ) -> TyResult<Type> {
        // TODO: check type compatibility for binary ops
        // TODO: desugar binary ops into function calls, when member functions are a thing
        eprintln!("Resolve binary operators {first} {op:?} {other}");
        if first != other {
            Err(Error::TypeMismatch { want: first, got: other })
        } else {
            Ok(first)
        }
    }
    fn resolve_unary_operator(&mut self, operand: Type, op: &operator::Unary) -> TyResult<Type> {
        // TODO: Allow more expressive unary operator type conversions
        todo!("Resolve unary operators {op:?} {operand}")
    }
}
impl Resolve for Call {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        match self {
            Call::FnCall(value) => value.resolve(resolver),
            Call::Primary(value) => value.resolve(resolver),
        }
    }
}
impl Resolve for FnCall {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        let FnCall { callee, args } = self;
        let mut callee = callee.resolve(resolver)?;
        for argset in args {
            // arguments should always be a tuple here
            let arguments = argset.resolve(resolver)?;
            let Type::Tuple(arguments) = arguments else {
                Err(Error::TypeMismatch {
                    want: Type::Tuple(vec![Type::ManyInferred]),
                    got: arguments,
                })?
            };
            // Verify that the callee is a function, and the arguments match.
            // We need the arguments
            let Type::Fn { args, ret } = callee else {
                return Err(Error::TypeMismatch {
                    want: Type::Fn { args: arguments, ret: Type::Inferred.into() },
                    got: callee,
                })?;
            };
            for (want, got) in args.iter().zip(&arguments) {
                // TODO: verify generics
                if let Type::Generic(_) = want {
                    continue;
                }
                if want != got {
                    return Err(Error::TypeMismatch {
                        want: Type::Fn { args: arguments, ret: Type::Inferred.into() },
                        got: Type::Fn { args, ret },
                    })?;
                }
            }
            callee = *ret;
        }
        Ok(callee)
    }
}
impl Resolve for Primary {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        match self {
            Primary::Identifier(value) => value.resolve(resolver),
            Primary::Literal(value) => value.resolve(resolver),
            Primary::Block(value) => value.resolve(resolver),
            Primary::Group(value) => value.resolve(resolver),
            Primary::Branch(value) => value.resolve(resolver),
        }
    }
}

impl Resolve for Group {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        match self {
            Group::Tuple(tuple) => tuple.resolve(resolver),
            Group::Single(expr) => expr.resolve(resolver),
            Group::Empty => Ok(Type::Empty),
        }
    }
}

impl Resolve for Tuple {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        let Tuple { elements } = self;
        let mut types = vec![];
        for expr in elements.iter_mut() {
            types.push(expr.resolve(resolver)?);
        }
        Ok(Type::Tuple(types))
    }
}

impl Resolve for Identifier {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        let Identifier { name, index: id_index } = self;
        let Variable { index, status, .. } = resolver.get(name)?;
        *id_index = Some(*index);
        let ty = match status {
            Status::Initialized(t) => t,
            _ => Err(Error::Uninitialized(name.to_owned(), *index))?,
        };
        debugln!("ty> Resolved {} #{index}: {ty}", name);
        Ok(ty.to_owned())
    }
}
impl Resolve for Literal {
    fn resolve(&mut self, _resolver: &mut Resolver) -> TyResult<Type> {
        Ok(match self {
            Literal::String(_) => Type::String,
            Literal::Char(_) => Type::Char,
            Literal::Bool(_) => Type::Bool,
            Literal::Float(_) => Type::Float,
            Literal::Int(_) => Type::Int,
        })
    }
}

impl Resolve for Flow {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        // TODO: Finish this
        match self {
            Flow::While(value) => value.resolve(resolver),
            Flow::If(value) => value.resolve(resolver),
            Flow::For(value) => value.resolve(resolver),
            Flow::Continue(value) => value.resolve(resolver),
            Flow::Return(value) => value.resolve(resolver),
            Flow::Break(value) => value.resolve(resolver),
        }
    }
}
impl Resolve for While {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        // TODO: Finish this
        // Visit else first, save that to a break-pattern stack in the Resolver,
        // and check it inside Break::resolve()
        let While { cond, body, else_ } = self;
        cond.resolve(resolver)?; // must be Type::Bool
        body.resolve(resolver)?; // discard
        else_.resolve(resolver) // compare with returns inside body
    }
}
impl Resolve for If {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        let If { cond, body, else_ } = self;
        let cond = cond.resolve(resolver)?;
        if Type::Bool != cond {
            return Err(Error::TypeMismatch { want: Type::Bool, got: cond });
        }
        let body_ty = body.resolve(resolver)?;
        let else_ty = else_.resolve(resolver)?;
        if body_ty == else_ty {
            Ok(body_ty)
        } else {
            Err(Error::TypeMismatch { want: body_ty, got: else_ty })
        }
    }
}

impl Resolve for For {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        let For { var: Identifier { name, index }, iter, body, else_ } = self;
        debugln!("> for {name} in ...");
        // Visit the iter expression and get its type
        let range = iter.resolve(resolver)?;
        let ty = match range {
            Type::Range(t) => t,
            got => Err(Error::TypeMismatch { want: Type::Range(Type::Inferred.into()), got })?,
        };
        let body_ty = {
            let mut resolver = resolver.frame();
            // bind the variable in the loop scope
            *index = Some(resolver.insert_scope(name, false)?);
            resolver.get_mut(name)?.assign(name, &ty)?;
            body.resolve(&mut resolver)
        }?;
        // visit the else block
        let else_ty = else_.resolve(resolver)?;
        if body_ty != else_ty {
            Err(Error::TypeMismatch { want: body_ty, got: else_ty })
        } else {
            Ok(body_ty)
        }
    }
}
impl Resolve for Else {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        let Else { expr } = self;
        expr.resolve(resolver)
    }
}

impl Resolve for Continue {
    fn resolve(&mut self, _resolver: &mut Resolver) -> TyResult<Type> {
        // TODO: Finish control flow
        Ok(Type::Never)
    }
}
impl Resolve for Break {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        // TODO: Finish control flow
        let Break { expr } = self;
        expr.resolve(resolver)
    }
}
impl Resolve for Return {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        // TODO: Finish control flow
        let Return { expr } = self;
        expr.resolve(resolver)
    }
}
// heakc yea man, generics
impl<T: Resolve> Resolve for Option<T> {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        match self {
            Some(t) => t.resolve(resolver),
            None => Ok(Type::Empty),
        }
    }
}
impl<T: Resolve> Resolve for Box<T> {
    fn resolve(&mut self, resolver: &mut Resolver) -> TyResult<Type> {
        self.as_mut().resolve(resolver)
    }
}

use error::{Error, TyResult};
pub mod error {
    use super::Type;
    use std::fmt::Display;

    pub type TyResult<T> = Result<T, Error>;

    impl std::error::Error for Error {}
    #[derive(Clone, Debug)]
    pub enum Error {
        StackUnderflow,
        // types
        TypeMismatch { want: Type, got: Type },
        // modules
        NonUniqueInModule(String),
        // lifetimes
        Uninitialized(String, usize),
        ImmutableAssign(String, usize),
        Unbound(String),
    }
    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::StackUnderflow => "Stack underflow in Resolver".fmt(f),
                Error::TypeMismatch { want, got } => {
                    write!(f, "Type error: {want} != {got}")
                }
                Error::ImmutableAssign(name, index) => {
                    write!(f, "Cannot mutate immutable variable {name}(#{index})")
                }
                Error::Uninitialized(name, index) => {
                    write!(f, "{name}(#{index}) was accessed before initialization")
                }
                Error::Unbound(name) => write!(f, "{name} not bound before use."),
                Error::NonUniqueInModule(name) => {
                    write!(f, "Name {name} not unique at module scope!")
                }
            }
        }
    }
}
