//! A work-in-progress tree walk interpreter for Conlang
//!
//! Currently, major parts of the interpreter are not yet implemented, and major parts will never be
//! implemented in its current form. Namely, since no [ConValue] has a stable location, it's
//! meaningless to get a pointer to one, and would be undefined behavior to dereference a pointer to
//! one in any situation.

use crate::constructor::Constructor;

use super::*;
use cl_ast::{ast_visitor::Visit, *};
use std::borrow::Borrow;

macro trace($($t:tt)*) {{
    #[cfg(debug_assertions)]
    if std::env::var("CONLANG_TRACE").is_ok() {
        eprintln!($($t)*)
    }
}}

/// A work-in-progress tree walk interpreter for Conlang
pub trait Interpret {
    /// Interprets this thing in the given [`Environment`].
    ///
    /// Everything returns a value!™
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue>;
}

impl Interpret for File {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        trace!("// Running {}", self.name);
        /// Sorts items
        #[derive(Debug, Default)]
        struct ItemSorter<'ast>(pub [Vec<&'ast Item>; 8]);
        impl<'ast> Visit<'ast> for ItemSorter<'ast> {
            fn visit_item(&mut self, i: &'ast Item) {
                for stage in match &i.kind {
                    ItemKind::Module(_) => [0].as_slice(),
                    ItemKind::Use(_) => &[1, 6],
                    ItemKind::Enum(_) | ItemKind::Struct(_) | ItemKind::Alias(_) => &[2],
                    ItemKind::Function(_) => &[3, 7],
                    ItemKind::Impl(_) => &[4],
                    ItemKind::Const(_) | ItemKind::Static(_) => &[5],
                } {
                    self.0[*stage].push(i)
                }
            }
        }

        let mut items = ItemSorter::default();
        items.visit_file(self);
        for item in items.0.into_iter().flatten() {
            item.interpret(env)?;
        }

        Ok(ConValue::Empty)
    }
}

impl Interpret for Item {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        match &self.kind {
            ItemKind::Alias(item) => item.interpret(env),
            ItemKind::Const(item) => item.interpret(env),
            ItemKind::Static(item) => item.interpret(env),
            ItemKind::Module(item) => item.interpret(env),
            ItemKind::Function(item) => item.interpret(env),
            ItemKind::Struct(item) => item.interpret(env),
            ItemKind::Enum(item) => item.interpret(env),
            ItemKind::Impl(item) => item.interpret(env),
            ItemKind::Use(item) => item.interpret(env),
        }
    }
}

impl Interpret for Alias {
    fn interpret(&self, _env: &mut Environment) -> IResult<ConValue> {
        trace!("// TODO: {self}");
        Ok(ConValue::Empty)
    }
}

impl Interpret for Const {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Const { name, ty: _, init } = self;
        trace!("// Defining const {name}");

        let init = init.as_ref().interpret(env)?;
        env.insert(*name, init);
        Ok(ConValue::Empty)
    }
}

impl Interpret for Static {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Static { mutable: _, name, ty: _, init } = self;
        trace!("// Defining static {name}");

        let init = init.as_ref().interpret(env)?;
        env.insert(*name, init);
        Ok(ConValue::Empty)
    }
}

impl Interpret for Module {
    // TODO: Keep modules around somehow, rather than putting them on the stack
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { name, file } = self;
        trace!("// Defining module {name}");

        let mut scope = env.frame(name.to_ref());

        let out = match file {
            Some(file) => file.interpret(&mut scope),
            None => {
                eprintln!("Module {name} specified, but not imported.");
                Ok(ConValue::Empty)
            }
        };

        let frame = scope
            .pop_values()
            .expect("Environment frames must be balanced");
        env.insert(*name, ConValue::Module(frame.into()));

        out
    }
}

impl Interpret for Function {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        trace!("// Defining fn {}", self.name);

        // register the function in the current environment
        env.insert_fn(self);
        Ok(ConValue::Empty)
    }
}

impl Interpret for Struct {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { name, gens: _, kind } = self;
        trace!("// Defining struct {name}");

        let mut frame = env.frame(name.to_ref());

