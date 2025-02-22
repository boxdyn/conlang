//! A work-in-progress tree walk interpreter for Conlang
//!
//! Currently, major parts of the interpreter are not yet implemented, and major parts will never be
//! implemented in its current form. Namely, since no [ConValue] has a stable location, it's
//! meaningless to get a pointer to one, and would be undefined behavior to dereference a pointer to
//! one in any situation.

use std::{borrow::Borrow, rc::Rc};

use super::*;
use cl_ast::{ast_visitor::Visit, *};
use cl_structures::intern::interned::Interned;
/// A work-in-progress tree walk interpreter for Conlang
pub trait Interpret {
    /// Interprets this thing in the given [`Environment`].
    ///
    /// Everything returns a value!™
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue>;
}

impl Interpret for File {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        /// Sorts items
        #[derive(Debug, Default)]
        struct ItemSorter<'ast>(pub [Vec<&'ast Item>; 6]);
        impl<'ast> Visit<'ast> for ItemSorter<'ast> {
            fn visit_item(&mut self, i: &'ast Item) {
                self.0[match &i.kind {
                    ItemKind::Module(_) => 0,
                    ItemKind::Use(_) => 1,
                    ItemKind::Enum(_) | ItemKind::Struct(_) | ItemKind::Alias(_) => 2,
                    ItemKind::Function(_) => 3,
                    ItemKind::Impl(_) => 4,
                    ItemKind::Const(_) | ItemKind::Static(_) => 5,
                }]
                .push(i)
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
        println!("TODO: {self}");
        Ok(ConValue::Empty)
    }
}
impl Interpret for Const {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Const { name, ty: _, init } = self;

        let init = init.as_ref().interpret(env)?;
        env.insert(*name, Some(init));
        Ok(ConValue::Empty)
    }
}
impl Interpret for Static {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Static { mutable: _, name, ty: _, init } = self;

        let init = init.as_ref().interpret(env)?;
        env.insert(*name, Some(init));
        Ok(ConValue::Empty)
    }
}
impl Interpret for Module {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { name, kind } = self;
        env.push_frame(Interned::to_ref(name), Default::default());
        let out = match kind {
            ModuleKind::Inline(file) => file.interpret(env),
            ModuleKind::Outline => {
                eprintln!("Module {name} specified, but not imported.");
                Ok(ConValue::Empty)
            }
        };

        let (frame, _) = env
            .pop_frame()
            .expect("Environment frames must be balanced");
        env.insert(*name, Some(ConValue::Module(frame.into())));

        out
    }
}
impl Interpret for Function {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        // register the function in the current environment
        env.insert_fn(self);
        Ok(ConValue::Empty)
    }
}
impl Interpret for Struct {
    fn interpret(&self, _env: &mut Environment) -> IResult<ConValue> {
        println!("TODO: {self}");
        Ok(ConValue::Empty)
    }
}
impl Interpret for Enum {
    fn interpret(&self, _env: &mut Environment) -> IResult<ConValue> {
        println!("TODO: {self}");
        Ok(ConValue::Empty)
    }
}
impl Interpret for Impl {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        println!("TODO: impl {}", self.target);
        let Self { target: _, body } = self;
        body.interpret(env)
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
                        Err(Error::TypeError)?
                    };
                    env.push_frame(Interned::to_ref(name), *m);
                    let out = get_bindings(tree, env, bindings);
                    env.pop_frame();
                    return out;
                }
                UseTree::Alias(name, alias) => {
                    bindings.insert(*alias, env.get(*name)?);
                }
                UseTree::Name(name) => {
                    bindings.insert(*name, env.get(*name)?);
                }
                UseTree::Glob => {
                    if let Some((frame, name)) = env.pop_frame() {
                        for (k, v) in &frame {
                            if let Some(v) = v {
                                bindings.insert(*k, v.clone());
                            }
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
            env.insert(name, Some(value));
        }

        Ok(ConValue::Empty)
    }
}

impl Interpret for Stmt {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { extents: _, kind, semi } = self;
        let out = match kind {
            StmtKind::Empty => ConValue::Empty,
            StmtKind::Item(stmt) => stmt.interpret(env)?,
            StmtKind::Expr(stmt) => stmt.interpret(env)?,
        };
        Ok(match semi {
            Semi::Terminated => ConValue::Empty,
            Semi::Unterminated => out,
        })
    }
}

impl Interpret for Expr {
    #[inline]
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { extents: _, kind } = self;
        kind.interpret(env)
    }
}

