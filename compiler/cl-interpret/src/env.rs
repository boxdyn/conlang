//! Lexical and non-lexical scoping for variables

use super::{
    builtin::{BINARY, MISC, RANGE, UNARY},
    convalue::ConValue,
    error::{Error, IResult},
    function::Function,
    BuiltIn, Callable, Interpret,
};
use cl_ast::{Function as FnDecl, Sym};
use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Deref, DerefMut},
    rc::Rc,
};

type StackFrame = HashMap<Sym, Option<ConValue>>;

/// Implements a nested lexical scope
#[derive(Clone, Debug)]
pub struct Environment {
    global: Vec<(StackFrame, &'static str)>,
    frames: Vec<(StackFrame, &'static str)>,
}

impl Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (frame, name) in self
            .global
            .iter()
            .rev()
            .take(2)
            .rev()
            .chain(self.frames.iter())
        {
            writeln!(f, "--- {name} ---")?;
            for (var, val) in frame {
                write!(f, "{var}: ")?;
                match val {
                    Some(value) => writeln!(f, "\t{value}"),
                    None => writeln!(f, "<undefined>"),
                }?
            }
        }
        Ok(())
    }
}
impl Default for Environment {
    fn default() -> Self {
        Self {
            global: vec![
                (to_hashmap(RANGE), "range ops"),
                (to_hashmap(UNARY), "unary ops"),
                (to_hashmap(BINARY), "binary ops"),
                (to_hashmap(MISC), "builtins"),
                (HashMap::new(), "globals"),
            ],
            frames: vec![],
        }
    }
}
fn to_hashmap(from: &[&'static dyn BuiltIn]) -> HashMap<Sym, Option<ConValue>> {
    from.iter().map(|&v| (v.name(), Some(v.into()))).collect()
}

impl Environment {
    pub fn new() -> Self {
        Self::default()
    }
    /// Creates an [Environment] with no [builtins](super::builtin)
    pub fn no_builtins() -> Self {
        Self { global: vec![(Default::default(), "globals")], frames: vec![] }
    }

    pub fn push_frame(&mut self, name: &'static str, frame: StackFrame) {
        self.frames.push((frame, name));
    }

    pub fn pop_frame(&mut self) -> Option<StackFrame> {
        self.frames.pop().map(|f| f.0)
    }

    pub fn eval(&mut self, node: &impl Interpret) -> IResult<ConValue> {
        node.interpret(self)
    }

    /// Calls a function inside the interpreter's scope,
    /// and returns the result
    pub fn call(&mut self, name: Sym, args: &[ConValue]) -> IResult<ConValue> {
        // FIXME: Clone to satisfy the borrow checker
        let function = self.get(name)?.clone();
        function.call(self, args)
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
    pub fn get_mut(&mut self, id: Sym) -> IResult<&mut Option<ConValue>> {
        for (frame, _) in self.frames.iter_mut().rev() {
            if let Some(var) = frame.get_mut(&id) {
                return Ok(var);
            }
        }
        for (frame, _) in self.global.iter_mut().rev() {
            if let Some(var) = frame.get_mut(&id) {
                return Ok(var);
            }
        }
        Err(Error::NotDefined(id))
    }
    /// Resolves a variable immutably.
    ///
    /// Returns a reference to the variable's contents, if it is defined and initialized.
    pub fn get(&self, id: Sym) -> IResult<ConValue> {
        for (frame, _) in self.frames.iter().rev() {
            match frame.get(&id) {
                Some(Some(var)) => return Ok(var.clone()),
                Some(None) => return Err(Error::NotInitialized(id)),
                _ => (),
            }
        }
        for (frame, _) in self.global.iter().rev() {
            match frame.get(&id) {
                Some(Some(var)) => return Ok(var.clone()),
                Some(None) => return Err(Error::NotInitialized(id)),
                _ => (),
            }
        }
        Err(Error::NotDefined(id))
    }

    pub(crate) fn get_local(&self, id: Sym) -> IResult<ConValue> {
        for (frame, _) in self.frames.iter().rev() {
            match frame.get(&id) {
                Some(Some(var)) => return Ok(var.clone()),
                Some(None) => return Err(Error::NotInitialized(id)),
                _ => (),
            }
        }
        Err(Error::NotInitialized(id))
    }
    
    /// Inserts a new [ConValue] into this [Environment]
    pub fn insert(&mut self, id: Sym, value: Option<ConValue>) {
        if let Some((frame, _)) = self.frames.last_mut() {
            frame.insert(id, value);
        } else if let Some((frame, _)) = self.global.last_mut() {
            frame.insert(id, value);
        }
    }
    /// A convenience function for registering a [FnDecl] as a [Function]
    pub fn insert_fn(&mut self, decl: &FnDecl) {
        let FnDecl { name, .. } = decl;
        let (name, function) = (name, Rc::new(Function::new(decl)));
        if let Some((frame, _)) = self.frames.last_mut() {
            frame.insert(*name, Some(ConValue::Function(function.clone())));
        } else if let Some((frame, _)) = self.global.last_mut() {
            frame.insert(*name, Some(ConValue::Function(function.clone())));
        }
        // Tell the function to lift its upvars now, after it's been declared
        function.lift_upvars(self);
    }
}

/// Functions which aid in the implementation of [`Frame`]
impl Environment {
    /// Enters a scope, creating a new namespace for variables
    fn enter(&mut self, name: &'static str) -> &mut Self {
        self.frames.push((Default::default(), name));
        self
    }

    /// Exits the scope, destroying all local variables and
    /// returning the outer scope, if there is one
    fn exit(&mut self) -> &mut Self {
        self.frames.pop();
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