        match kind {
            StructKind::Empty => {
                let cs = Constructor { arity: 0, name: *name };
                frame.insert("call".into(), ConValue::TupleConstructor(cs));
                frame.insert("__nmemb".into(), ConValue::Int(0));
            }
            StructKind::Tuple(args) => {
                let cs = Constructor { arity: args.len() as _, name: *name };
                frame.insert("call".into(), ConValue::TupleConstructor(cs));
                frame.insert("__nmemb".into(), ConValue::Int(args.len() as _));
            }
            StructKind::Struct(members) => {
                // TODO: more precise type checking of structs
                for (idx, StructMember { vis: _, name: nm, ty: _ }) in members.iter().enumerate() {
                    trace!("// Defining {name}::{nm}");
                    frame.insert(*nm, ConValue::Int(idx as _));
                }
                frame.insert("__nmemb".into(), ConValue::Int(members.len() as _));
            }
        }

        let frame = frame
            .pop_values()
            .expect("Environment frames must be balanced");

        env.insert(*name, ConValue::Module(Box::new(frame)));
        Ok(ConValue::Empty)
    }
}

impl Interpret for Enum {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { name, gens: _, variants } = self;
        trace!("// Defining enum {name}");

        let mut scope = env.frame(name.to_ref());
        for (idx, Variant { name, kind, body }) in variants.iter().enumerate() {
            match (kind, body) {
                (StructKind::Empty, None) => scope.insert(*name, ConValue::Int(idx as _)),
                (StructKind::Empty, Some(idx)) => {
                    let idx = idx.interpret(&mut scope)?;
                    scope.insert(*name, idx)
                }
                (StructKind::Tuple(args), None) => {
                    let cs = Constructor { arity: args.len() as _, name: *name };
                    let mut variant = scope.frame(name.to_ref());
                    variant.insert("call".into(), ConValue::TupleConstructor(cs));
                    let frame = variant
                        .pop_values()
                        .expect("Frame stack should remain balanced.");
                    scope.insert(*name, ConValue::Module(Box::new(frame)));
                }
                (StructKind::Struct(_), None) => {}
                _ => eprintln!("Well-formedness error in {self}"),
            }
        }
        let frame = scope
            .pop_values()
            .expect("Frame stack should remain balanced.");
        env.insert(*name, ConValue::Module(Box::new(frame)));
        Ok(ConValue::Empty)
    }
}

impl Interpret for Impl {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self {
            gens: _,
            target: ImplKind::Type(Ty { span, kind: TyKind::Path(name), .. }),
            body,
        } = self
        else {
            trace!("TODO: impl X for Ty");
            return Ok(ConValue::Empty);
        };
        let mut frame = env.frame("impl");
        body.interpret(&mut frame)?;

        let frame = frame
            .pop_values()
            .expect("Environment frames must be balanced");
        match assignment::addrof_path(env, name.parts.as_slice())
            .map_err(|err| err.with_span(*span))?
        {
            ConValue::Module(m) => m.extend(frame),
            other => eprintln!("TODO: impl for {other}"),
        }
        Ok(ConValue::Empty)
    }
}

impl Interpret for Use {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { absolute: _, tree } = self;
        tree.interpret(env)
    }
}

impl Interpret for UseTree {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        // TODO: raw-bind use items
        type Bindings = HashMap<Sym, ConValue>;
        use std::collections::HashMap;

        fn get_bindings(
            tree: &UseTree,
            env: &mut Environment,
            bindings: &mut Bindings,
        ) -> IResult<()> {
            match tree {
                UseTree::Tree(use_trees) => {
                    for tree in use_trees {
                        get_bindings(tree, env, bindings)?;
                    }
                }
                UseTree::Path(PathPart::Ident(name), tree) => {
                    let Ok(ConValue::Module(m)) = env.get(*name) else {
                        Err(Error::TypeError())?
                    };
                    let mut scope = env.with_frame(name.to_ref(), *m);
                    let out = get_bindings(tree, &mut scope, bindings);
                    return out;
                }
                UseTree::Alias(name, alias) => {
                    bindings.insert(*alias, env.get(*name)?);
                }
                UseTree::Name(name) => {
                    bindings.insert(*name, env.get(*name)?);
                }
                UseTree::Glob => {
                    trace!("TODO: Improve glob imports");
                    if let Some((frame, name)) = env.pop_frame() {
                        for (k, v) in &frame {
                            bindings.insert(*k, v.clone());
                        }
                        env.push_frame(name, frame);
                    }
                }
                other => {
                    eprintln!("ERROR: Cannot use {other}");
                }
            }
            Ok(())
        }

