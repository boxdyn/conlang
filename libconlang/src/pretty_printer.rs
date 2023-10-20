use super::ast::preamble::*;
use std::{
    fmt::Display,
    io::{stdout, Result as IOResult, StdoutLock, Write},
};
pub trait PrettyPrintable {
    fn print(&self);
    fn write(&self, into: impl Write) -> IOResult<()>;
}
impl PrettyPrintable for Start {
    fn print(&self) {
        let _ = self.walk(&mut Printer::default());
    }
    fn write(&self, into: impl Write) -> IOResult<()> {
        self.walk(&mut Printer::from(into))
    }
}

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
    fn visit_binary<F, Op>(&mut self, expr: &math::Binary<F, (Op, F)>) -> IOResult<()>
    where
        F: Walk<Self, IOResult<()>>,
        Op: Walk<Self, IOResult<()>>,
    {
        expr.first().walk(self)?;
        for (op, target) in expr.other() {
            op.walk(self)?;
            target.walk(self)?;
        }
        Ok(())
    }

    fn visit_unary(&mut self, expr: &math::Unary) -> IOResult<()> {
        for op in &expr.0 {
            op.walk(self)?;
        }
        expr.1.walk(self)
    }
    fn visit_ignore_op(&mut self, _op: &operator::Ignore) -> IOResult<()> {
        self.put(";")?.newline().map(drop)
    }
    fn visit_compare_op(&mut self, op: &operator::Compare) -> IOResult<()> {
        visit_operator!(self.match op {
            operator::Compare::Less => "<",
            operator::Compare::LessEq => "<=",
            operator::Compare::Equal => "==",
            operator::Compare::NotEq => "!=",
            operator::Compare::GreaterEq => ">=",
            operator::Compare::Greater => ">",
        })
    }
    fn visit_assign_op(&mut self, op: &operator::Assign) -> IOResult<()> {
        visit_operator!( self.match op {
            operator::Assign::Assign => "=",
            operator::Assign::AddAssign => "+=",
            operator::Assign::SubAssign => "-=",
            operator::Assign::MulAssign => "*=",
            operator::Assign::DivAssign => "/=",
            operator::Assign::BitAndAssign => "&=",
            operator::Assign::BitOrAssign => "|=",
            operator::Assign::BitXorAssign => "^=",
            operator::Assign::ShlAssign => "<<=",
            operator::Assign::ShrAssign => ">>=",
        })
    }
    fn visit_logic_op(&mut self, op: &operator::Logic) -> IOResult<()> {
        visit_operator!(self.match op {
            operator::Logic::LogAnd => "&&",
            operator::Logic::LogOr => "||",
            operator::Logic::LogXor => "^^",
        })
    }
    fn visit_bitwise_op(&mut self, op: &operator::Bitwise) -> IOResult<()> {
        visit_operator!(self.match op {
            operator::Bitwise::BitAnd => "&",
            operator::Bitwise::BitOr => "|",
            operator::Bitwise::BitXor => "^",
        })
    }
    fn visit_shift_op(&mut self, op: &operator::Shift) -> IOResult<()> {
        visit_operator!(self.match op {
            operator::Shift::Lsh => "<<",
            operator::Shift::Rsh => ">>",
        })
    }
    fn visit_term_op(&mut self, op: &operator::Term) -> IOResult<()> {
        visit_operator!(self.match op {
            operator::Term::Add => "+",
            operator::Term::Sub => "-",
        })
    }
    fn visit_factor_op(&mut self, op: &operator::Factor) -> IOResult<()> {
        visit_operator!(self.match op {
            operator::Factor::Mul => "*",
            operator::Factor::Div => "/",
            operator::Factor::Rem => "%",
        })
    }
    fn visit_unary_op(&mut self, op: &operator::Unary) -> IOResult<()> {
        self.put(match op {
            operator::Unary::RefRef => "&&",
            operator::Unary::Deref => "*",
            operator::Unary::Ref => "&",
            operator::Unary::Neg => "-",
            operator::Unary::Not => "!",
            operator::Unary::At => "@",
            operator::Unary::Hash => "#",
            operator::Unary::Tilde => "~",
        })
        .map(drop)
    }

    fn visit_if(&mut self, expr: &control::If) -> IOResult<()> {
        expr.cond.walk(self.put("if")?.space()?)?;
        expr.body.walk(self.space()?)?;
        if let Some(e) = &expr.else_ {
            e.walk(self)?
        }
        Ok(())
    }
    fn visit_while(&mut self, expr: &control::While) -> IOResult<()> {
        expr.cond.walk(self.put("while")?.space()?)?;
        expr.body.walk(self.space()?)?;
        if let Some(e) = &expr.else_ {
            e.walk(self)?
        }
        Ok(())
    }
    fn visit_for(&mut self, expr: &control::For) -> IOResult<()> {
        expr.var.walk(self.put("for")?.space()?)?;
        expr.iter.walk(self.space()?.put("in")?.space()?)?;
        expr.body.walk(self.space()?)?;
        if let Some(e) = &expr.else_ {
            e.walk(self)?
        }
        Ok(())
    }
    fn visit_else(&mut self, expr: &control::Else) -> IOResult<()> {
        expr.block.walk(self.space()?.put("else")?.space()?)
    }
    fn visit_continue(&mut self, _expr: &control::Continue) -> IOResult<()> {
        self.put("continue").map(drop)
    }
    fn visit_break(&mut self, expr: &control::Break) -> IOResult<()> {
        expr.expr.walk(self.put("break")?.space()?)
    }
    fn visit_return(&mut self, expr: &control::Return) -> IOResult<()> {
        expr.expr.walk(self.put("return")?.space()?)
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
        self.put("").map(drop)
    }

    fn visit_block(&mut self, expr: &expression::Block) -> IOResult<()> {
        self.put('{')?;
        match &expr.expr {
            Some(expr) => {
                expr.walk(self.indent().newline()?)?;
                self.dedent().newline()?;
            }
            None => ().walk(self.space()?)?,
        }
        self.put('}').map(drop)
    }

    fn visit_group(&mut self, expr: &expression::Group) -> IOResult<()> {
        self.put('(')?.space()?;
        expr.walk(self)?;
        self.space()?.put(')').map(drop)
    }
}