impl Interpret for ExprKind {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        match self {
            ExprKind::Empty => Ok(ConValue::Empty),
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
            ExprKind::Continue => Err(Error::Continue),
        }
    }
}

impl Interpret for Quote {
    fn interpret(&self, _env: &mut Environment) -> IResult<ConValue> {
        // TODO: squoosh down into a ConValue?
        Ok(ConValue::Quote(self.quote.clone()))
    }
}

impl Interpret for Let {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Let { mutable: _, name, ty: _, init } = self;
        match init.as_ref().map(|i| i.interpret(env)).transpose()? {
            Some(value) => {
                for (name, value) in assignment::pattern_substitution(name, value)? {
                    env.insert(*name, Some(value));
                }
            }
            None => {
                for name in assignment::pattern_variables(name) {
                    env.insert(*name, None);
                }
            }
        }
        Ok(ConValue::Empty)
    }
}

impl Interpret for Match {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { scrutinee, arms } = self;
        let scrutinee = scrutinee.interpret(env)?;
        for MatchArm(pat, expr) in arms {
            if let Ok(substitution) = assignment::pattern_substitution(pat, scrutinee.clone()) {
                let mut env = env.frame("match");
                for (name, value) in substitution {
                    env.insert(*name, Some(value));
                }
                return expr.interpret(&mut env);
            }
        }
        Err(Error::MatchNonexhaustive)
    }
}

mod assignment {
    /// Pattern matching engine for assignment
    use super::*;
    use std::collections::HashMap;
    type Namespace = HashMap<Sym, Option<ConValue>>;