        let mut bindings = Bindings::new();
        get_bindings(self, env, &mut bindings)?;

        for (name, value) in bindings {
            env.insert(name, value);
        }

        Ok(ConValue::Empty)
    }
}

impl Interpret for Stmt {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { span, kind, semi } = self;
        let out = match kind {
            StmtKind::Empty => Ok(ConValue::Empty),
            StmtKind::Item(stmt) => stmt.interpret(env),
            StmtKind::Expr(stmt) => stmt.interpret(env),
        }
        .map_err(|err| err.with_span(*span))?;
        Ok(match semi {
            Semi::Terminated => ConValue::Empty,
            Semi::Unterminated => out,
        })
    }
}

impl Interpret for Expr {
    #[inline]
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { span, kind } = self;
        kind.interpret(env).map_err(|err| err.with_span(*span))
    }
}

impl Interpret for ExprKind {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        match self {
            ExprKind::Empty => Ok(ConValue::Empty),
            ExprKind::Closure(v) => v.interpret(env),
            ExprKind::Quote(q) => q.interpret(env),
            ExprKind::Let(v) => v.interpret(env),
            ExprKind::Match(v) => v.interpret(env),
            ExprKind::Assign(v) => v.interpret(env),
            ExprKind::Modify(v) => v.interpret(env),
            ExprKind::Binary(v) => v.interpret(env),
            ExprKind::Unary(v) => v.interpret(env),
            ExprKind::Cast(v) => v.interpret(env),
            ExprKind::Member(v) => v.interpret(env),
            ExprKind::Index(v) => v.interpret(env),
            ExprKind::Structor(v) => v.interpret(env),
            ExprKind::Path(v) => v.interpret(env),
            ExprKind::Literal(v) => v.interpret(env),
            ExprKind::Array(v) => v.interpret(env),
            ExprKind::ArrayRep(v) => v.interpret(env),
            ExprKind::AddrOf(v) => v.interpret(env),
            ExprKind::Block(v) => v.interpret(env),
            ExprKind::Group(v) => v.interpret(env),
            ExprKind::Tuple(v) => v.interpret(env),
            ExprKind::While(v) => v.interpret(env),
            ExprKind::If(v) => v.interpret(env),
            ExprKind::For(v) => v.interpret(env),
            ExprKind::Break(v) => v.interpret(env),
            ExprKind::Return(v) => v.interpret(env),
            ExprKind::Continue => Err(Error::Continue()),
        }
    }
}

impl Interpret for Closure {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        Ok(ConValue::Closure(
            crate::closure::Closure::new(env, self).into(),
        ))
    }
}

impl Interpret for Quote {
    fn interpret(&self, _env: &mut Environment) -> IResult<ConValue> {
        // TODO: squoosh down into a ConValue?
        Ok(ConValue::Quote(self.quote.as_ref().clone().into()))
    }
}

impl Interpret for Let {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Let { mutable: _, name, ty: _, init } = self;
        match init.as_ref().map(|i| i.interpret(env)).transpose()? {
            Some(value) => {
                match pattern::substitution(env, name, value) {
                    Ok(sub) => {
                        for (name, value) in sub {
                            env.insert(name, value);
                        }
                        return Ok(ConValue::Bool(true));
                    }
                    Err(_e) => trace!("{_e}"),
                };
            }
            None => {
                for name in pattern::variables(name) {
                    env.insert(*name, ConValue::Empty);
                }
            }
        }
        Ok(ConValue::Bool(false))
    }
}

impl Interpret for Match {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { scrutinee, arms } = self;
        let scrutinee = scrutinee.interpret(env)?;
        for MatchArm(pat, expr) in arms {
            if let Ok(substitution) = pattern::substitution(env, pat, scrutinee.clone()) {
                let mut env = env.frame("match");
                for (name, value) in substitution {
                    env.insert(name, value);
                }
                return expr.interpret(&mut env);
            }
        }
        Err(Error::MatchNonexhaustive())
    }
}

pub(crate) mod assignment {
    /// Pattern matching engine for assignment
    use super::*;
    use std::collections::HashMap;
    type Namespace = HashMap<Sym, ConValue>;

