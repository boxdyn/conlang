//! A work-in-progress tree walk interpreter for Conlang
//!
//! Currently, major parts of the interpreter are not yet implemented, and major parts will never be
//! implemented in its current form. Namely, since no [ConValue] has a stable location, it's
//! meaningless to get a pointer to one, and would be undefined behavior to dereference a pointer to
//! one in any situation.

use std::{borrow::Borrow, rc::Rc};

use super::*;
use cl_ast::*;
/// A work-in-progress tree walk interpreter for Conlang
pub trait Interpret {
    /// Interprets this thing in the given [`Environment`].
    ///
    /// Everything returns a value!™
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue>;
}

impl Interpret for File {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        for item in &self.items {
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
            ItemKind::Use(_) => todo!("namespaces and imports in the interpreter"),
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
        // TODO: Enter this module's namespace
        match kind {
            ModuleKind::Inline(file) => file.interpret(env),
            ModuleKind::Outline => Err(Error::Outlined(*name)),
        }
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
        println!("TODO: {self}");
        let Self { target: _, body } = self;
        body.interpret(env)
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
impl Interpret for Let {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Let { mutable: _, name, ty: _, init } = self;
        let init = init.as_ref().map(|i| i.interpret(env)).transpose()?;
        env.insert(*name, init);
        Ok(ConValue::Empty)
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
            ExprKind::Let(v) => v.interpret(env),
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

fn evaluate_place_expr<'e>(
    env: &'e mut Environment,
    expr: &ExprKind,
) -> IResult<(&'e mut Option<ConValue>, Sym)> {
    match expr {
        ExprKind::Path(Path { parts, .. }) if parts.len() == 1 => {
            match parts.last().expect("parts should not be empty") {
                PathPart::SuperKw => Err(Error::NotAssignable),
                PathPart::SelfKw => todo!("Assignment to `self`"),
                PathPart::SelfTy => todo!("What does it mean to assign to capital-S Self?"),
                PathPart::Ident(s) => env.get_mut(*s).map(|v| (v, *s)),
            }
        }
        ExprKind::Index(_) => todo!("Assignment to an index operation"),
        ExprKind::Path(_) => todo!("Path expression resolution (IMPORTANT)"),
        ExprKind::Empty | ExprKind::Group(_) | ExprKind::Tuple(_) => {
            todo!("Pattern Destructuring?")
        }
        _ => Err(Error::NotAssignable),
    }
}

impl Interpret for Assign {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Assign { parts } = self;
        let (head, tail) = parts.borrow();
        let init = tail.interpret(env)?;
        // Resolve the head pattern
        let target = evaluate_place_expr(env, head)?;
        use std::mem::discriminant as variant;
        // runtime typecheck
        match target.0 {
            Some(value) if variant(value) == variant(&init) => {
                *value = init;
            }
            value @ None => *value = Some(init),
            _ => Err(Error::TypeError)?,
        }
        Ok(ConValue::Empty)
    }
}
impl Interpret for Modify {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Modify { kind: op, parts } = self;
        let (head, tail) = parts.borrow();
        // Get the initializer and the tail
        let init = tail.interpret(env)?;
        // Resolve the head pattern
        let target = evaluate_place_expr(env, head)?;
        let (Some(target), _) = target else {
            return Err(Error::NotInitialized(target.1));
        };

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
        Ok(ConValue::Struct(Rc::new((name, map))))
    }
}

impl Interpret for Path {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { absolute: _, parts } = self;

        if parts.len() == 1 {
            match parts.last().expect("parts should not be empty") {
                PathPart::SuperKw | PathPart::SelfKw => todo!("Path navigation"),
                PathPart::SelfTy => todo!("Path navigation to Self"),
                PathPart::Ident(name) => env.get(*name),
            }
        } else {
            todo!("Path navigation!")
        }
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
        let Self { count: _, mutable: _, expr } = self;
        match expr.as_ref() {
            ExprKind::Index(_) => todo!("AddrOf array index"),
            // ExprKind::Path(Path { absolute: false, parts }) => match parts.as_slice() {
            //     [PathPart::Ident(id)] => env.get_ref(id),
            //     _ => todo!("Path traversal in addrof"),
            // },
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
        // TODO: A better iterator model
        let mut bounds = match cond.interpret(env)? {
            ConValue::RangeExc(a, b) => a..=b,
            ConValue::RangeInc(a, b) => a..=b,
            _ => Err(Error::TypeError)?,
        };
        loop {
            let mut env = env.frame("loop variable");
            if let Some(loop_var) = bounds.next() {
                env.insert(*name, Some(loop_var.into()));
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