    /// Gets the path variables in the given Pattern
    pub fn pattern_variables(pat: &Pattern) -> Vec<&Sym> {
        fn patvars<'p>(set: &mut Vec<&'p Sym>, pat: &'p Pattern) {
            match pat {
                Pattern::Name(name) if &**name == "_" => {}
                Pattern::Name(name) => set.push(name),
                Pattern::Literal(_) => {}
                Pattern::Ref(_, pattern) => patvars(set, pattern),
                Pattern::Tuple(patterns) | Pattern::Array(patterns) => {
                    patterns.iter().for_each(|pat| patvars(set, pat))
                }
                Pattern::Struct(_path, items) => {
                    items.iter().for_each(|(name, pat)| match pat {
                        Some(pat) => patvars(set, pat),
                        None => set.push(name),
                    });
                }
            }
        }
        let mut set = Vec::new();
        patvars(&mut set, pat);
        set
    }

    /// Appends a substitution to the provided table
    pub fn append_sub<'pat>(
        sub: &mut HashMap<&'pat Sym, ConValue>,
        pat: &'pat Pattern,
        value: ConValue,
    ) -> IResult<()> {
        match (pat, value) {
            (Pattern::Array(patterns), ConValue::Array(values))
            | (Pattern::Tuple(patterns), ConValue::Tuple(values)) => {
                if patterns.len() != values.len() {
                    Err(Error::ArgNumber { want: patterns.len(), got: values.len() })?
                }
                for (pat, value) in patterns.iter().zip(Vec::from(values).into_iter()) {
                    append_sub(sub, pat, value)?;
                }
                Ok(())
            }
            (Pattern::Tuple(patterns), ConValue::Empty) if patterns.is_empty() => Ok(()),

            (Pattern::Literal(Literal::Bool(a)), ConValue::Bool(b)) => {
                (*a == b).then_some(()).ok_or(Error::NotAssignable)
            }
            (Pattern::Literal(Literal::Char(a)), ConValue::Char(b)) => {
                (*a == b).then_some(()).ok_or(Error::NotAssignable)
            }
            (Pattern::Literal(Literal::Float(a)), ConValue::Float(b)) => (f64::from_bits(*a) == b)
            .then_some(())
            .ok_or(Error::NotAssignable),
            (Pattern::Literal(Literal::Int(a)), ConValue::Int(b)) => {
                (b == *a as _).then_some(()).ok_or(Error::NotAssignable)
            }
            (Pattern::Literal(Literal::String(a)), ConValue::String(b)) => {
                (*a == *b).then_some(()).ok_or(Error::NotAssignable)
            }
            (Pattern::Literal(_), _) => Err(Error::NotAssignable),

            (Pattern::Name(name), _) if "_".eq(&**name) => Ok(()),
            (Pattern::Name(name), value) => {
                sub.insert(name, value);
                Ok(())
            }

            (Pattern::Ref(_, pat), ConValue::Ref(r)) => {
                append_sub(sub, pat, Rc::unwrap_or_clone(r))
            }

            (Pattern::Struct(_path, patterns), ConValue::Struct(parts)) => {
                let (_name, mut values) = *parts;
                if patterns.len() != values.len() {
                    return Err(Error::TypeError);
                }
                for (name, pat) in patterns {
                    let value = values.remove(name).ok_or(Error::TypeError)?;
                    match pat {
                        Some(pat) => append_sub(sub, pat, value)?,
                        None => {
                            sub.insert(name, value);
                        }
                    }
                }
                Ok(())
            }

            (pat, value) => {
                eprintln!("Could not match pattern `{pat}` with value `{value}`!");
                Err(Error::NotAssignable)
            }
        }
    }

    /// Constructs a substitution from a pattern and a value
    pub fn pattern_substitution(
        pat: &Pattern,
        value: ConValue,
    ) -> IResult<HashMap<&Sym, ConValue>> {
        let mut sub = HashMap::new();
        append_sub(&mut sub, pat, value)?;
        Ok(sub)
    }

    pub(super) fn pat_assign(env: &mut Environment, pat: &Pattern, value: ConValue) -> IResult<()> {
        let mut substitution = HashMap::new();
        append_sub(&mut substitution, pat, value)
            .map_err(|_| Error::PatFailed(pat.clone().into()))?;
        for (path, value) in substitution {
            env.insert(*path, Some(value));
        }
        Ok(())
    }

    pub(super) fn assign(env: &mut Environment, pat: &ExprKind, value: ConValue) -> IResult<()> {
        if let Ok(pat) = Pattern::try_from(pat.clone()) {
            return pat_assign(env, &pat, value);
        }
        match pat {
            ExprKind::Member(member) => *addrof_member(env, member)? = value,
            ExprKind::Index(index) => *addrof_index(env, index)? = value,
            _ => Err(Error::NotAssignable)?,
        }
        Ok(())
    }

    pub(super) fn addrof<'e>(
        env: &'e mut Environment,
        pat: &ExprKind,
    ) -> IResult<&'e mut ConValue> {
        match pat {
            ExprKind::Path(path) => addrof_path(env, &path.parts)?
                .as_mut()
                .ok_or(Error::NotInitialized("".into())),
            ExprKind::Member(member) => addrof_member(env, member),
            ExprKind::Index(index) => addrof_index(env, index),
            ExprKind::Group(Group { expr }) => addrof(env, expr),
            ExprKind::AddrOf(AddrOf { mutable: Mutability::Mut, expr }) => addrof(env, expr),
            _ => Err(Error::TypeError),
        }
    }

    pub fn addrof_path<'e>(
        env: &'e mut Environment,
        path: &[PathPart],
    ) -> IResult<&'e mut Option<ConValue>> {
        match path {
            [PathPart::Ident(name)] => env.get_mut(*name),
            [PathPart::Ident(name), rest @ ..] => match env.get_mut(*name)? {
                Some(ConValue::Module(env)) => addrof_path_within_namespace(env, rest),
                _ => Err(Error::NotIndexable),
            },
            _ => Err(Error::NotAssignable),
        }
    }

    fn addrof_member<'e>(env: &'e mut Environment, member: &Member) -> IResult<&'e mut ConValue> {
        let Member { head, kind } = member;
        let ExprKind::Path(path) = head.as_ref() else {
            return Err(Error::TypeError);
        };
        let slot = addrof_path(env, &path.parts)?
            .as_mut()
            .ok_or(Error::NotAssignable)?;
        Ok(match (slot, kind) {
            (ConValue::Struct(s), MemberKind::Struct(id)) => {
                s.1.get_mut(id).ok_or(Error::NotDefined(*id))?
            }
            (ConValue::Tuple(t), MemberKind::Tuple(Literal::Int(id))) => t
                .get_mut(*id as usize)
                .ok_or_else(|| Error::NotDefined(id.to_string().into()))?,
            _ => Err(Error::TypeError)?,
        })
    }

    fn addrof_index<'e>(env: &'e mut Environment, index: &Index) -> IResult<&'e mut ConValue> {
        let Index { head, indices } = index;
        let indices = indices
            .iter()
            .map(|index| index.interpret(env))
            .collect::<IResult<Vec<_>>>()?;
        let mut head = addrof(env, head)?;
        for index in indices {
            head = match (head, index) {
                (ConValue::Array(a), ConValue::Int(i)) => {
                    let a_len = a.len();
                    a.get_mut(i as usize)
                        .ok_or(Error::OobIndex(i as usize, a_len))?
                }
                _ => Err(Error::NotIndexable)?,
            }
        }
        Ok(head)
    }

    pub fn addrof_path_within_namespace<'e>(
        env: &'e mut Namespace,
        path: &[PathPart],
    ) -> IResult<&'e mut Option<ConValue>> {
        match path {
            [] => Err(Error::NotAssignable),
            [PathPart::Ident(name)] => env.get_mut(name).ok_or(Error::NotDefined(*name)),
            [PathPart::Ident(name), rest @ ..] => {
                match env.get_mut(name).ok_or(Error::NotDefined(*name))? {
                    Some(ConValue::Module(env)) => addrof_path_within_namespace(env, rest),
                    _ => Err(Error::NotIndexable),
                }
            }
            [PathPart::SelfKw, rest @ ..] => addrof_path_within_namespace(env, rest),
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

        // Short-circuiting ops
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
        match kind {
            BinaryKind::Lt => head.lt(&tail),
            BinaryKind::LtEq => head.lt_eq(&tail),
            BinaryKind::Equal => head.eq(&tail),
            BinaryKind::NotEq => head.neq(&tail),
            BinaryKind::GtEq => head.gt_eq(&tail),
            BinaryKind::Gt => head.gt(&tail),
            BinaryKind::RangeExc => head.range_exc(tail),
            BinaryKind::RangeInc => head.range_inc(tail),
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
                _ => Err(Error::TypeError),
            },
            _ => Ok(head),
        }

        // // Temporarily disabled, to avoid function dispatch overhead while I screw around
        // // Not like it helped much in the first place!
        // match kind {
        //     BinaryKind::Mul => env.call("mul", &[head, tail]),
        //     BinaryKind::Div => env.call("div", &[head, tail]),
        //     BinaryKind::Rem => env.call("rem", &[head, tail]),
        //     BinaryKind::Add => env.call("add", &[head, tail]),
        //     BinaryKind::Sub => env.call("sub", &[head, tail]),
        //     BinaryKind::Shl => env.call("shl", &[head, tail]),
        //     BinaryKind::Shr => env.call("shr", &[head, tail]),
        //     BinaryKind::BitAnd => env.call("and", &[head, tail]),
        //     BinaryKind::BitOr => env.call("or", &[head, tail]),
        //     BinaryKind::BitXor => env.call("xor", &[head, tail]),
        //     BinaryKind::RangeExc => env.call("range_exc", &[head, tail]),
        //     BinaryKind::RangeInc => env.call("range_inc", &[head, tail]),
        //     BinaryKind::Lt => env.call("lt", &[head, tail]),
        //     BinaryKind::LtEq => env.call("lt_eq", &[head, tail]),
        //     BinaryKind::Equal => env.call("eq", &[head, tail]),
        //     BinaryKind::NotEq => env.call("neq", &[head, tail]),
        //     BinaryKind::GtEq => env.call("gt_eq", &[head, tail]),
        //     BinaryKind::Gt => env.call("gt", &[head, tail]),
        //     BinaryKind::Dot => todo!("search within a type's namespace!"),
        //     BinaryKind::Call => match tail {
        //         ConValue::Empty => head.call(env, &[]),
        //         ConValue::Tuple(args) => head.call(env, &args),
        //         _ => Err(Error::TypeError),
        //     },
        //     _ => Ok(head),
        // }
    }
}

