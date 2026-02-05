//! Lexical and non-lexical scoping for variables

use crate::{builtin::Builtin, constructor::Constructor};

use super::{
    Callable, Interpret,
    builtin::{Builtins, Math},
    convalue::ConValue,
    error::{Error, IResult},
    function::Function,
};
use cl_ast::{Bind as FnDecl, types::Symbol};
use cl_structures::span::Span;
use std::{
    collections::HashMap,
    fmt::Display,
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
}

impl Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
                write!(f, "{idx:4} {name}: ")?;
                match self.values.get(*idx) {
                    Some(value) => writeln!(f, "\t{value}"),
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
        Self { values: Vec::new(), frames: vec![EnvFrame::default()] }
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

    /// Binds a value to the given name in the current scope.
    pub fn bind(&mut self, name: impl Into<Symbol>, value: impl Into<ConValue>) {
        self.insert(name.into(), value.into());
    }

    pub fn bind_raw(&mut self, name: Symbol, id: usize) -> Option<()> {
        let EnvFrame { name: _, span: _, base: _, binds } = self.frames.last_mut()?;
        binds.insert(name, id);
        Some(())
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

    pub fn push_frame(&mut self, name: &'static str, frame: StackFrame) {
        self.frames.push(EnvFrame {
            name: Some(name),
            span: None,
            base: self.values.len(),
            binds: HashMap::new(),
        });
        for (k, v) in frame {
            self.insert(k, v);
        }
    }

    pub fn pop_frame(&mut self) -> Option<(StackFrame, &'static str)> {
        let mut out = HashMap::new();
        let EnvFrame { name, span: _, base, binds } = self.frames.pop()?;
        for (k, v) in binds {
            out.insert(k, self.values.get_mut(v).map(std::mem::take)?);
        }
        self.values.truncate(base);
        Some((out, name.unwrap_or("")))
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

    /// Resolves a variable mutably.
    ///
    /// Returns a mutable reference to the variable's record, if it exists.
    pub fn get_mut(&mut self, name: Symbol) -> IResult<&mut ConValue> {
        let at = self.id_of(name)?;
        self.get_id_mut(at).ok_or(Error::NotDefined(name))
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

    pub fn get_id(&self, id: usize) -> Option<&ConValue> {
        self.values.get(id)
    }

    pub fn get_id_mut(&mut self, id: usize) -> Option<&mut ConValue> {
        self.values.get_mut(id)
    }

    pub fn get_slice(&self, start: usize, len: usize) -> Option<&[ConValue]> {
        self.values.get(start..start + len)
    }

    pub fn get_slice_mut(&mut self, start: usize, len: usize) -> Option<&mut [ConValue]> {
        self.values.get_mut(start..start + len)
    }

    /// Inserts a new [ConValue] into this [Environment]
    pub fn insert(&mut self, k: Symbol, v: ConValue) {
        if self.bind_raw(k, self.values.len()).is_some() {
            self.values.push(v);
        }
    }

    pub fn insert_tup_constructor(&mut self, name: Symbol, arity: usize) {
        let cs = Constructor { arity: arity as _, name };
        self.insert(name, ConValue::TupleConstructor(cs));
    }

    /// Gets the current stack top position
    pub fn pos(&self) -> usize {
        self.values.len()
    }

    /// Allocates a local variable
    pub fn stack_alloc(&mut self, value: ConValue) -> IResult<usize> {
        let adr = self.values.len();
        self.values.push(value);
        Ok(adr)
    }

    /// Allocates some space on the stack
    pub fn alloca(&mut self, value: ConValue, len: usize) -> ConValue {
        let idx = self.values.len();
        self.values.extend(std::iter::repeat_n(value, len));
        ConValue::Slice(idx, len)
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
        });

        Self { scope }
    }

    pub fn pop_values(mut self) -> Option<StackFrame> {
        let mut out = HashMap::new();
        let binds = std::mem::take(&mut self.frames.last_mut()?.binds);
        for (k, v) in binds {
            out.insert(k, self.values.get_mut(v).map(std::mem::take)?);
        }
        Some(out)
    }

    pub fn into_binds(mut self) -> Option<StackBinds> {
        let EnvFrame { name: _, span: _, base: _, binds } = self.frames.pop()?;
        std::mem::forget(self);
        Some(binds)
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
        if let Some(frame) = self.frames.pop() {
            self.values.truncate(frame.base);
        }
    }
}
