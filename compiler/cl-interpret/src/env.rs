//! Lexical & non-lexical [scoping](Environment) for variables, and [Backtrace] support

use crate::{
    builtin::Builtin,
    place::Place,
    typeinfo::{self, Model, Type},
};

use super::{
    Callable, Interpret,
    builtin::{Builtins, Math},
    convalue::ConValue,
    error::{Error, IResult},
    function::Function,
};
use cl_ast::{Bind as FnDecl, fmt::FmtAdapter, types::Symbol};
use cl_structures::{intern::interned::Interned, span::Span};
use std::{
    collections::HashMap,
    fmt::Display,
    mem::take,
    ops::{Deref, DerefMut},
    rc::Rc,
};

pub type StackFrame = HashMap<Symbol, ConValue>;

pub type StackBinds = HashMap<Symbol, usize>;

#[derive(Clone, Debug, Default)]
pub(crate) struct EnvFrame {
    pub name: Option<&'static str>,

    pub span: Option<Span>,
    /// The length of the array when this stack frame was constructed
    pub base: usize,
    /// The bindings of name to stack position
    pub binds: StackBinds,
    /// A list of deferred instructions to run on scope exit
    pub defer: Vec<cl_ast::Expr>,
}

#[derive(Clone, Copy, Debug)]
pub struct Backtrace<'env> {
    frames: &'env [EnvFrame],
}

impl std::fmt::Display for Backtrace<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut count = 0;
        for EnvFrame { name, span, .. } in self.frames.iter().rev() {
            if let (Some(name), Some(span)) = (name, span) {
                writeln!(f, "{count:>4}: {name}")?;
                count += 1;
            }
        }
        Ok(())
    }
}

/// Implements a nested lexical scope
#[derive(Clone, Debug)]
pub struct Environment {
    values: Vec<ConValue>,
    frames: Vec<EnvFrame>,
    types: HashMap<Symbol, Type>,
    pub(crate) impls: HashMap<typeinfo::Type, HashMap<Symbol, ConValue>>,
}

impl Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write as _;
        for EnvFrame { name, binds, .. } in self.frames.iter().rev() {
            writeln!(
                f,
                "--- {}[{}] ---",
                if let Some(name) = name { name } else { "" },
                binds.len(),
            )?;
            let mut binds: Vec<_> = binds.iter().collect();
            binds.sort_by_key(|(_, a)| *a);
            for (name, idx) in binds {
                let mut f = f.indent();
                writeln!(f, "{idx:4} {name}:")?;
                match self.values.get(*idx) {
                    Some(ConValue::TypeInfo(t)) => writeln!(f, "type {t}"),
                    Some(ConValue::Function(v)) => writeln!(f, "fn {}", v.decl().0),
                    Some(value) => writeln!(f, "{value}"),
                    None => writeln!(f, "ERROR: {name}'s address blows the stack!"),
                }?
            }
        }
        Ok(())
    }
}

impl Default for Environment {
    fn default() -> Self {
        let mut this = Self::no_builtins();
        for (ident, model) in Model::defaults() {
            let value = this.def_type(ident.into(), model.intern());
            this.bind(ident, ConValue::TypeInfo(value));
        }
        this.add_builtins(Builtins).add_builtins(Math);
        this
    }
}

impl Environment {
    pub fn new() -> Self {
        Self::default()
    }
    /// Creates an [Environment] with no [builtins](super::builtin)
    pub fn no_builtins() -> Self {
        Self {
            values: Vec::new(),
            frames: vec![EnvFrame::default()],
            types: HashMap::new(),
            impls: HashMap::new(),
        }
    }

    /// Reflexively evaluates a node
    pub fn eval(&mut self, node: &impl Interpret) -> IResult<ConValue> {
        node.interpret(self)
    }

    /// Calls a function inside the Environment's scope,
    /// and returns the result
    pub fn call(&mut self, name: Symbol, args: &[ConValue]) -> IResult<ConValue> {
        let function = self.get(name)?;
        function.call(self, args)
    }

    /// Defers an expression until the end of scope. The expression must not fail..?
    pub fn defer(&mut self, expr: cl_ast::Expr) -> Option<()> {
        let EnvFrame { name: _, span: _, base: _, binds: _, defer } = self.frames.last_mut()?;
        defer.push(expr);
        Some(())
    }

    /// Binds a value to the given name in the current scope.
    pub fn bind(&mut self, name: impl Into<Symbol>, value: impl Into<ConValue>) {
        self.insert(name.into(), value.into());
    }

    pub fn bind_raw(&mut self, name: Symbol, id: usize) -> Option<()> {
        let EnvFrame { name: _, span: _, base: _, binds, defer: _ } = self.frames.last_mut()?;
        binds.insert(name, id);
        Some(())
    }

    pub fn implement(&mut self, ty: Type, name: Symbol, value: ConValue) -> Option<ConValue> {
        self.impls.entry(ty).or_default().insert(name, value)
    }

