//! A [Printer] pretty-prints a Conlang [syntax tree](crate::ast)

use super::ast::preamble::*;
use std::{
    fmt::Display,
    io::{stdout, Result as IOResult, StdoutLock, Write},
};
/// Prettily prints this node
pub trait PrettyPrintable {
    /// Prettily prints this node
    fn print(&self) {
        let _ = self.visit(&mut Printer::default());
    }
    /// Prettily writes this node into the given [Writer](Write)
    fn write(&self, into: impl Write) -> IOResult<()> {
        self.visit(&mut Printer::from(into))
    }
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()>;
}

/// Prints a Conlang [syntax tree](crate::ast) into a [Writer](Write)
#[derive(Debug)]
pub struct Printer<W: Write> {
    level: u32,
    writer: W,
}

impl<W: Write> Write for Printer<W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> IOResult<usize> {
        self.writer.write(buf)
    }
    #[inline]
    fn flush(&mut self) -> IOResult<()> {
        self.writer.flush()
    }
}

impl<'t> Default for Printer<StdoutLock<'t>> {
    fn default() -> Self {
        Self { level: 0, writer: stdout().lock() }
    }
}
impl<W: Write> From<W> for Printer<W> {
    fn from(writer: W) -> Self {
        Self { level: 0, writer }
    }
}
impl<W: Write> Printer<W> {
    fn pad(&mut self) -> IOResult<&mut Self> {
        for _ in 0..self.level * 4 {
            write!(self.writer, " ")?;
        }
        Ok(self)
    }
    fn newline(&mut self) -> IOResult<&mut Self> {
        writeln!(self.writer)?;
        self.pad()
    }
    fn put(&mut self, d: impl Display) -> IOResult<&mut Self> {
        write!(self.writer, "{d}")?;
        Ok(self)
    }
    fn space(&mut self) -> IOResult<&mut Self> {
        write!(self.writer, " ").map(|_| self)
    }
    /// Increase the indentation level by 1
    fn indent(&mut self) -> &mut Self {
        self.level += 1;
        self
    }
    fn dedent(&mut self) -> &mut Self {
        self.level -= 1;
        self
    }
}
macro visit_operator($self:ident.$op:expr) {
    $self.space()?.put($op)?.space().map(drop)
}

impl PrettyPrintable for Start {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let Self(program) = self;
        program.visit(p)
    }
}
impl PrettyPrintable for Program {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let Self(module) = self;
        for decl in module {
            decl.visit(p)?;
            p.newline()?;
        }
        Ok(())
    }
}
impl PrettyPrintable for Stmt {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        match self {
            Stmt::Let(value) => value.visit(p),
            Stmt::Fn(value) => value.visit(p),
            Stmt::Expr(value) => {
                value.visit(p)?;
                p.put(';')?.newline().map(drop)
            }
        }
    }
}
impl PrettyPrintable for Let {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let Let { name: Name { name, mutable, ty }, init } = self;
        p.put("let")?.space()?;
        if *mutable {
            p.put("mut")?.space()?;
        }
        name.visit(p)?;
        if let Some(ty) = ty {
            ty.visit(p.put(':')?.space()?)?
        }
        if let Some(init) = init {
            init.visit(p.space()?.put('=')?.space()?)?;
        }
        p.put(';').map(drop)
    }
}
impl PrettyPrintable for FnDecl {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let FnDecl { name, args, body } = self;
        p.put("fn")?.space()?;
        name.visit(p)?;
        p.space()?.put('(')?;
        for (idx, arg) in args.iter().enumerate() {
            if idx > 0 {
                p.put(',')?.space()?;
            }
            arg.visit(p)?;
        }
        p.put(')')?.space()?;
        body.visit(p)
    }
}
impl PrettyPrintable for Name {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        if self.mutable {
            p.put("mut")?.space()?;
        }
        self.name.visit(p)?;
        if let Some(ty) = &self.ty {
            ty.visit(p.put(':')?.space()?)?;
        }
        Ok(())
    }
}
impl PrettyPrintable for TypeExpr {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        match self {
            TypeExpr::TupleType(_tt) => todo!(),
            TypeExpr::TypePath(t) => t.visit(p),
            TypeExpr::Empty(_) => p.put("()").map(drop),
            TypeExpr::Never(_) => p.put('!').map(drop),
        }
    }
}
impl PrettyPrintable for Path {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let Path { absolute, parts } = self;
        if *absolute {
            p.put("::")?;
        }
        for (idx, part) in parts.iter().enumerate() {
            if idx != 0 { p.put("::")?;}
            part.visit(p)?;
        }
        Ok(())
    }
}
impl PrettyPrintable for PathPart {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        match self {
            PathPart::PathSuper => p.put("super").map(drop),
            PathPart::PathSelf => p.put("self").map(drop),
            PathPart::PathIdent(id) =>id.visit(p),
        }
    }
}
impl PrettyPrintable for Block {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let Block { let_count: _, statements, expr } = self;
        p.put('{')?.indent().newline()?;
        for stmt in statements {
            stmt.visit(p)?;
        }
        for expr in expr {
            expr.visit(p)?;
        }
        p.dedent().newline()?.put('}').map(drop)
    }
}
impl PrettyPrintable for Expr {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let Expr(expr) = self;
        expr.visit(p)
    }
}