    pub(super) fn pat_assign(env: &mut Environment, pat: &Pattern, value: ConValue) -> IResult<()> {
        for (name, value) in pattern::substitution(env, pat, value)
            .map_err(|_| Error::PatFailed(pat.clone().into()))?
        {
            match env.get_mut(name)? {
                &mut ConValue::Ref(id) => {
                    *(env.get_id_mut(id).ok_or(Error::StackOverflow(id))?) = value;
                }
                other => *other = value,
            }
        }
        Ok(())
    }

    pub(super) fn assign(env: &mut Environment, pat: &Expr, value: ConValue) -> IResult<()> {
        if let Ok(pat) = Pattern::try_from(pat.clone()) {
            return pat_assign(env, &pat, value);
        }
        match &pat.kind {
            ExprKind::Member(member) => *addrof_member(env, member)? = value,
            ExprKind::Index(index) => *addrof_index(env, index)? = value,
            ExprKind::Path(path) => *addrof_path(env, &path.parts)? = value,
            ExprKind::Unary(Unary { kind: UnaryKind::Deref, tail }) => match addrof(env, tail)? {
                &mut ConValue::Ref(r) => *env.get_id_mut(r).ok_or(Error::StackOverflow(r))? = value,
                _ => Err(Error::NotAssignable())?,
            },
            _ => Err(Error::NotAssignable())?,
        }
        Ok(())
    }

    pub(super) fn addrof<'e>(env: &'e mut Environment, pat: &Expr) -> IResult<&'e mut ConValue> {
        match &pat.kind {
            ExprKind::Path(path) => addrof_path(env, &path.parts),
            ExprKind::Member(member) => addrof_member(env, member),
            ExprKind::Index(index) => addrof_index(env, index),
            ExprKind::Group(Group { expr }) => addrof(env, expr),
            ExprKind::Unary(Unary { kind: UnaryKind::Deref, tail }) => match *addrof(env, tail)? {
                ConValue::Ref(place) => env.get_id_mut(place).ok_or(Error::NotIndexable()),
                _ => Err(Error::TypeError()),
            },
            _ => Err(Error::TypeError()),
        }
    }

    pub fn addrof_path<'e>(
        env: &'e mut Environment,
        path: &[PathPart],
    ) -> IResult<&'e mut ConValue> {
        match path {
            [PathPart::Ident(name)] => env.get_mut(*name),
            [PathPart::Ident(name), rest @ ..] => match env.get_mut(*name)? {
                ConValue::Module(env) => project_path_in_namespace(env, rest),
                _ => Err(Error::NotIndexable()),
            },
            _ => Err(Error::NotAssignable()),
        }
    }

    pub fn addrof_member<'e>(
        env: &'e mut Environment,
        member: &Member,
    ) -> IResult<&'e mut ConValue> {
        let Member { head, kind } = member;

        let head = addrof(env, head)?;
        project_memberkind(head, kind)
    }

    fn addrof_index<'e>(env: &'e mut Environment, index: &Index) -> IResult<&'e mut ConValue> {
        let Index { head, indices } = index;
        let indices = indices
            .iter()
            .map(|index| index.interpret(env))
            .collect::<IResult<Vec<_>>>()?;

        let mut head = addrof(env, head)?;
        // match head {
        //     ConValue::Slice(id, len) | ConValue::Array(id, len) => {

        //     }
        // }
        for index in indices {
            head = project_index(head, &index)?;
        }
        Ok(head)
    }

    /// Performs member-access "projection" from a ConValue to a particular element
    pub fn project_memberkind<'v>(
        value: &'v mut ConValue,
        kind: &MemberKind,
    ) -> IResult<&'v mut ConValue> {
        match (value, kind) {
            (ConValue::Struct(_name, s), MemberKind::Struct(id)) => {
                s.get_mut(id).ok_or(Error::NotDefined(*id))
            }
            (ConValue::TupleStruct(_name, s), MemberKind::Tuple(Literal::Int(id))) => {
                let len = s.len();
                s.get_mut(*id as usize)
                    .ok_or(Error::OobIndex(*id as _, len))
            }
            (ConValue::Tuple(t), MemberKind::Tuple(Literal::Int(id))) => {
                let len = t.len();
                t.get_mut(*id as usize)
                    .ok_or(Error::OobIndex(*id as _, len))
            }
            _ => Err(Error::TypeError()),
        }
    }

    /// Performs index "projection" from a ConValue to a particular element
    pub fn project_index<'v>(
        _value: &'v mut ConValue,
        _index: &ConValue,
    ) -> IResult<&'v mut ConValue> {
        Err(Error::NotIndexable())
    }

    pub fn project_path_in_namespace<'e>(
        env: &'e mut Namespace,
        path: &[PathPart],
    ) -> IResult<&'e mut ConValue> {
        match path {
            [] => Err(Error::NotAssignable()),
            [PathPart::Ident(name)] => env.get_mut(name).ok_or(Error::NotDefined(*name)),
            [PathPart::Ident(name), rest @ ..] => {
                match env.get_mut(name).ok_or(Error::NotDefined(*name))? {
                    ConValue::Module(env) => project_path_in_namespace(env, rest),
                    _ => Err(Error::NotIndexable()),
                }
            }
            [PathPart::SelfTy, ..] => todo!("calc_address for `Self`"),
            [PathPart::SuperKw, ..] => todo!("calc_address for `super`"),
        }
    }
}