impl Interpret for Unary {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Unary { kind, tail } = self;
        match kind {
            UnaryKind::Loop => loop {
                match tail.interpret(env) {
                    Err(Error::Break(value)) => return Ok(value),
                    Err(Error::Continue) => continue,
                    e => e?,
                };
            },
            UnaryKind::Deref => {
                let operand = tail.interpret(env)?;
                env.call("deref".into(), &[operand])
            }
            UnaryKind::Neg => {
                let operand = tail.interpret(env)?;
                env.call("neg".into(), &[operand])
            }
            UnaryKind::Not => {
                let operand = tail.interpret(env)?;
                env.call("not".into(), &[operand])
            }
            UnaryKind::At => {
                let operand = tail.interpret(env)?;
                println!("{operand}");
                Ok(operand)
            }
            UnaryKind::Tilde => unimplemented!("Tilde operator"),
        }
    }
}

fn cast(value: ConValue, ty: Sym) -> IResult<ConValue> {
    let value = match value {
        ConValue::Empty => 0,
        ConValue::Int(i) => i as _,
        ConValue::Bool(b) => b as _,
        ConValue::Char(c) => c as _,
        ConValue::Ref(v) => return cast((*v).clone(), ty),
        // TODO: This, better
        ConValue::Float(_) if ty.starts_with('f') => return Ok(value),
        ConValue::Float(f) => f as _,
        _ => Err(Error::TypeError)?,
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
        "bool" => ConValue::Bool(value < 0),
        _ => Err(Error::NotDefined(ty))?,
    })
}

