//! Lexical and non-lexical scoping for variables

use crate::builtin::Builtin;

use super::{
    Callable, Interpret,
    builtin::{Builtins, Math},
    convalue::ConValue,
    error::{Error, IResult},
    function::Function,
};
use cl_ast::{Function as FnDecl, Sym};
use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Deref, DerefMut},
    rc::Rc,
};

type StackFrame = HashMap<Sym, Option<ConValue>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    Global(Sym),
    Local(usize),
}

impl Display for Place {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Place::Global(name) => name.fmt(f),
            Place::Local(id) => id.fmt(f),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct EnvFrame {
    /// The length of the array when this stack frame was constructed
    pub name: Option<&'static str>,
    pub base: usize,
    pub binds: HashMap<Sym, usize>,
}

/// Implements a nested lexical scope
#[derive(Clone, Debug)]
pub struct Environment {
    global: HashMap<Sym, Option<ConValue>>,
    values: Vec<Option<ConValue>>,
    frames: Vec<EnvFrame>,
}

impl Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for EnvFrame { name, base: _, binds } in self.frames.iter().rev() {
            writeln!(
                f,
                "--- {} ---",
                if let Some(name) = name { name } else { "" }
            )?;
            for (var, val) in binds {
                write!(f, "{var}: ")?;
                match self.values.get(*val) {
                    Some(Some(value)) => writeln!(f, "\t{value}"),
                    Some(None) => writeln!(f, "<undefined>"),
                    None => writeln!(f, "ERROR: {var} address blows the stack!"),
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
        Self { values: Vec::new(), global: HashMap::new(), frames: vec![] }
    }

    /// Reflexively evaluates a node
    pub fn eval(&mut self, node: &impl Interpret) -> IResult<ConValue> {
        node.interpret(self)
    }

    /// Calls a function inside the interpreter's scope,
    /// and returns the result
    pub fn call(&mut self, name: Sym, args: &[ConValue]) -> IResult<ConValue> {
        let function = self.get(name)?;
        function.call(self, args)
    }

    /// Adds builtins
    ///
    /// # Panics
    ///
    /// Will panic if globals table is non-empty!
    pub fn add_builtins(&mut self, builtins: &'static [Builtin]) -> &mut Self {
        let Self { global, .. } = self;
        for builtin in builtins {
            global.insert(builtin.name(), Some(builtin.into()));
        }
        self
    }

    pub fn push_frame(&mut self, name: &'static str, frame: StackFrame) {
        self.enter(name);
        for (k, v) in frame {
            self.insert(k, v);
        }
    }

    pub fn pop_frame(&mut self) -> Option<(StackFrame, &'static str)> {
        let mut out = HashMap::new();
        let EnvFrame { name, base, binds } = self.frames.pop()?;
        for (k, v) in binds {
            out.insert(k, self.values.get_mut(v).and_then(std::mem::take));
        }
        self.values.truncate(base);
        Some((out, name.unwrap_or("")))
    }

    /// Enters a nested scope, returning a [`Frame`] stack-guard.
    ///
    /// [`Frame`] implements Deref/DerefMut for [`Environment`].
    pub fn frame(&mut self, name: &'static str) -> Frame {
        Frame::new(self, name)
    }

    /// Resolves a variable mutably.
    ///
    /// Returns a mutable reference to the variable's record, if it exists.
    pub fn get_mut(&mut self, name: Sym) -> IResult<&mut Option<ConValue>> {
        let at = self.id_of(name)?;
        self.get_id_mut(at).ok_or(Error::NotDefined(name))
    }

    /// Resolves a variable immutably.
    ///
    /// Returns a reference to the variable's contents, if it is defined and initialized.
    pub fn get(&self, name: Sym) -> IResult<ConValue> {
        let id = self.id_of(name)?;
        let res = match id {
            Place::Global(name) => self.global.get(&name),
            Place::Local(id) => self.values.get(id),
        };
        match res.ok_or(Error::NotDefined(name))? {
            Some(value) => Ok(value.clone()),
            None => Err(Error::NotInitialized(name)),
        }
    }

    /// Resolves the [Place] associated with a [Sym]
    pub fn id_of(&self, name: Sym) -> IResult<Place> {
        for EnvFrame { binds, .. } in self.frames.iter().rev() {
            if let Some(id) = binds.get(&name).copied() {
                return Ok(Place::Local(id));
            }
        }
        Ok(Place::Global(name))
    }

    pub fn get_id(&self, at: Place) -> Option<&ConValue> {
        let res = match at {
            Place::Global(name) => self.global.get(&name),
            Place::Local(id) => self.values.get(id),
        }?;
        res.as_ref()
    }

    pub fn get_id_mut(&mut self, at: Place) -> Option<&mut Option<ConValue>> {
        match at {
            Place::Global(name) => self.global.get_mut(&name),
            Place::Local(id) => self.values.get_mut(id),
        }
    }

    /// Inserts a new [ConValue] into this [Environment]
    pub fn insert(&mut self, k: Sym, v: Option<ConValue>) {
        if self.bind_raw(k, self.values.len()).is_some() {
            self.values.push(v);
        } else {
            self.global.insert(k, v);
        }
    }

    /// A convenience function for registering a [FnDecl] as a [Function]
    pub fn insert_fn(&mut self, decl: &FnDecl) {
        let FnDecl { name, .. } = decl;
        let (name, function) = (*name, Rc::new(Function::new(decl)));
        self.insert(name, Some(ConValue::Function(function.clone())));
        // Tell the function to lift its upvars now, after it's been declared
        function.lift_upvars(self);
    }

    /// Allocates a local variable
    pub fn stack_alloc(&mut self, value: ConValue) -> IResult<usize> {
        let adr = self.values.len();
        self.values.push(Some(value));
        Ok(adr)
    }

    pub fn bind_raw(&mut self, name: Sym, id: usize) -> Option<()> {
        let EnvFrame { name: _, base: _, binds } = self.frames.last_mut()?;
        binds.insert(name, id);
        Some(())
    }
}

/// Functions which aid in the implementation of [`Frame`]
impl Environment {
    /// Enters a scope, creating a new namespace for variables
    fn enter(&mut self, name: &'static str) -> &mut Self {
        let new_frame =
            EnvFrame { name: Some(name), base: self.values.len(), binds: HashMap::new() };
        self.frames.push(new_frame);
        self
    }

    /// Exits the scope, destroying all local variables and
    /// returning the outer scope, if there is one
    fn exit(&mut self) -> &mut Self {
        if let Some(frame) = self.frames.pop() {
            self.values.truncate(frame.base);
        }
        self
    }
}

/// Represents a stack frame
#[derive(Debug)]
pub struct Frame<'scope> {
    scope: &'scope mut Environment,
}
impl<'scope> Frame<'scope> {
    fn new(scope: &'scope mut Environment, name: &'static str) -> Self {
        Self { scope: scope.enter(name) }
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
        self.scope.exit();
    }
}