impl Interpret for Assign {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Assign { parts } = self;
        let (head, tail) = parts.borrow();
        let init = tail.interpret(env)?;
        // Resolve the head pattern
        assignment::assign(env, head, init).map(|_| ConValue::Empty)
    }
}
impl Interpret for Modify {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Modify { kind: op, parts } = self;
        let (head, tail) = parts.borrow();
        // Get the initializer and the tail
        let init = tail.interpret(env)?;
        // Resolve the head pattern
        let target = assignment::addrof(env, head)?;

        match op {
            ModifyKind::Add => target.add_assign(init),
            ModifyKind::Sub => target.sub_assign(init),
            ModifyKind::Mul => target.mul_assign(init),
            ModifyKind::Div => target.div_assign(init),
            ModifyKind::Rem => target.rem_assign(init),
            ModifyKind::And => target.bitand_assign(init),
            ModifyKind::Or => target.bitor_assign(init),
            ModifyKind::Xor => target.bitxor_assign(init),
            ModifyKind::Shl => target.shl_assign(init),
            ModifyKind::Shr => target.shr_assign(init),
        }?;
        Ok(ConValue::Empty)
    }
}
impl Interpret for Binary {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Binary { kind, parts } = self;
        let (head, tail) = parts.borrow();

        let head = head.interpret(env)?;
        match kind {
            BinaryKind::LogAnd => {
                return if head.truthy()? {
                    tail.interpret(env)
                } else {
                    Ok(head)
                }; // Short circuiting
            }
            BinaryKind::LogOr => {
                return if !head.truthy()? {
                    tail.interpret(env)
                } else {
                    Ok(head)
                }; // Short circuiting
            }
            BinaryKind::LogXor => {
                return Ok(ConValue::Bool(
                    head.truthy()? ^ tail.interpret(env)?.truthy()?,
                ));
            }
            _ => {}
        }

        let tail = tail.interpret(env)?;

        #[allow(unused)]
        let operator = match kind {
            BinaryKind::Lt => "lt",
            BinaryKind::LtEq => "lteq",
            BinaryKind::Equal => "eq",
            BinaryKind::NotEq => "neq",
            BinaryKind::GtEq => "gteq",
            BinaryKind::Gt => "gt",
            BinaryKind::RangeExc => "range_exc",
            BinaryKind::RangeInc => "range_inc",
            BinaryKind::LogAnd => "log_and",
            BinaryKind::LogOr => "log_or",
            BinaryKind::LogXor => "log_xor",
            BinaryKind::BitAnd => "bit_and",
            BinaryKind::BitOr => "bit_or",
            BinaryKind::BitXor => "bit_xor",
            BinaryKind::Shl => "shl",
            BinaryKind::Shr => "shr",
            BinaryKind::Add => "add",
            BinaryKind::Sub => "sub",
            BinaryKind::Mul => "mul",
            BinaryKind::Div => "div",
            BinaryKind::Rem => "rem",
            BinaryKind::Call => "call",
        };

        if let ConValue::Struct(name, _) | ConValue::TupleStruct(name, _) = head
            && let Ok(ConValue::Module(m)) = env.get_mut(name)
            && let Some(f) = m.get(&operator.into())
        {
            return f.clone().call(env, &[head, tail]);
        }

