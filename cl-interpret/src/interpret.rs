//! A work-in-progress tree walk interpreter for Conlang
//!
//! Currently, major parts of the interpreter are not yet implemented, and major parts will never be
//! implemented in its current form. Namely, since no [ConValue] has a stable location, it's
//! meaningless to get a pointer to one, and would be undefined behavior to dereference a pointer to
//! one in any situation.

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
        }
    }
}
impl Interpret for Alias {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        todo!("Interpret type alias in {env}")
    }
}
impl Interpret for Const {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        todo!("interpret const in {env}")
    }
}
impl Interpret for Static {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        todo!("interpret static in {env}")
    }
}
impl Interpret for Module {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        // TODO: Enter this module's namespace
        match &self.kind {
            ModuleKind::Inline(file) => file.interpret(env),
            ModuleKind::Outline => todo!("Load and parse external files"),
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
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        todo!("Interpret structs in {env}")
    }
}
impl Interpret for Enum {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        todo!("Interpret enums in {env}")
    }
}
impl Interpret for Impl {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        todo!("Enter a struct's namespace and insert function definitions into it in {env}");
    }
}
impl Interpret for Stmt {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { extents: _, kind, semi } = self;
        let out = match kind {
            StmtKind::Empty => ConValue::Empty,
            StmtKind::Local(stmt) => stmt.interpret(env)?,
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
        let Let { mutable: _, name: Identifier(name), ty: _, init } = self;
        let init = init.as_ref().map(|i| i.interpret(env)).transpose()?;
        env.insert(name, init);
        Ok(ConValue::Empty)
    }
}
impl Interpret for Expr {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { extents: _, kind } = self;
        match kind {
            ExprKind::Assign(v) => v.interpret(env),
            ExprKind::Binary(v) => v.interpret(env),
            ExprKind::Unary(v) => v.interpret(env),
            ExprKind::Member(v) => v.interpret(env),
            ExprKind::Call(v) => v.interpret(env),
            ExprKind::Index(v) => v.interpret(env),
            ExprKind::Path(v) => v.interpret(env),
            ExprKind::Literal(v) => v.interpret(env),
            ExprKind::Array(v) => v.interpret(env),
            ExprKind::ArrayRep(v) => v.interpret(env),
            ExprKind::AddrOf(v) => v.interpret(env),
            ExprKind::Block(v) => v.interpret(env),
            ExprKind::Empty => Ok(ConValue::Empty),
            ExprKind::Group(v) => v.interpret(env),
            ExprKind::Tuple(v) => v.interpret(env),
            ExprKind::While(v) => v.interpret(env),
            ExprKind::If(v) => v.interpret(env),
            ExprKind::For(v) => v.interpret(env),
            ExprKind::Break(v) => v.interpret(env),
            ExprKind::Return(v) => v.interpret(env),
            ExprKind::Continue(v) => v.interpret(env),
        }
    }
}
impl Interpret for Assign {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Assign { head, op, tail } = self;
        // Resolve the head pattern
        let head = match &head.kind {
            ExprKind::Path(Path { parts, .. }) if parts.len() == 1 => {
                match parts.last().expect("parts should not be empty") {
                    PathPart::SuperKw => Err(Error::NotAssignable(head.extents.head))?,
                    PathPart::SelfKw => todo!("Assignment to `self`"),
                    PathPart::Ident(Identifier(s)) => s,
                }
            }
            ExprKind::Member(_) => todo!("Member access assignment"),
            ExprKind::Call(_) => todo!("Assignment to the result of a function call?"),
            ExprKind::Index(_) => todo!("Assignment to an index operation"),
            ExprKind::Path(_) => todo!("Path expression resolution (IMPORTANT)"),
            ExprKind::Empty | ExprKind::Group(_) | ExprKind::Tuple(_) => {
                todo!("Pattern Destructuring?")
            }
            _ => Err(Error::NotAssignable(head.extents.head))?,
        };
        // Get the initializer and the tail
        let init = tail.interpret(env)?;
        let target = env.get_mut(head)?;

        if let AssignKind::Plain = op {
            use std::mem::discriminant as variant;
            // runtime typecheck
            match target {
                Some(value) if variant(value) == variant(&init) => {
                    *value = init;
                }
                value @ None => *value = Some(init),
                _ => Err(Error::TypeError)?,
            }
            return Ok(ConValue::Empty);
        }
        let Some(target) = target else {
            return Err(Error::NotInitialized(head.into()));
        };