impl PrettyPrintable for Operation {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        match self {
            Operation::Assign(value) => value.visit(p),
            Operation::Binary(value) => value.visit(p),
            Operation::Unary(value) => value.visit(p),
            Operation::Call(value) => value.visit(p),
        }
    }
}
impl PrettyPrintable for Assign {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let Assign { target, operator, init } = self;
        target.visit(p)?;
        operator.visit(p)?;
        init.visit(p)
    }
}
impl PrettyPrintable for operator::Assign {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        use operator::Assign;
        visit_operator!(p.match self {
            Assign::Assign => "=",
            Assign::AddAssign => "+=",
            Assign::SubAssign => "-=",
            Assign::MulAssign => "*=",
            Assign::DivAssign => "/=",
            Assign::RemAssign => "%=",
            Assign::BitAndAssign => "&=",
            Assign::BitOrAssign => "|=",
            Assign::BitXorAssign => "^=",
            Assign::ShlAssign => "<<=",
            Assign::ShrAssign => ">>=",
        })
    }
}
impl PrettyPrintable for Binary {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let Binary { first, other } = self;
        p.put('(')?;
        first.visit(p)?;
        for (op, other) in other {
            op.visit(p)?;
            other.visit(p)?
        }
        p.put(')').map(drop)
    }
}
impl PrettyPrintable for operator::Binary {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        use operator::Binary;
        visit_operator!(p.match self {
            Binary::Mul => "*",
            Binary::Div => "/",
            Binary::Rem => "%",
            Binary::Add => "+",
            Binary::Sub => "-",
            Binary::Lsh => "<<",
            Binary::Rsh => ">>",
            Binary::BitAnd => "&",
            Binary::BitOr => "|",
            Binary::BitXor => "^",
            Binary::LogAnd => "&&",
            Binary::LogOr => "||",
            Binary::LogXor => "^^",
            Binary::RangeExc => "..",
            Binary::RangeInc => "..=",
            Binary::Less => "<",
            Binary::LessEq => "<=",
            Binary::Equal => "==",
            Binary::NotEq => "!=",
            Binary::GreaterEq => ">=",
            Binary::Greater => ">",
        })
    }
}
impl PrettyPrintable for Unary {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let Unary { operators, operand } = self;
        for op in operators {
            op.visit(p)?;
        }
        operand.visit(p)
    }
}
impl PrettyPrintable for operator::Unary {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        use operator::Unary;
        p.put(match self {
            Unary::RefRef => "&&",
            Unary::Deref => "*",
            Unary::Ref => "&",
            Unary::Neg => "-",
            Unary::Not => "!",
            Unary::At => "@",
            Unary::Hash => "#",
            Unary::Tilde => "~",
        })
        .map(drop)
    }
}