        if let Ok(ConValue::Module(m)) = env.get_mut(head.typename().into())
            && let Some(f) = m.get(&operator.into())
        {
            return f.clone().call(env, &[head, tail]);
        }

        match kind {
            BinaryKind::Lt => head.lt(&tail),
            BinaryKind::LtEq => head.lt_eq(&tail),
            BinaryKind::Equal => head.eq(&tail),
            BinaryKind::NotEq => head.neq(&tail),
            BinaryKind::GtEq => head.gt_eq(&tail),
            BinaryKind::Gt => head.gt(&tail),
            BinaryKind::RangeExc => env.call("RangeExc".into(), &[head, tail]),
            BinaryKind::RangeInc => env.call("RangeInc".into(), &[head, tail]),
            BinaryKind::BitAnd => head & tail,
            BinaryKind::BitOr => head | tail,
            BinaryKind::BitXor => head ^ tail,
            BinaryKind::Shl => head << tail,
            BinaryKind::Shr => head >> tail,
            BinaryKind::Add => head + tail,
            BinaryKind::Sub => head - tail,
            BinaryKind::Mul => head * tail,
            BinaryKind::Div => head / tail,
            BinaryKind::Rem => head % tail,
            BinaryKind::Call => match tail {
                ConValue::Empty => head.call(env, &[]),
                ConValue::Tuple(args) => head.call(env, &args),
                _ => Err(Error::TypeError()),
            },
            _ => Ok(head),
        }
    }
}

impl Interpret for Unary {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Unary { kind, tail } = self;
        if *kind == UnaryKind::Loop {
            loop {
                match tail.interpret(env) {
                    Err(Error { kind: ErrorKind::Break(value), .. }) => return Ok(value),
                    Err(Error { kind: ErrorKind::Continue, .. }) => continue,
                    e => e?,
                };
            }
        }

        let operator = match kind {
            UnaryKind::Loop => unreachable!(),
            UnaryKind::Deref => "deref",
            UnaryKind::Neg => "neg",
            UnaryKind::Not => "not",
            UnaryKind::RangeExc => "RangeTo",
            UnaryKind::RangeInc => "RangeToInc",
            UnaryKind::At => "at",
            UnaryKind::Tilde => "tilde",
        };

        let operand = tail.interpret(env)?;

        if let ConValue::Struct(name, _) | ConValue::TupleStruct(name, _) = operand
            && let Ok(ConValue::Module(m)) = env.get_mut(name)
            && let Some(f) = m.get(&operator.into())
        {
            return f.clone().call(env, &[operand]);
        }

        if let Ok(ConValue::Module(m)) = env.get_mut(operand.typename().into())
            && let Some(f) = m.get(&operator.into())
        {
            return f.clone().call(env, &[operand]);
        }

        env.call(operator.into(), &[operand])
    }
}

fn cast(env: &Environment, value: ConValue, ty: Sym) -> IResult<ConValue> {
    let value = match (value, ty.to_ref()) {
        (value, "str") => return Ok(ConValue::Str(format!("{value}").into())),
        (value, "String") => return Ok(ConValue::String(format!("{value}"))),
        (ConValue::Empty, _) => 0,
        (ConValue::Int(i), _) => i as _,
        (ConValue::Bool(b), _) => b as _,
        (ConValue::Char(c), _) => c as _,
        (ConValue::Ref(v), _) => {
            return cast(
                env,
                env.get_id(v).cloned().ok_or(Error::StackUnderflow())?,
                ty,
            );
        }
        (ConValue::Float(f), "f32" | "f64") => return Ok(ConValue::Float(f)),
        (ConValue::Float(f), _) => f as _,
        _ => Err(Error::TypeError())?,
    };
    Ok(match &*ty {
        "u8" => ConValue::Int(value as u8 as _),
        "i8" => ConValue::Int(value as i8 as _),
        "u16" => ConValue::Int(value as u16 as _),
        "i16" => ConValue::Int(value as i16 as _),
        "u32" => ConValue::Int(value as u32 as _),
        "i32" => ConValue::Int(value as i32 as _),
        "u64" => ConValue::Int(value),
        "i64" => ConValue::Int(value),
        "f32" => ConValue::Float(value as f32 as _),
        "f64" => ConValue::Float(value as f64 as _),
        "char" => ConValue::Char(char::from_u32(value as _).unwrap_or('\u{fffd}')),
        "bool" => ConValue::Bool(value != 0),
        _ => Err(Error::NotDefined(ty))?,
    })
}