impl Interpret for Cast {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Cast { head, ty } = self;
        let value = head.interpret(env)?;
        if TyKind::Empty == ty.kind {
            return Ok(ConValue::Empty);
        };
        let TyKind::Path(Path { absolute: false, parts }) = &ty.kind else {
            Err(Error::TypeError)?
        };
        match parts.as_slice() {
            [PathPart::Ident(ty)] => cast(value, *ty),
            _ => Err(Error::TypeError),
        }
    }
}

impl Interpret for Member {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Member { head, kind } = self;
        let head = head.interpret(env)?;
        match (head, kind) {
            (ConValue::Tuple(v), MemberKind::Tuple(Literal::Int(id))) => v
                .get(*id as usize)
                .cloned()
                .ok_or(Error::OobIndex(*id as usize, v.len())),
            (ConValue::Struct(parts), MemberKind::Struct(name)) => {
                parts.1.get(name).cloned().ok_or(Error::NotDefined(*name))
            }
            (ConValue::Struct(parts), MemberKind::Call(name, args)) => {
                let mut values = vec![];
                for arg in &args.exprs {
                    values.push(arg.interpret(env)?);
                }
                (parts.1)
                    .get(name)
                    .cloned()
                    .ok_or(Error::NotDefined(*name))?
                    .call(env, &values)
            }
            (head, MemberKind::Call(name, args)) => {
                let mut values = vec![head];
                for arg in &args.exprs {
                    values.push(arg.interpret(env)?);
                }
                env.call(*name, &values)
            }
            _ => Err(Error::TypeError)?,
        }
    }
}
impl Interpret for Index {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { head, indices } = self;
        let mut head = head.interpret(env)?;
        for index in indices {
            head = head.index(&index.interpret(env)?)?;
        }
        Ok(head)
    }
}
impl Interpret for Structor {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { to: Path { absolute: _, parts }, init } = self;
        use std::collections::HashMap;

        let name = match parts.last() {
            Some(PathPart::Ident(name)) => *name,
            Some(PathPart::SelfKw) => "self".into(),
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
        Ok(ConValue::Struct(Box::new((name, map))))
    }
}

impl Interpret for Path {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { absolute: _, parts } = self;

        assignment::addrof_path(env, parts)
            .cloned()
            .transpose()
            .ok_or_else(|| Error::NotInitialized(format!("{self}").into()))?
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
        let repeat = match repeat.interpret(env)? {
            ConValue::Int(v) => v,
            _ => Err(Error::TypeError)?,
        };
        let value = value.interpret(env)?;
        Ok(ConValue::Array(vec![value; repeat as usize].into()))
    }
}
impl Interpret for AddrOf {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { mutable: _, expr } = self;
        match expr.as_ref() {
            ExprKind::Index(_) => todo!("AddrOf array index"),
            ExprKind::Path(_) => todo!("Path traversal in addrof"),
            _ => Ok(ConValue::Ref(Rc::new(expr.interpret(env)?))),
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
                    Err(Error::Break(value)) => return Ok(value),
                    Err(Error::Continue) => continue,
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
        let Self { bind: name, cond, pass, fail } = self;
        let cond = cond.interpret(env)?;
        // TODO: A better iterator model
        let mut bounds: Box<dyn Iterator<Item = ConValue>> = match &cond {
            &ConValue::RangeExc(a, b) => Box::new((a..b).map(ConValue::Int)),
            &ConValue::RangeInc(a, b) => Box::new((a..=b).map(ConValue::Int)),
            ConValue::Array(a) => Box::new(a.iter().cloned()),
            ConValue::String(s) => Box::new(s.chars().map(ConValue::Char)),
            _ => Err(Error::TypeError)?,
        };
        loop {
            let mut env = env.frame("loop variable");
            if let Some(loop_var) = bounds.next() {
                let subs = assignment::pattern_substitution(name, loop_var)?;
                for (name, value) in subs {
                    env.insert(*name, Some(value));
                }
                match pass.interpret(&mut env) {
                    Err(Error::Break(value)) => return Ok(value),
                    Err(Error::Continue) => continue,
                    result => result?,
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