    pub fn get_impl(&self, ty: Type, name: Symbol) -> IResult<ConValue> {
        let res = self.impls.get(&ty).and_then(|map| map.get(&name));
        Ok(res.ok_or(Error::NotDefined(name))?.clone())
    }

    /// Gets all registered globals, bound or unbound.
    pub(crate) fn globals(&self) -> &EnvFrame {
        self.frames.first().unwrap()
    }

    pub fn backtrace(&self) -> Backtrace<'_> {
        Backtrace { frames: &self.frames }
    }

    /// Adds builtins
    ///
    /// # Panics
    ///
    /// Will panic if stack contains more than the globals frame!
    pub fn add_builtins(&mut self, builtins: &'static [Builtin]) -> &mut Self {
        if self.frames.len() != 1 {
            panic!("Cannot add builtins to full stack: {self}")
        }

        for builtin in builtins {
            self.insert(
                builtin.name().expect("Builtin functions must have names!"),
                builtin.into(),
            );
        }

        self
    }

    /// Enters a nested scope, returning a [`Frame`] stack-guard.
    ///
    /// [`Frame`] implements Deref/DerefMut for [`Environment`].
    pub fn frame(&mut self, name: &'static str, span: Option<Span>) -> Frame<'_> {
        Frame::new(self, name, span)
    }

    /// Enters a nested scope, assigning the contents of `frame`,
    /// and returning a [`Frame`] stack-guard.
    ///
    /// [`Frame`] implements Deref/DerefMut for [`Environment`].
    pub fn with_frame<'e>(&'e mut self, name: &'static str, frame: StackFrame) -> Frame<'e> {
        let mut scope = self.frame(name, None);
        for (k, v) in frame {
            scope.insert(k, v);
        }
        scope
    }

    /// Resolves a variable immutably.
    ///
    /// Returns a reference to the variable's contents, if it is defined and initialized.
    pub fn get(&self, name: Symbol) -> IResult<ConValue> {
        let id = self.id_of(name)?;
        let res = self.values.get(id);
        Ok(res.ok_or(Error::NotDefined(name))?.clone())
    }

    /// Resolves the index associated with a [Symbol]
    pub fn id_of(&self, name: Symbol) -> IResult<usize> {
        for EnvFrame { binds, .. } in self.frames.iter().rev() {
            if let Some(id) = binds.get(&name).copied() {
                return Ok(id);
            }
        }
        Err(Error::NotDefined(name))
    }

    /// Returns a shared reference to the `id`'s record, if it exists.
    pub fn get_id(&self, id: usize) -> Option<&ConValue> {
        self.values.get(id)
    }

    /// Returns a mutable reference to the `id`'s record, if it exists.
    pub fn get_id_mut(&mut self, id: usize) -> Option<&mut ConValue> {
        self.values.get_mut(id)
    }

    pub fn def_type(&mut self, name: Symbol, ty: Type) -> Type {
        self.types.insert(name, ty);
        ty
    }

    pub fn get_type(&self, name: Symbol) -> Option<Type> {
        self.types.get(&name).copied()
    }

    /// Inserts a new [ConValue] into this [Environment]
    pub fn insert(&mut self, k: Symbol, v: ConValue) {
        if self.bind_raw(k, self.values.len()).is_some() {
            self.values.push(v);
        }
    }

    /// Allocates a local variable
    pub fn stack_alloc(&mut self, value: ConValue) -> IResult<usize> {
        let adr = self.values.len();
        self.values.push(value);
        Ok(adr)
    }
}

/// Represents a stack frame
#[derive(Debug)]
pub struct Frame<'scope> {
    scope: &'scope mut Environment,
}
impl<'scope> Frame<'scope> {
    fn new(scope: &'scope mut Environment, name: &'static str, span: Option<Span>) -> Self {
        scope.frames.push(EnvFrame {
            name: Some(name),
            span,
            base: scope.values.len(),
            binds: HashMap::new(),
            defer: vec![],
        });

        Self { scope }
    }

    pub fn pop_values(mut self) -> Option<StackFrame> {
        let mut out = HashMap::new();
        let binds = take(&mut self.frames.last_mut()?.binds);
        for (k, v) in binds {
            out.insert(k, self.values.get_mut(v).map(take)?);
        }
        Some(out)
    }
}
impl Deref for Frame<'_> {
    type Target = Environment;
    fn deref(&self) -> &Self::Target {
        self.scope
    }
}
impl DerefMut for Frame<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.scope
    }
}
impl Drop for Frame<'_> {
    fn drop(&mut self) {
        if let Some(EnvFrame { base, defer, .. }) = self.frames.last_mut() {
            let (base, deferred) = (*base, take(defer));
            for defer in deferred.iter().rev() {
                if let Err(e) = defer.interpret(self) {
                    println!("Error during scope cleanup: {e}")
                }
            }

            self.frames.pop();
            self.values.truncate(base);
        }
    }
}