impl Interpret for Cast {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Cast { head, ty } = self;
        let value = head.interpret(env)?;

        if let TyKind::Tuple(TyTuple { types }) = &ty.kind
            && types.as_slice().is_empty()
        {
            return Ok(ConValue::Empty);
        }
        let TyKind::Path(Path { absolute: false, parts }) = &ty.kind else {
            Err(Error::TypeError())?
        };
        match parts.as_slice() {
            [PathPart::Ident(ty)] => cast(env, value, *ty),
            _ => Err(Error::TypeError()),
        }
    }
}

impl Interpret for Member {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Member { head, kind } = self;
        // Attempt member access projection (fast path)
        if let Ok(member) = assignment::addrof_member(env, self) {
            return Ok(member.clone());
        }
        let mut values = vec![];

        // Evaluate if this can be Self'd
        let addr = match (&head.kind, kind) {
            (ExprKind::Path(p), MemberKind::Call(..)) => {
                p.as_sym()
                    .and_then(|name| Some(ConValue::Ref(env.id_of(name).ok()?))) // "borrow" it
            }
            _ => None,
        };

        let mut value = head.interpret(env)?;

        // Perform alternate member access
        match (&value, &kind) {
            (ConValue::Struct(_name, memb), MemberKind::Call(name, args))
                if memb.contains_key(name) =>
            {
                let f = memb.get(name).cloned().expect("function exists");
                values.push(addr.unwrap_or(value));
                for arg in &args.exprs {
                    values.push(arg.interpret(env)?);
                }
                f.call(env, &values)
            }
            (_, MemberKind::Call(name, args)) => {
                values.push(addr.unwrap_or(value));
                for arg in &args.exprs {
                    values.push(arg.interpret(env)?);
                }
                env.call(*name, &values)
            }
            (_, kind) => assignment::project_memberkind(&mut value, kind).cloned(),
        }
    }
}
impl Interpret for Index {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { head, indices } = self;
        let mut head = head.interpret(env)?;
        for index in indices {
            head = head.index(&index.interpret(env)?, env)?;
        }
        Ok(head)
    }
}
impl Interpret for Structor {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { to: Path { absolute: _, parts }, init } = self;
        use std::collections::HashMap;

        // Look up struct/enum-struct definition
        // use that definition to place the struct parts

        let name = match parts.last() {
            Some(PathPart::Ident(name)) => *name,
            Some(PathPart::SelfTy) => "Self".into(),
            Some(PathPart::SuperKw) => "super".into(),
            None => "".into(),
        };

        let mut map = HashMap::new();
        for Fielder { name, init } in init {
            let value = match init {
                Some(init) => init.interpret(env)?,
                None => env.get(*name)?,
            };
            map.insert(*name, value);
        }
        Ok(ConValue::Struct(name, Box::new(map)))
    }
}

impl Interpret for Path {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { absolute: _, parts } = self;

        assignment::addrof_path(env, parts).cloned()
    }
}
impl Interpret for Literal {
    fn interpret(&self, _env: &mut Environment) -> IResult<ConValue> {
        Ok(match self {
            Literal::String(value) => ConValue::from(value.as_str()),
            Literal::Char(value) => ConValue::Char(*value),
            Literal::Bool(value) => ConValue::Bool(*value),
            Literal::Float(value) => ConValue::Float(f64::from_bits(*value)),
            Literal::Int(value) => ConValue::Int(*value as _),
        })
    }
}
impl Interpret for Array {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { values } = self;
        let mut out = vec![];
        for expr in values {
            out.push(expr.interpret(env)?)
        }
        Ok(ConValue::Array(out.into()))
    }
}
impl Interpret for ArrayRep {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { value, repeat } = self;
        let value = value.interpret(env)?;
        let ConValue::Int(repeat) = repeat.interpret(env)? else {
            Err(Error::TypeError())?
        };
        if repeat < 0 {
            // TODO: a special error just for you
            Err(Error::NotIndexable())?
        }