impl PrettyPrintable for Call {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        match self {
            Call::FnCall(value) => value.visit(p),
            Call::Primary(value) => value.visit(p),
        }
    }
}
impl PrettyPrintable for FnCall {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let FnCall { callee, args } = self;
        callee.visit(p)?;
        for arg_list in args {
            p.put('(')?;
            arg_list.visit(p)?;
            p.put(')')?;
        }
        Ok(())
    }
}
impl PrettyPrintable for Primary {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        match self {
            Primary::Identifier(value) => value.visit(p),
            Primary::Literal(value) => value.visit(p),
            Primary::Block(value) => value.visit(p),
            Primary::Group(value) => value.visit(p),
            Primary::Branch(value) => value.visit(p),
        }
    }
}
impl PrettyPrintable for Group {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        p.put('(')?;
        match self {
            Group::Tuple(tuple) => tuple.visit(p.space()?)?,
            Group::Single(expr) => expr.visit(p.space()?)?,
            Group::Empty => (),
        };
        p.space()?.put(')').map(drop)
    }
}
impl PrettyPrintable for Tuple {
    /// Writes a *non-delimited* [Tuple] to the [Printer]
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let Tuple { elements } = self;
        for (idx, expr) in elements.iter().enumerate() {
            if idx > 0 {
                p.put(',')?.space()?;
            }
            expr.visit(p)?;
        }
        Ok(())
    }
}

impl PrettyPrintable for Identifier {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let Identifier { name, index: _ } = self; // TODO: Pretty-print variable number as well
        p.put(name).map(drop)
    }
}
impl PrettyPrintable for Literal {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        match self {
            Literal::String(value) => write!(p, "\"{value}\""),
            Literal::Char(value) => write!(p, "'{value}'"),
            Literal::Bool(value) => write!(p, "{value}"),
            Literal::Float(value) => value.visit(p),
            Literal::Int(value) => write!(p, "{value}"),
        }
    }
}
impl PrettyPrintable for Float {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let Float { sign, exponent, mantissa } = self;
        p.put(sign)?.put(exponent)?.put(mantissa).map(drop)
    }
}

impl PrettyPrintable for Flow {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        match self {
            Flow::While(value) => value.visit(p),
            Flow::If(value) => value.visit(p),
            Flow::For(value) => value.visit(p),
            Flow::Continue(value) => value.visit(p),
            Flow::Return(value) => value.visit(p),
            Flow::Break(value) => value.visit(p),
        }
    }
}
impl PrettyPrintable for While {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let While { cond, body, else_ } = self;
        cond.visit(p.put("while")?.space()?)?;
        body.visit(p.space()?)?;
        match else_ {
            Some(e) => e.visit(p),
            None => Ok(()),
        }
    }
}
impl PrettyPrintable for If {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let If { cond, body, else_ } = self;
        cond.visit(p.put("if")?.space()?)?;
        body.visit(p.space()?)?;
        match else_ {
            Some(e) => e.visit(p),
            None => Ok(()),
        }
    }
}
impl PrettyPrintable for For {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let For { var, iter, body, else_ } = self;
        var.visit(p.put("for")?.space()?)?;
        iter.visit(p.space()?.put("in")?.space()?)?;
        body.visit(p.space()?)?;
        match else_ {
            Some(e) => e.visit(p),
            None => Ok(()),
        }
    }
}
impl PrettyPrintable for Else {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let Else { expr } = self;
        expr.visit(p.space()?.put("else")?.space()?)
    }
}
impl PrettyPrintable for Continue {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let control::Continue = self; // using pattern destructuring, rather than assigning to "Continue"
        p.put("continue").map(drop)
    }
}
impl PrettyPrintable for Break {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let Break { expr } = self;
        expr.visit(p.put("break")?.space()?)
    }
}
impl PrettyPrintable for Return {
    fn visit<W: Write>(&self, p: &mut Printer<W>) -> IOResult<()> {
        let Return { expr } = self;
        expr.visit(p.put("return")?.space()?)
    }
}
