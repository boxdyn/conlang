//! Collects identifiers into a list

use cl_repl::repline::Repline;
use cl_structures::span::Loc;
use conlang::{lexer::Lexer, parser::Parser};
use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
    ops::{Deref, DerefMut},
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut rl = Repline::new("\x1b[33m", "cl>", "? >");
    while let Ok(line) = rl.read() {
        let mut parser = Parser::new(Lexer::new(&line));
        let code = match parser.stmt() {
            Ok(code) => {
                rl.accept();
                code
            }
            Err(_) => {
                continue;
            }
        };
        let c = Collector::from(&code);
        print!("\x1b[G\x1b[J{c}");
    }
    Ok(())
}

/// Ties the [Collector] location stack to the program stack
pub struct CollectorLoc<'loc, 'code> {
    inner: &'loc mut Collector<'code>,
}
impl<'l, 'c> CollectorLoc<'l, 'c> {
    pub fn new(c: &'l mut Collector<'c>, loc: Loc) -> Self {
        c.location.push(loc);
        Self { inner: c }
    }
}
impl<'l, 'c> Deref for CollectorLoc<'l, 'c> {
    type Target = Collector<'c>;
    fn deref(&self) -> &Self::Target {
        self.inner
    }
}
impl<'l, 'c> DerefMut for CollectorLoc<'l, 'c> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}
impl<'l, 'c> Drop for CollectorLoc<'l, 'c> {
    fn drop(&mut self) {
        let Self { inner: c } = self;
        c.location.pop();
    }
}

#[derive(Clone, Debug, Default)]
pub struct Collector<'code> {
    location: Vec<Loc>,
    defs: HashMap<&'code str, Vec<Loc>>,
    refs: HashMap<&'code str, Vec<Loc>>,
}

impl<'c> Collector<'c> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn location<'loc>(&'loc mut self, loc: Loc) -> CollectorLoc<'loc, 'c> {
        CollectorLoc::new(self, loc)
    }
    pub fn collect(&mut self, collectible: &'c impl Collectible<'c>) -> &mut Self {
        collectible.collect(self);
        self
    }
    pub fn define(&mut self, name: &'c str) -> &mut Self {
        // println!("Inserted definition of {name}");
        let loc = self.location.last().copied().unwrap_or(Loc(0, 0));
        self.defs
            .entry(name)
            .and_modify(|c| c.push(loc))
            .or_insert_with(|| vec![loc]);
        self
    }
    pub fn reference(&mut self, name: &'c str) -> &mut Self {
        // println!("Inserted usage of {name}");
        let loc = self.location.last().copied().unwrap_or(Loc(0, 0));
        self.refs
            .entry(name)
            .and_modify(|c| c.push(loc))
            .or_insert_with(|| vec![loc]);
        self
    }
}
impl<'c> Display for Collector<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { location: _, defs, refs } = self;
        writeln!(f, "Definitions:")?;
        for (name, locs) in defs {
            for loc in locs {
                writeln!(f, "{loc} {name}")?;
            }
        }
        writeln!(f, "Usages:")?;
        for (name, locs) in refs {
            for loc in locs {
                writeln!(f, "{loc} {name}")?;
            }
        }
        Ok(())
    }
}

impl<'c, C: Collectible<'c>> From<&'c C> for Collector<'c> {
    fn from(value: &'c C) -> Self {
        let mut c = Self::new();
        c.collect(value);
        c
    }
}

use collectible::Collectible;
pub mod collectible {

