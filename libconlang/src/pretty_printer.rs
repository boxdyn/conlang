//! A [Printer] pretty-prints a Conlang [syntax tree](crate::ast)
use super::ast::preamble::*;
use std::{
    fmt::Display,
    io::{stdout, Result as IOResult, StdoutLock, Write},
};
/// Prettily prints this node
pub trait PrettyPrintable {
    /// Prettily prints this node
    fn print(&self);
    /// Prettily writes this node into the given [Writer](Write)
    fn write(&self, into: impl Write) -> IOResult<()>;
}
impl PrettyPrintable for Start {
    fn print(&self) {
        let _ = Printer::default().visit(self);
    }
    fn write(&self, into: impl Write) -> IOResult<()> {
        Printer::from(into).visit(self)
    }
}

/// Prints a Conlang [syntax tree](crate::ast) into a [Writer](Write)
#[derive(Debug)]
pub struct Printer<W: Write> {
    level: u32,
    writer: W,
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
impl<W: Write> Visitor<IOResult<()>> for Printer<W> {
    fn visit_program(&mut self, prog: &Program) -> IOResult<()> {
        // delegate to the walker
        for stmt in &prog.0 {
            self.visit_statement(stmt)?;
        }
        Ok(())
    }
    fn visit_statement(&mut self, stmt: &Stmt) -> IOResult<()> {
        match stmt {
            Stmt::Let(stmt) => self.visit_let(stmt)?,
            Stmt::Expr(e) => {
                self.visit_expr(e)?;
                self.put(';').map(drop)?
            }
        }
        self.newline().map(drop)
    }
    fn visit_let(&mut self, stmt: &Let) -> IOResult<()> {
        let Let { name, mutable, ty, init } = stmt;
        self.put("let")?.space()?;
        if *mutable {
            self.put("mut")?.space()?;
        }
        self.visit_identifier(name)?;
        if let Some(ty) = ty {
            self.put(':')?.space()?.visit_identifier(ty)?;
        }
        if let Some(init) = init {
            self.space()?.put('=')?.space()?.visit_expr(init)?;
        }
        self.put(';').map(drop)
    }

    fn visit_assign(&mut self, assign: &math::Assign) -> IOResult<()> {
        let math::Assign { target, operator, init } = assign;
        self.visit_identifier(target)?;
        self.visit_assign_op(operator)?;
        self.visit_operation(init)
    }
    fn visit_binary(&mut self, binary: &math::Binary) -> IOResult<()> {
        let math::Binary { first, other } = binary;
        self.put('(')?.visit_operation(first)?;
        for (op, other) in other {
            self.visit_binary_op(op)?;
            self.visit_operation(other)?;
        }
        self.put(')').map(drop)
    }
    fn visit_unary(&mut self, unary: &math::Unary) -> IOResult<()> {
        let math::Unary { operators, operand } = unary;
        for op in operators {
            self.visit_unary_op(op)?;
        }
        self.visit_operation(operand)
    }

    fn visit_assign_op(&mut self, op: &operator::Assign) -> IOResult<()> {
        use operator::Assign;
        visit_operator!(self.match op {
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
    fn visit_binary_op(&mut self, op: &operator::Binary) -> IOResult<()> {
        use operator::Binary;
        visit_operator!(self.match op {
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
    fn visit_unary_op(&mut self, op: &operator::Unary) -> IOResult<()> {
        use operator::Unary;
        self.put(match op {
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
    fn visit_if(&mut self, expr: &control::If) -> IOResult<()> {
        self.put("while")?.space()?.visit_expr(&expr.cond)?;
        self.space()?.visit_block(&expr.body)?;
        match &expr.else_ {
            Some(e) => self.visit_else(e),
            None => Ok(()),
        }
    }
    fn visit_while(&mut self, expr: &control::While) -> IOResult<()> {
        self.put("while")?.space()?.visit_expr(&expr.cond)?;
        self.space()?.visit_block(&expr.body)?;
        match &expr.else_ {
            Some(e) => self.visit_else(e),
            None => Ok(()),
        }
    }
    fn visit_for(&mut self, expr: &control::For) -> IOResult<()> {
        self.put("for")?.space()?.visit_identifier(&expr.var)?;
        self.space()?.put("in")?.space()?.visit_expr(&expr.iter)?;
        self.space()?.visit_block(&expr.body)?;
        match &expr.else_ {
            Some(e) => self.visit_else(e),
            None => Ok(()),
        }
    }
    fn visit_else(&mut self, expr: &control::Else) -> IOResult<()> {
        self.space()?.put("else")?.space()?.visit_block(&expr.block)
    }
    fn visit_continue(&mut self, _: &control::Continue) -> IOResult<()> {
        self.put("continue").map(drop)
    }
    fn visit_break(&mut self, brk: &control::Break) -> IOResult<()> {
        self.put("break")?.space()?.visit_expr(&brk.expr)
    }
    fn visit_return(&mut self, ret: &control::Return) -> IOResult<()> {
        self.put("return")?.space()?.visit_expr(&ret.expr)
    }

    fn visit_identifier(&mut self, ident: &Identifier) -> IOResult<()> {
        self.put(&ident.0).map(drop)
    }
    fn visit_string_literal(&mut self, string: &str) -> IOResult<()> {
        self.put("\"")?.put(string)?.put("\"").map(drop)
    }
    fn visit_char_literal(&mut self, char: &char) -> IOResult<()> {
        self.put("'")?.put(char)?.put("'").map(drop)
    }
    fn visit_bool_literal(&mut self, bool: &bool) -> IOResult<()> {
        self.put(bool).map(drop)
    }
    fn visit_float_literal(&mut self, float: &literal::Float) -> IOResult<()> {
        self.put(float.sign)?
            .put(float.exponent)?
            .put(float.mantissa)
            .map(drop)
    }
    fn visit_int_literal(&mut self, int: &u128) -> IOResult<()> {
        self.put(int).map(drop)
    }
    fn visit_empty(&mut self) -> IOResult<()> {
        self.put("()").map(drop)
    }

    fn visit_block(&mut self, block: &expression::Block) -> IOResult<()> {
        self.put('{')?.indent().newline()?;
        for stmt in &block.statements {
            self.visit_statement(stmt)?;
        }
        for expr in &block.expr {
            self.visit_expr(expr)?;
        }
        self.dedent().newline()?.put('}').map(drop)
    }

    fn visit_group(&mut self, expr: &expression::Group) -> IOResult<()> {
        match expr {
            expression::Group::Expr(expr) => {
                self.put('(')?.space()?;
                self.visit_expr(expr)?;
                self.space()?.put(')').map(drop)
            }
            expression::Group::Empty => self.visit_empty(),
        }
    }
}