        match op {
            AssignKind::Add => target.add_assign(init)?,
            AssignKind::Sub => target.sub_assign(init)?,
            AssignKind::Mul => target.mul_assign(init)?,
            AssignKind::Div => target.div_assign(init)?,
            AssignKind::Rem => target.rem_assign(init)?,
            AssignKind::And => target.bitand_assign(init)?,
            AssignKind::Or => target.bitor_assign(init)?,
            AssignKind::Xor => target.bitxor_assign(init)?,
            AssignKind::Shl => target.shl_assign(init)?,
            AssignKind::Shr => target.shr_assign(init)?,
            _ => (),
        }
        Ok(ConValue::Empty)
    }
}
impl Interpret for Binary {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Binary { head, tail } = self;
        let mut head = head.interpret(env)?;
        // Short-circuiting ops
        for (op, tail) in tail {
            match op {
                BinaryKind::LogAnd => {
                    if head.truthy()? {
                        head = tail.interpret(env)?;
                        continue;
                    }
                    return Ok(head); // Short circuiting
                }
                BinaryKind::LogOr => {
                    if !head.truthy()? {
                        head = tail.interpret(env)?;
                        continue;
                    }
                    return Ok(head); // Short circuiting
                }
                BinaryKind::LogXor => {
                    head = ConValue::Bool(head.truthy()? ^ tail.interpret(env)?.truthy()?);
                    continue;
                }
                _ => {}
            }
            let tail = tail.interpret(env)?;
            head = match op {
                BinaryKind::Mul => env.call("mul", &[head, tail]),
                BinaryKind::Div => env.call("div", &[head, tail]),
                BinaryKind::Rem => env.call("rem", &[head, tail]),
                BinaryKind::Add => env.call("add", &[head, tail]),
                BinaryKind::Sub => env.call("sub", &[head, tail]),
                BinaryKind::Shl => env.call("shl", &[head, tail]),
                BinaryKind::Shr => env.call("shr", &[head, tail]),
                BinaryKind::BitAnd => env.call("and", &[head, tail]),
                BinaryKind::BitOr => env.call("or", &[head, tail]),
                BinaryKind::BitXor => env.call("xor", &[head, tail]),
                BinaryKind::RangeExc => env.call("range_exc", &[head, tail]),
                BinaryKind::RangeInc => env.call("range_inc", &[head, tail]),
                BinaryKind::Lt => env.call("lt", &[head, tail]),
                BinaryKind::LtEq => env.call("lt_eq", &[head, tail]),
                BinaryKind::Equal => env.call("eq", &[head, tail]),
                BinaryKind::NotEq => env.call("neq", &[head, tail]),
                BinaryKind::GtEq => env.call("gt_eq", &[head, tail]),
                BinaryKind::Gt => env.call("gt", &[head, tail]),
                BinaryKind::Dot => todo!("search within a type's namespace!"),
                _ => Ok(head),
            }?;
        }
        Ok(head)
    }
}
impl Interpret for Unary {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Unary { tail, ops } = self;
        let mut operand = tail.interpret(env)?;

        for op in ops.iter().rev() {
            operand = match op {
                UnaryKind::Deref => env.call("deref", &[operand])?,
                UnaryKind::Neg => env.call("neg", &[operand])?,
                UnaryKind::Not => env.call("not", &[operand])?,
                UnaryKind::At => {
                    println!("{operand}");
                    operand
                }
                UnaryKind::Tilde => unimplemented!("Tilde operator"),
            };
        }
        Ok(operand)
    }
}
impl Interpret for Member {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        todo!("Interpret member accesses in {env}")
    }
}
impl Interpret for Call {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { callee, args } = self;
        // evaluate the callee
        let mut callee = callee.interpret(env)?;
        for args in args {
            let ConValue::Tuple(args) = args.interpret(env)? else {
                Err(Error::TypeError)?
            };
            callee = callee.call(env, &args)?;
        }
        Ok(callee)
    }
}
impl Interpret for Index {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { head, indices } = self;
        let mut head = head.interpret(env)?;
        for indices in indices {
            let Indices { exprs } = indices;
            for index in exprs {
                head = head.index(&index.interpret(env)?)?;
            }
        }
        Ok(head)
    }
}
impl Interpret for Path {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { absolute: _, parts } = self;

        if parts.len() == 1 {
            match parts.last().expect("parts should not be empty") {
                PathPart::SuperKw | PathPart::SelfKw => todo!("Path navigation"),
                PathPart::Ident(Identifier(s)) => env.get(s).cloned(),
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
            // Literal::Float(value) => todo!("Float values in interpreter: {value:?}"),
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
        Ok(ConValue::Array(out))
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
        Ok(ConValue::Array(vec![value; repeat as usize]))
    }
}
impl Interpret for AddrOf {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { count: _, mutable: _, expr } = self;
        // this is stupid
        todo!("Create reference\nfrom expr: {expr}\nin env:\n{env}\n")
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
        Ok(ConValue::Tuple(exprs.iter().try_fold(
            vec![],
            |mut out, element| {
                out.push(element.interpret(env)?);
                Ok(out)
            },
        )?))
    }
}
impl Interpret for While {
    fn interpret(&self, env: &mut Environment) -> IResult<ConValue> {
        let Self { cond, pass, fail } = self;
        while cond.interpret(env)?.truthy()? {
            match pass.interpret(env) {
                Err(Error::Break(value)) => return Ok(value),
                Err(Error::Continue) => continue,
                e => e?,
            };
        }
        fail.interpret(env)
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
        let Self { bind: Identifier(name), cond, pass, fail } = self;
        // TODO: A better iterator model
        let bounds = match cond.interpret(env)? {
            ConValue::RangeExc(a, b) => a..=b,
            ConValue::RangeInc(a, b) => a..=b,
            _ => Err(Error::TypeError)?,
        };
        {
            let mut env = env.frame("loop variable");
            for loop_var in bounds {
                env.insert(name, Some(loop_var.into()));
                match pass.interpret(&mut env) {
                    Err(Error::Break(value)) => return Ok(value),
                    Err(Error::Continue) => continue,
                    result => result?,
                };
            }
        }
        fail.interpret(env)
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
impl Interpret for Continue {
    fn interpret(&self, _env: &mut Environment) -> IResult<ConValue> {
        Err(Error::Continue)
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
