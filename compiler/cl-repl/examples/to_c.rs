//! Pretty prints a conlang AST in yaml

use cl_ast::{File, Stmt};
use cl_lexer::Lexer;
use cl_parser::Parser;
use repline::{Repline, error::Error as RlError};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    if let Some(path) = std::env::args().nth(1) {
        let f = std::fs::read_to_string(&path).expect("Path must be valid.");
        let mut parser = Parser::new(path, Lexer::new(&f));
        let code: File = match parser.parse() {
            Ok(f) => f,
            Err(e) => {
                eprintln!("{e}");
                return Ok(());
            }
        };
        CLangifier::new().p(&code);
        println!();
        return Ok(());
    }
    let mut rl = Repline::new("\x1b[33m", "cl>", "? >");
    loop {
        let mut line = match rl.read() {
            Err(RlError::CtrlC(_)) => break,
            Err(RlError::CtrlD(line)) => {
                rl.deny();
                line
            }
            Ok(line) => line,
            Err(e) => Err(e)?,
        };
        if !line.ends_with(';') {
            line.push(';');
        }

        let mut parser = Parser::new("stdin", Lexer::new(&line));
        let code = match parser.parse::<Stmt>() {
            Ok(code) => {
                rl.accept();
                code
            }
            Err(e) => {
                print!("\x1b[40G\x1bJ\x1b[91m{e}\x1b[0m");
                continue;
            }
        };
        print!("\x1b[G\x1b[J");
        CLangifier::new().p(&code);
        println!();
    }
    Ok(())
}

pub use clangifier::CLangifier;
pub mod clangifier {
    use crate::clangify::CLangify;
    use std::{
        fmt::Display,
        io::Write,
        ops::{Add, Deref, DerefMut},
    };
    #[derive(Debug, Default)]
    pub struct CLangifier {
        depth: usize,
    }

    impl CLangifier {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn indent(&mut self) -> Section {
            Section::new(self)
        }

        /// Prints a [Yamlify] value
        #[inline]
        pub fn p<T: CLangify + ?Sized>(&mut self, yaml: &T) -> &mut Self {
            yaml.print(self);
            self
        }

        fn increase(&mut self) {
            self.depth += 1;
        }

        fn decrease(&mut self) {
            self.depth -= 1;
        }

        fn print_indentation(&self, writer: &mut impl Write) {
            for _ in 0..self.depth {
                let _ = write!(writer, "    ");
            }
        }

        pub fn endl(&mut self) -> &mut Self {
            self.p("\n")
                .print_indentation(&mut std::io::stdout().lock());
            self
        }

        /// Prints a section header and increases indentation
        pub fn nest(&mut self, name: impl Display) -> Section {
            print!("{name}");
            self.indent()
        }
    }

    impl<C: CLangify + ?Sized> Add<&C> for &mut CLangifier {
        type Output = Self;

        fn add(self, rhs: &C) -> Self::Output {
            self.p(rhs)
        }
    }

    /// Tracks the start and end of an indented block (a "section")
    pub struct Section<'y> {
        yamler: &'y mut CLangifier,
    }

    impl<'y> Section<'y> {
        pub fn new(yamler: &'y mut CLangifier) -> Self {
            yamler.increase();
            Self { yamler }
        }
    }

    impl Deref for Section<'_> {
        type Target = CLangifier;
        fn deref(&self) -> &Self::Target {
            self.yamler
        }
    }
    impl DerefMut for Section<'_> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.yamler
        }
    }

    impl Drop for Section<'_> {
        fn drop(&mut self) {
            let Self { yamler } = self;
            yamler.decrease();
        }
    }
}

pub mod clangify {
    use core::panic;
    use std::iter;

    use super::clangifier::CLangifier;
    use cl_ast::*;

    pub trait CLangify {
        fn print(&self, y: &mut CLangifier);
    }