    use super::Collector;
    use conlang::ast::*;
    pub trait Collectible<'code> {
        fn collect(&'code self, c: &mut Collector<'code>);
    }

    impl<'c> Collectible<'c> for File {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let File { items } = self;
            for item in items {
                item.collect(c)
            }
        }
    }
    impl<'c> Collectible<'c> for Item {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { extents, attrs: _, vis: _, kind } = self;
            kind.collect(&mut c.location(extents.head));
        }
    }
    impl<'c> Collectible<'c> for ItemKind {
        fn collect(&'c self, c: &mut Collector<'c>) {
            match self {
                ItemKind::Alias(f) => f.collect(c),
                ItemKind::Const(f) => f.collect(c),
                ItemKind::Static(f) => f.collect(c),
                ItemKind::Module(f) => f.collect(c),
                ItemKind::Function(f) => f.collect(c),
                ItemKind::Struct(f) => f.collect(c),
                ItemKind::Enum(f) => f.collect(c),
                ItemKind::Impl(f) => f.collect(c),
            }
        }
    }
    impl<'c> Collectible<'c> for Alias {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { to, from } = self;
            c.collect(to).collect(from);
        }
    }
    impl<'c> Collectible<'c> for Const {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { name: Identifier(name), ty, init } = self;
            c.define(name).collect(init).collect(ty);
        }
    }
    impl<'c> Collectible<'c> for Static {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { mutable: _, name: Identifier(name), ty, init } = self;
            c.define(name).collect(init).collect(ty);
        }
    }
    impl<'c> Collectible<'c> for Module {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { name: Identifier(name), kind } = self;
            c.define(name).collect(kind);
        }
    }
    impl<'c> Collectible<'c> for ModuleKind {
        fn collect(&'c self, c: &mut Collector<'c>) {
            match self {
                ModuleKind::Inline(f) => f.collect(c),
                ModuleKind::Outline => {}
            }
        }
    }
    impl<'c> Collectible<'c> for Function {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { name: Identifier(name), args, body, rety } = self;
            c.define(name).collect(args).collect(body).collect(rety);
        }
    }
    impl<'c> Collectible<'c> for Struct {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { name: Identifier(name), kind } = self;
            c.define(name).collect(kind);
        }
    }
    impl<'c> Collectible<'c> for StructKind {
        fn collect(&'c self, c: &mut Collector<'c>) {
            match self {
                StructKind::Empty => {}
                StructKind::Tuple(k) => k.collect(c),
                StructKind::Struct(k) => k.collect(c),
            }
        }
    }
    impl<'c> Collectible<'c> for StructMember {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { vis: _, name: Identifier(name), ty } = self;
            c.define(name).collect(ty);
        }
    }
    impl<'c> Collectible<'c> for Enum {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { name: Identifier(name), kind } = self;
            c.define(name).collect(kind);
        }
    }
    impl<'c> Collectible<'c> for EnumKind {
        fn collect(&'c self, c: &mut Collector<'c>) {
            match self {
                EnumKind::NoVariants => {}
                EnumKind::Variants(v) => v.collect(c),
            }
        }
    }
    impl<'c> Collectible<'c> for Variant {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { name: Identifier(name), kind } = self;
            c.define(name).collect(kind);
        }
    }
    impl<'c> Collectible<'c> for VariantKind {
        fn collect(&'c self, c: &mut Collector<'c>) {
            match self {
                VariantKind::Plain => {}
                VariantKind::CLike(_) => {}
                VariantKind::Tuple(v) => v.collect(c),
                VariantKind::Struct(v) => v.collect(c),
            }
        }
    }
    impl<'c> Collectible<'c> for Impl {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { target, body } = self;
            c.collect(target).collect(body);
        }
    }
    impl<'c> Collectible<'c> for Block {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { stmts } = self;
            for stmt in stmts {
                stmt.collect(c);
            }
        }
    }
    impl<'c> Collectible<'c> for Stmt {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { extents, kind, semi: _ } = self;
            c.location(extents.head).collect(kind);
        }
    }
    impl<'c> Collectible<'c> for StmtKind {
        fn collect(&'c self, c: &mut Collector<'c>) {
            match self {
                StmtKind::Empty => {}
                StmtKind::Local(s) => s.collect(c),
                StmtKind::Item(s) => s.collect(c),
                StmtKind::Expr(s) => s.collect(c),
            }
        }
    }
    impl<'c> Collectible<'c> for Let {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { mutable: _, name: Identifier(name), ty, init } = self;
            c.collect(init).collect(ty).define(name);
        }
    }
    impl<'c> Collectible<'c> for Expr {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { extents, kind } = self;
            c.location(extents.head).collect(kind);
        }
    }
    impl<'c> Collectible<'c> for ExprKind {
        fn collect(&'c self, c: &mut Collector<'c>) {
            match self {
                ExprKind::Assign(k) => k.collect(c),
                ExprKind::Binary(k) => k.collect(c),
                ExprKind::Unary(k) => k.collect(c),
                ExprKind::Member(k) => k.collect(c),
                ExprKind::Call(k) => k.collect(c),
                ExprKind::Index(k) => k.collect(c),
                ExprKind::Path(k) => k.collect(c),
                ExprKind::Literal(k) => k.collect(c),
                ExprKind::Array(k) => k.collect(c),
                ExprKind::ArrayRep(k) => k.collect(c),
                ExprKind::AddrOf(k) => k.collect(c),
                ExprKind::Block(k) => k.collect(c),
                ExprKind::Empty => {}
                ExprKind::Group(k) => k.collect(c),
                ExprKind::Tuple(k) => k.collect(c),
                ExprKind::While(k) => k.collect(c),
                ExprKind::If(k) => k.collect(c),
                ExprKind::For(k) => k.collect(c),
                ExprKind::Break(k) => k.collect(c),
                ExprKind::Return(k) => k.collect(c),
                ExprKind::Continue(k) => k.collect(c),
            }
        }
    }
    impl<'c> Collectible<'c> for Assign {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { head, op: _, tail } = self;
            c.collect(head).collect(tail);
        }
    }
    impl<'c> Collectible<'c> for Binary {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { head, tail } = self;
            c.collect(head);
            for (_, tail) in tail {
                c.collect(tail);
            }
        }
    }
    impl<'c> Collectible<'c> for Unary {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { ops: _, tail } = self;
            c.collect(tail);
        }
    }
    impl<'c> Collectible<'c> for Member {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { head, tail } = self;
            c.collect(head).collect(tail);
        }
    }
    impl<'c> Collectible<'c> for Call {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { callee, args } = self;
            c.collect(callee).collect(args);
        }
    }
    impl<'c> Collectible<'c> for Tuple {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { exprs } = self;
            c.collect(exprs);
        }
    }
    impl<'c> Collectible<'c> for Index {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { head, indices } = self;
            c.collect(head).collect(indices);
        }
    }
    impl<'c> Collectible<'c> for Indices {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { exprs } = self;
            c.collect(exprs);
        }
    }
    impl<'c> Collectible<'c> for Array {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { values } = self;
            c.collect(values);
        }
    }
    impl<'c> Collectible<'c> for ArrayRep {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { value, repeat } = self;
            c.collect(value).collect(repeat);
        }
    }
    impl<'c> Collectible<'c> for AddrOf {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { count: _, mutable: _, expr } = self;
            c.collect(expr);
        }
    }
    impl<'c> Collectible<'c> for Group {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { expr } = self;
            c.collect(expr);
        }
    }
    impl<'c> Collectible<'c> for While {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { cond, pass, fail } = self;
            c.collect(cond).collect(pass).collect(fail);
        }
    }
    impl<'c> Collectible<'c> for Else {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { body } = self;
            c.collect(body);
        }
    }
    impl<'c> Collectible<'c> for If {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { cond, pass, fail } = self;
            c.collect(cond).collect(pass).collect(fail);
        }
    }
    impl<'c> Collectible<'c> for For {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { bind: Identifier(name), cond, pass, fail } = self;
            c.collect(cond).define(name).collect(pass).collect(fail);
        }
    }
    impl<'c> Collectible<'c> for Break {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { body } = self;
            c.collect(body);
        }
    }
    impl<'c> Collectible<'c> for Return {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { body } = self;
            c.collect(body);
        }
    }
    impl<'c> Collectible<'c> for Continue {
        fn collect(&'c self, _c: &mut Collector<'c>) {}
    }
    impl<'c> Collectible<'c> for Literal {
        fn collect(&'c self, _c: &mut Collector<'c>) {}
    }
    impl<'c> Collectible<'c> for Identifier {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self(name) = self;
            c.reference(name);
        }
    }
    impl<'c> Collectible<'c> for Param {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { mutability: _, name, ty } = self;
            c.collect(name).collect(ty);
        }
    }
    impl<'c> Collectible<'c> for Ty {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { extents, kind } = self;
            c.location(extents.head).collect(kind);
        }
    }
    impl<'c> Collectible<'c> for TyKind {
        fn collect(&'c self, c: &mut Collector<'c>) {
            match self {
                TyKind::Never => {}
                TyKind::Empty => {}
                TyKind::SelfTy => {}
                TyKind::Path(t) => t.collect(c),
                TyKind::Tuple(t) => t.collect(c),
                TyKind::Ref(t) => t.collect(c),
                TyKind::Fn(t) => t.collect(c),
            }
        }
    }
    impl<'c> Collectible<'c> for Path {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { absolute: _, parts } = self;
            for part in parts {
                c.collect(part);
            }
        }
    }
    impl<'c> Collectible<'c> for PathPart {
        fn collect(&'c self, c: &mut Collector<'c>) {
            match self {
                PathPart::SuperKw => {}
                PathPart::SelfKw => {}
                PathPart::Ident(i) => i.collect(c),
            }
        }
    }
    impl<'c> Collectible<'c> for TyTuple {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { types } = self;
            for ty in types {
                c.collect(ty);
            }
        }
    }
    impl<'c> Collectible<'c> for TyRef {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { count: _, to } = self;
            c.collect(to);
        }
    }
    impl<'c> Collectible<'c> for TyFn {
        fn collect(&'c self, c: &mut Collector<'c>) {
            let Self { args, rety } = self;
            c.collect(args).collect(rety);
        }
    }

    impl<'c, C: Collectible<'c> + Sized> Collectible<'c> for Vec<C> {
        fn collect(&'c self, c: &mut Collector<'c>) {
            for i in self {
                c.collect(i);
            }
        }
    }
    impl<'c, C: Collectible<'c> + Sized> Collectible<'c> for Option<C> {
        fn collect(&'c self, c: &mut Collector<'c>) {
            if let Some(i) = self {
                c.collect(i);
            }
        }
    }
    impl<'c, C: Collectible<'c>> Collectible<'c> for Box<C> {
        fn collect(&'c self, c: &mut Collector<'c>) {
            c.collect(self.as_ref());
        }
    }
}