        Ok(ConValue::Array(vec![value; repeat as usize].into()))
    }
}
impl Interpret for AddrOf {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { mutable: _, expr } = self;
        match &expr.kind {
            ExprKind::Index(_) => todo!("AddrOf array index"),
            ExprKind::Path(Path { parts, .. }) => match parts.as_slice() {
                [PathPart::Ident(name)] => Ok(ConValue::Ref(env.id_of(*name)?)),
                _ => todo!("Path traversal in AddrOf(\"{self}\")"),
            },
            _ => {
                let value = expr.interpret(env)?;
                let temp = env.stack_alloc(value)?;
                Ok(ConValue::Ref(temp))
            }
        }
    }
}
impl Interpret for Block {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { stmts } = self;
        let mut env = env.frame("block");
        let mut out = ConValue::Empty;
        for stmt in stmts {
            out = stmt.interpret(&mut env)?;
        }
        Ok(out)
    }
}
impl Interpret for Group {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { expr } = self;
        expr.interpret(env)
    }
}
impl Interpret for Tuple {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { exprs } = self;
        Ok(ConValue::Tuple(
            exprs
                .iter()
                .try_fold(vec![], |mut out, element| {
                    out.push(element.interpret(env)?);
                    Ok(out)
                })?
                .into(),
        ))
    }
}
impl Interpret for While {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { cond, pass, fail } = self;
        loop {
            if cond.interpret(env)?.truthy()? {
                match pass.interpret(env) {
                    Err(Error { kind: ErrorKind::Break(value), .. }) => break Ok(value),
                    Err(Error { kind: ErrorKind::Continue, .. }) => continue,
                    e => e?,
                };
            } else {
                break fail.interpret(env);
            }
        }
    }
}
impl Interpret for If {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { cond, pass, fail } = self;
        if cond.interpret(env)?.truthy()? {
            pass.interpret(env)
        } else {
            fail.interpret(env)
        }
    }
}
impl Interpret for For {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { bind, cond, pass, fail } = self;
        let cond = cond.interpret(env)?;
        // TODO: A better iterator model
        let mut bounds: Box<dyn Iterator<Item = ConValue>> = match &cond {
            ConValue::TupleStruct(name, values) => match name.to_ref() {
                "RangeExc" => match values.as_ref().as_ref() {
                    &[ConValue::Int(from), ConValue::Int(to)] => {
                        Box::new((from..to).map(ConValue::Int))
                    }
                    _ => Err(Error::NotIterable())?,
                },
                "RangeInc" => match values.as_ref().as_ref() {
                    &[ConValue::Int(from), ConValue::Int(to)] => {
                        Box::new((from..=to).map(ConValue::Int))
                    }
                    _ => Err(Error::NotIterable())?,
                },
                _ => Err(Error::NotIterable())?,
            },
            ConValue::Array(a) => Box::new(a.iter().cloned()),
            &ConValue::Slice(head, len) => Box::new((head..head + len).map(ConValue::Ref)),
            // In the production compiler this may be fully unrolled
            ConValue::Tuple(t) => Box::new(t.iter().cloned()),
            ConValue::Str(s) => Box::new(s.chars().map(ConValue::Char)),
            _ => Err(Error::TypeError())?,
        };
        loop {
            let mut env = env.frame("loop variable");
            if let Some(value) = bounds.next() {
                for (name, value) in pattern::substitution(&env, bind, value)? {
                    env.insert(name, value);
                }
                match pass.interpret(&mut env) {
                    Err(Error { kind: ErrorKind::Break(value), .. }) => break Ok(value),
                    Err(Error { kind: ErrorKind::Continue, .. }) => continue,
                    e => e?,
                };
            } else {
                break fail.interpret(&mut env);
            }
        }
    }
}
impl Interpret for Else {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { body } = self;
        match body {
            Some(body) => body.interpret(env),
            None => Ok(ConValue::Empty),
        }
    }
}
impl Interpret for Return {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { body } = self;
        Err(Error::Return(
            body.as_ref()
                .map(|body| body.interpret(env))
                .unwrap_or(Ok(ConValue::Empty))?,
        ))
    }
}
impl Interpret for Break {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { body } = self;
        Err(Error::Break(
            body.as_ref()
                .map(|body| body.interpret(env))
                .unwrap_or(Ok(ConValue::Empty))?,
        ))
    }
}