    impl CLangify for File {
        fn print(&self, mut y: &mut CLangifier) {
            let File { name, items } = self;
            // TODO: turn name into include guard
            y = (y + "// Generated from " + name).endl();
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    y.endl().endl();
                }
                y.p(item);
            }
            y.endl();
        }
    }
    impl CLangify for Visibility {
        fn print(&self, _y: &mut CLangifier) {}
    }
    impl CLangify for Mutability {
        fn print(&self, y: &mut CLangifier) {
            if let Mutability::Not = self {
                y.p("const ");
            }
        }
    }

    impl CLangify for Attrs {
        fn print(&self, y: &mut CLangifier) {
            let Self { meta } = self;
            y.nest("Attrs").p(meta);
            todo!("Attributes");
        }
    }
    impl CLangify for Meta {
        fn print(&self, y: &mut CLangifier) {
            let Self { name, kind } = self;
            y.nest("Meta").p(name).p(kind);
            todo!("Attributes");
        }
    }
    impl CLangify for MetaKind {
        fn print(&self, y: &mut CLangifier) {
            match self {
                MetaKind::Plain => y,
                MetaKind::Equals(value) => y.p(value),
                MetaKind::Func(args) => y.p(args),
            };
            todo!("Attributes");
        }
    }

    impl CLangify for Item {
        fn print(&self, y: &mut CLangifier) {
            let Self { span: _, attrs: _, vis, kind } = self;
            y.p(vis).p(kind);
        }
    }
    impl CLangify for ItemKind {
        fn print(&self, y: &mut CLangifier) {
            match self {
                ItemKind::Alias(f) => y.p(f),
                ItemKind::Const(f) => y.p(f),
                ItemKind::Static(f) => y.p(f),
                ItemKind::Module(f) => y.p(f),
                ItemKind::Function(f) => y.p(f),
                ItemKind::Struct(f) => y.p(f),
                ItemKind::Enum(f) => y.p(f),
                ItemKind::Impl(f) => y.p(f),
                ItemKind::Use(f) => y.p(f),
            };
        }
    }
    impl CLangify for Alias {
        fn print(&self, y: &mut CLangifier) {
            let Self { name, from } = self;
            y.p("typedef ").p(from).p(" ");
            y.p(name).p("; ");
        }
    }
    impl CLangify for Const {
        fn print(&self, y: &mut CLangifier) {
            let Self { name, ty, init } = self;
            y.p("const ").p(ty).p(" ");
            y.p(name).p(" = ").p(init);
        }
    }
    impl CLangify for Static {
        fn print(&self, y: &mut CLangifier) {
            let Self { mutable, name, ty, init } = self;
            y.p(mutable).p(ty).p(" ");
            y.p(name).p(" = ").p(init);
        }
    }
    impl CLangify for Module {
        fn print(&self, y: &mut CLangifier) {
            let Self { name, file } = self;
            y.nest("// mod ").p(name).p(" {").endl();
            y.p(file);
            y.endl().p("// } mod ").p(name);
        }
    }
    impl CLangify for Function {
        fn print(&self, y: &mut CLangifier) {
            let Self { name, sign, bind, body } = self;
            let TyFn { args, rety } = sign;
            let types = match args.as_ref() {
                TyKind::Tuple(TyTuple { types }) => types.as_slice(),
                TyKind::Empty => &[],
                _ => panic!("Unsupported function args: {args}"),
            };
            let bind = match bind {
                Pattern::Tuple(tup) => tup.as_slice(),
                _ => panic!("Unsupported function binders: {args}"),
            };
            match rety {
                Some(ty) => y.p(ty),
                None => y.p("void"),
            }
            .p(" ")
            .p(name)
            .p(" (");
            for (idx, (bind, ty)) in bind.iter().zip(types).enumerate() {
                if idx > 0 {
                    y.p(", ");
                }
                // y.print("/* TODO: desugar pat match args */");
                y.p(ty).p(" ").p(bind);
            }
            y.p(") ").p(body);
        }
    }
    impl CLangify for Struct {
        fn print(&self, y: &mut CLangifier) {
            let Self { name, kind } = self;
            y.p("struct ").p(name).nest(" {").p(kind);
            y.endl().p("}");
        }
    }
    impl CLangify for StructKind {
        fn print(&self, y: &mut CLangifier) {
            match self {
                StructKind::Empty => y.endl().p("char _zero_sized_t;"),
                StructKind::Tuple(k) => {
                    for (idx, ty) in k.iter().enumerate() {
                        y.endl().p(ty).p(" _").p(&idx).p(";");
                    }
                    y
                }
                StructKind::Struct(k) => y.p(k),
            };
        }
    }
    impl CLangify for StructMember {
        fn print(&self, y: &mut CLangifier) {
            let Self { vis, name, ty } = self;
            y.p(vis).p(ty).p(" ").p(name).p(";");
        }
    }
    impl CLangify for Enum {
        fn print(&self, y: &mut CLangifier) {
            let Self { name, variants } = self;
            y.nest("enum ").p(name).p(" {").endl();
            match variants {
                Some(v) => {
                    for (idx, variant) in v.iter().enumerate() {
                        if idx > 0 {
                            y.p(",").endl();
                        }
                        y.p(variant);
                    }
                }
                None => todo!(),
            }
            y.endl().p("\n}");
        }
    }
    impl CLangify for Variant {
        fn print(&self, y: &mut CLangifier) {
            let Self { name, kind } = self;
            y.p(name).p(kind);
        }
    }
    impl CLangify for VariantKind {
        fn print(&self, y: &mut CLangifier) {
            match self {
                VariantKind::Plain => y,
                VariantKind::CLike(v) => y.p(" = ").p(v),
                VariantKind::Tuple(v) => y.p("/* TODO: ").p(v).p(" */"),
                VariantKind::Struct(v) => y.p("/* TODO: ").p(v).p(" */"),
            };
        }
    }
    impl CLangify for Impl {
        fn print(&self, y: &mut CLangifier) {
            let Self { target, body } = self;
            y.nest("/* TODO: impl ").p(target).p(" { */ ");
            y.p(body);
            y.p("/* } // impl ").p(target).p(" */ ");
        }
    }
    impl CLangify for ImplKind {
        fn print(&self, y: &mut CLangifier) {
            match self {
                ImplKind::Type(t) => y.p(t),
                ImplKind::Trait { impl_trait, for_type } => {
                    todo!("impl {impl_trait} for {for_type}")
                }
            };
        }
    }
    impl CLangify for Use {
        fn print(&self, y: &mut CLangifier) {
            let Self { absolute: _, tree } = self;
            y.p(tree);
        }
    }
    impl CLangify for UseTree {
        fn print(&self, y: &mut CLangifier) {
            match self {
                UseTree::Tree(trees) => y.p(trees),
                UseTree::Path(path, tree) => y.p("/* ").p(path).p(" */").p(tree),
                UseTree::Alias(from, to) => y.p("#import <").p(from).p(">.h// ").p(to).p(" "),
                UseTree::Name(name) => y.p("#import <").p(name).p(".h> "),
                UseTree::Glob => y.p("/* TODO: use globbing */"),
            };
        }
    }
    impl CLangify for Block {
        fn print(&self, y: &mut CLangifier) {
            let Self { stmts } = self;
            {
                let mut y = y.nest("{");
                y.endl();
                if let [
                    stmts @ ..,
                    Stmt { span: _, kind: StmtKind::Expr(expr), semi: Semi::Unterminated },
                ] = stmts.as_slice()
                {
                    y.p(stmts).p("return ").p(expr).p(";");
                } else {
                    y.p(stmts);
                }
            }
            y.endl().p("}");
        }
    }
    impl CLangify for Stmt {
        fn print(&self, y: &mut CLangifier) {
            let Self { span: _, kind, semi: _ } = self;
            y.p(kind).p(";").endl();
        }
    }
    impl CLangify for Semi {
        fn print(&self, y: &mut CLangifier) {
            y.p(";");
        }
    }
    impl CLangify for StmtKind {
        fn print(&self, y: &mut CLangifier) {
            match self {
                StmtKind::Empty => y,
                StmtKind::Item(s) => y.p(s),
                StmtKind::Expr(s) => y.p(s),
            };
        }
    }
    impl CLangify for Expr {
        fn print(&self, y: &mut CLangifier) {
            let Self { span: _, kind } = self;
            y.p(kind);
        }
    }
    impl CLangify for ExprKind {
        fn print(&self, y: &mut CLangifier) {
            match self {
                ExprKind::Quote(k) => k.print(y),
                ExprKind::Let(k) => k.print(y),
                ExprKind::Match(k) => k.print(y),
                ExprKind::Assign(k) => k.print(y),
                ExprKind::Modify(k) => k.print(y),
                ExprKind::Binary(k) => k.print(y),
                ExprKind::Unary(k) => k.print(y),
                ExprKind::Cast(k) => k.print(y),
                ExprKind::Member(k) => k.print(y),
                ExprKind::Index(k) => k.print(y),
                ExprKind::Structor(k) => k.print(y),
                ExprKind::Path(k) => k.print(y),
                ExprKind::Literal(k) => k.print(y),
                ExprKind::Array(k) => k.print(y),
                ExprKind::ArrayRep(k) => k.print(y),
                ExprKind::AddrOf(k) => k.print(y),
                ExprKind::Block(k) => k.print(y),
                ExprKind::Empty => {}
                ExprKind::Group(k) => k.print(y),
                ExprKind::Tuple(k) => k.print(y),
                ExprKind::While(k) => k.print(y),
                ExprKind::If(k) => k.print(y),
                ExprKind::For(k) => k.print(y),
                ExprKind::Break(k) => k.print(y),
                ExprKind::Return(k) => k.print(y),
                ExprKind::Continue => {
                    y.nest("continue");
                }
            }
        }
    }
    impl CLangify for Quote {
        fn print(&self, y: &mut CLangifier) {
            y.nest("\"");
            print!("{self}");
            y.p("\"");
        }
    }
    impl CLangify for Let {
        fn print(&self, y: &mut CLangifier) {
            let Self { mutable, name, ty, init } = self;
            let ty = ty.as_deref().map(|ty| &ty.kind).unwrap_or(&TyKind::Infer);
            match ty {
                TyKind::Array(TyArray { ty, count }) => {
                    y.p(ty).p(" ").p(mutable).p(name).p("[").p(count).p("]");
                }
                TyKind::Fn(TyFn { args, rety }) => {
                    y.nest("(").p(rety).p(" *").p(mutable).p(name).p(")(");
                    match args.as_ref() {
                        TyKind::Empty => {}
                        TyKind::Tuple(TyTuple { types }) => {
                            for (idx, ty) in types.iter().enumerate() {
                                if idx > 0 {
                                    y.p(", ");
                                }
                                y.p(ty);
                            }
                        }
                        _ => {
                            y.p(args);
                        }
                    }
                    y.p(")");
                }
                _ => {
                    y.indent().p(ty).p(" ").p(mutable).p(name);
                }
            }
            if let Some(init) = init {
                y.p(" = ").p(init);
            }
        }
    }

    impl CLangify for Pattern {
        fn print(&self, y: &mut CLangifier) {
            // TODO: Pattern match desugaring!!!
            match self {
                Pattern::Name(name) => y.p(name),
                Pattern::Literal(literal) => y.p(literal),
                Pattern::Rest(name) => y.p("..").p(name),
                Pattern::Ref(mutability, pattern) => y.p("&").p(mutability).p(pattern),
                Pattern::RangeExc(head, tail) => y.p("RangeExc").p(head).p(tail),
                Pattern::RangeInc(head, tail) => y.p("RangeExc").p(head).p(tail),
                Pattern::Tuple(patterns) => y.nest("Tuple").p(patterns),
                Pattern::Array(patterns) => y.nest("Array").p(patterns),
                Pattern::Struct(path, items) => {
                    {
                        let mut y = y.nest("Struct");
                        y.p(path);
                        for (name, item) in items {
                            y.p(name).p(item);
                        }
                    }
                    y
                }
                Pattern::TupleStruct(path, items) => {
                    {
                        let mut y = y.nest("TupleStruct");
                        y.p(path).p(items);
                    }
                    y
                }
            };
        }
    }
    impl CLangify for Match {
        fn print(&self, y: &mut CLangifier) {
            let Self { scrutinee, arms } = self;
            y.p("/* match ").p(scrutinee);
            y.nest(" { ").p(arms);
            y.p(" } */");
        }
    }

    impl CLangify for MatchArm {
        fn print(&self, y: &mut CLangifier) {
            let Self(pat, expr) = self;
            y.p(pat).p(" => ").p(expr).p(", ");
        }
    }
    impl CLangify for Assign {
        fn print(&self, y: &mut CLangifier) {
            let Self { parts } = self;
            y.p(&parts.0).p(" = ").p(&parts.1);
        }
    }
    impl CLangify for Modify {
        fn print(&self, y: &mut CLangifier) {
            let Self { kind, parts } = self;
            y.p(&parts.0).p(kind).p(&parts.1);
        }
    }
    impl CLangify for ModifyKind {
        fn print(&self, _y: &mut CLangifier) {
            print!(" {self} ");
        }
    }
    impl CLangify for Binary {
        fn print(&self, y: &mut CLangifier) {
            let Self { kind, parts } = self;
            match kind {
                BinaryKind::Call => y.p(&parts.0).p(&parts.1),
                _ => y.p("(").p(&parts.0).p(kind).p(&parts.1).p(")"),
            };
        }
    }
    impl CLangify for BinaryKind {
        fn print(&self, _y: &mut CLangifier) {
            print!(" {self} ");
        }
    }
    impl CLangify for Unary {
        fn print(&self, y: &mut CLangifier) {
            let Self { kind, tail } = self;
            match kind {
                UnaryKind::Deref => y.p("*").p(tail),
                UnaryKind::Neg => y.p("-").p(tail),
                UnaryKind::Not => y.p("!").p(tail),
                UnaryKind::RangeInc => todo!("Unary RangeInc in C"),
                UnaryKind::RangeExc => todo!("Unary RangeExc in C"),
                UnaryKind::Loop => y.nest("while (1) { ").p(tail).p(" }"),
                UnaryKind::At => todo!(),
                UnaryKind::Tilde => todo!(),
            };
        }
    }
    impl CLangify for Cast {
        fn print(&self, y: &mut CLangifier) {
            let Self { head, ty } = self;
            y.nest("(").p(ty).p(")");
            y.p(head);
        }
    }
    impl CLangify for Member {
        fn print(&self, y: &mut CLangifier) {
            let Self { head, kind } = self;
            match kind {
                MemberKind::Call(name, Tuple { exprs }) => {
                    y.p(name);
                    y.p("(");
                    for (idx, expr) in iter::once(head.as_ref()).chain(exprs).enumerate() {
                        if idx > 0 {
                            y.p(", ");
                        }
                        y.p(expr);
                    }
                    y.p(")")
                }
                MemberKind::Struct(name) => y.p(head).p(".").p(name),
                MemberKind::Tuple(idx) => y.p(head).p("._").p(idx),
            };
        }
    }
    impl CLangify for Tuple {
        fn print(&self, y: &mut CLangifier) {
            let Self { exprs } = self;
            let mut y = y.nest("( ");
            for (idx, expr) in exprs.iter().enumerate() {
                if idx > 0 {
                    y.p(", ");
                }
                y.p(expr);
            }
            y.p(" )");
        }
    }
    impl CLangify for Index {
        fn print(&self, y: &mut CLangifier) {
            let Self { head, indices } = self;
            y.p(head);
            for index in indices {
                y.p("[").p(index).p("]");
            }
        }
    }
    impl CLangify for Structor {
        fn print(&self, y: &mut CLangifier) {
            let Self { to, init } = self;
            y.nest("(").p(to).p(")");
            {
                let mut y = y.nest("{ ");
                for (idx, field) in init.iter().enumerate() {
                    if idx > 0 {
                        y.p(", ");
                    }
                    y.p(field);
                }
                y.p(init);
            }
            y.p("}");
        }
    }
    impl CLangify for Fielder {
        fn print(&self, y: &mut CLangifier) {
            let Self { name, init } = self;
            y.p(".").p(name).p(" = ").p(init);
        }
    }
    impl CLangify for Array {
        fn print(&self, y: &mut CLangifier) {
            let Self { values } = self;
            {
                let mut y = y.nest("{");
                y.endl();
                for (idx, value) in values.iter().enumerate() {
                    if idx > 0 {
                        y.p(", ");
                    }
                    y.p(value);
                }
            }
            y.endl().p("}");
        }
    }
    impl CLangify for ArrayRep {
        fn print(&self, y: &mut CLangifier) {
            let Self { value, repeat } = self;
            {
                let mut y = y.nest("{");
                y.endl();
                for idx in 0..*repeat {
                    if idx > 0 {
                        y.p(", ");
                    }
                    y.p(value);
                }
            }
            y.endl().p("}");
        }
    }
    impl CLangify for AddrOf {
        fn print(&self, y: &mut CLangifier) {
            let Self { mutable: _, expr } = self;
            y.p("&").p(expr);
        }
    }
    impl CLangify for Group {
        fn print(&self, y: &mut CLangifier) {
            let Self { expr } = self;
            y.p("(").p(expr).p(")");
        }
    }
    impl CLangify for While {
        fn print(&self, y: &mut CLangifier) {
            // TODO: to properly propagate intermediate values, a new temp variable needs to be
            // declared on every line lmao. This will require type info.
            let Self { cond, pass, fail } = self;
            let Else { body: fail } = fail;
            y.nest("while(1) {")
                .endl()
                .p("if (")
                .p(cond)
                .p(") ")
                .p(pass);
            {
                let mut y = y.nest(" else {");
                y.endl();
                if let Some(fail) = fail {
                    y.p(fail).p(";").endl();
                }
                y.p("break;");
            }
            y.endl().p("}");
        }
    }
    impl CLangify for Else {
        fn print(&self, y: &mut CLangifier) {
            let Self { body } = self;
            if let Some(body) = body {
                y.p(" else ").p(body);
            }
        }
    }
    impl CLangify for If {
        fn print(&self, y: &mut CLangifier) {
            let Self { cond, pass, fail } = self;
            y.p("if (").p(cond).p(")");
            y.p(pass).p(fail);
        }
    }
    impl CLangify for For {
        #[rustfmt::skip]
        fn print(&self, y: &mut CLangifier) {
            let Self { bind, cond, pass, fail: _ } = self;
            let (mode, (head, tail)) = match &cond.kind {
                ExprKind::Binary(Binary { kind: BinaryKind::RangeExc, parts }) => (false, &**parts),
                ExprKind::Binary(Binary { kind: BinaryKind::RangeInc, parts }) => (true, &**parts),
                _ => todo!("Clangify for loops"),
            };
            // for (int bind = head; bind mode? < : <= tail; bind++);
            y.p("for ( int ").p(bind).p(" = ").p(head).p("; ");
            y.p(bind).p(if mode {"<="} else {"<"}).p(tail).p("; ");
            y.p("++").p(bind).p(" ) ").p(pass);

        }
    }
    impl CLangify for Break {
        fn print(&self, y: &mut CLangifier) {
            let Self { body } = self;
            y.nest("break ").p(body);
        }
    }
    impl CLangify for Return {
        fn print(&self, y: &mut CLangifier) {
            let Self { body } = self;
            y.nest("return ").p(body);
        }
    }
    impl CLangify for Literal {
        fn print(&self, y: &mut CLangifier) {
            match self {
                Literal::Float(l) => y.p(l),
                Literal::Bool(l) => y.p(l),
                Literal::Int(l) => y.p(l),
                Literal::Char(l) => y.p("'").p(l).p("'"),
                Literal::String(l) => y.p(&'"').p(l).p(&'"'),
            };
        }
    }
    impl CLangify for Sym {
        fn print(&self, y: &mut CLangifier) {
            y.p(self.to_ref());
        }
    }
    impl CLangify for Ty {
        fn print(&self, y: &mut CLangifier) {
            let Self { span: _, kind } = self;
            y.p(kind);
        }
    }
    impl CLangify for TyKind {
        fn print(&self, y: &mut CLangifier) {
            match self {
                TyKind::Never => y.p("Never"),
                TyKind::Empty => y.p("Empty"),
                TyKind::Infer => y.p("Any"),
                TyKind::Path(t) => y.p(t),
                TyKind::Tuple(t) => y.p(t),
                TyKind::Ref(t) => y.p(t),
                TyKind::Fn(t) => y.p(t),
                TyKind::Slice(t) => y.p(t),
                TyKind::Array(t) => y.p(t),
            };
        }
    }
    impl CLangify for Path {
        fn print(&self, y: &mut CLangifier) {
            let Self { absolute: _, parts } = self;
            for (idx, part) in parts.iter().enumerate() {
                if idx > 0 {
                    y.p("_");
                }
                y.p(part);
            }
        }
    }
    impl CLangify for PathPart {
        fn print(&self, y: &mut CLangifier) {
            match self {
                PathPart::SuperKw => y.p("super"),
                PathPart::SelfKw => y.p("self"),
                PathPart::SelfTy => y.p("Self"),
                PathPart::Ident(i) => y.p(i),
            };
        }
    }
    impl CLangify for TyArray {
        fn print(&self, y: &mut CLangifier) {
            let Self { ty, count } = self;
            y.p(ty).p("[").p(count).p("]");
        }
    }
    impl CLangify for TySlice {
        fn print(&self, y: &mut CLangifier) {
            let Self { ty } = self;
            y.p(ty).p("* ");
        }
    }
    impl CLangify for TyTuple {
        fn print(&self, y: &mut CLangifier) {
            let Self { types } = self;
            {
                let mut y = y.nest("struct {");
                y.endl();
                for (idx, ty) in types.iter().enumerate() {
                    if idx > 0 {
                        y.p(",").endl();
                    }
                    y.p(ty);
                }
            }
            y.endl().p("}");
        }
    }
    impl CLangify for TyRef {
        fn print(&self, y: &mut CLangifier) {
            let Self { count, mutable, to } = self;
            y.p(mutable).p(to);
            for _ in 0..*count {
                y.p("*");
            }
        }
    }
    impl CLangify for TyFn {
        fn print(&self, y: &mut CLangifier) {
            let Self { args, rety } = self;
            // TODO: function pointer syntax
            y.nest("(").p(rety).p(" *)(");
            match args.as_ref() {
                TyKind::Empty => y,
                TyKind::Tuple(TyTuple { types }) => {
                    for (idx, ty) in types.iter().enumerate() {
                        if idx > 0 {
                            y.p(", ");
                        }
                        y.p(ty);
                    }
                    y
                }
                _ => y.p(args),
            }
            .p(")");
        }
    }

    impl<T: CLangify> CLangify for Option<T> {
        fn print(&self, y: &mut CLangifier) {
            if let Some(v) = self {
                y.p(v);
            }
        }
    }
    impl<T: CLangify> CLangify for Box<T> {
        fn print(&self, y: &mut CLangifier) {
            y.p(&**self);
        }
    }
    impl<T: CLangify> CLangify for Vec<T> {
        fn print(&self, y: &mut CLangifier) {
            for thing in self {
                y.p(thing);
            }
        }
    }
    impl<T: CLangify> CLangify for [T] {
        fn print(&self, y: &mut CLangifier) {
            for thing in self {
                y.p(thing);
            }
        }
    }
    impl CLangify for () {
        fn print(&self, _y: &mut CLangifier) {
            // TODO: C has no language support for zst
        }
    }

    impl<T: CLangify> CLangify for &T {
        fn print(&self, y: &mut CLangifier) {
            (*self).print(y)
        }
    }

    impl CLangify for std::fmt::Arguments<'_> {
        fn print(&self, _y: &mut CLangifier) {
            print!("{self}")
        }
    }
    macro_rules! scalar {
        ($($t:ty),*$(,)?) => {
            $(impl CLangify for $t {
                fn print(&self, _y: &mut CLangifier) {
                    print!("{self}");
                }
            })*
        };
    }

    scalar! {
        bool, char, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, str, &str, String
    }
}
