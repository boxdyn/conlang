//! Pretty prints a conlang AST in yaml

use cl_ast::Stmt;
use cl_lexer::Lexer;
use cl_parser::Parser;
use repline::{error::Error as RlError, Repline};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut rl = Repline::new("\x1b[33m", "cl>", "? >");
    loop {
        let line = match rl.read() {
            Err(RlError::CtrlC(_)) => break,
            Err(RlError::CtrlD(line)) => {
                rl.deny();
                line
            }
            Ok(line) => line,
            Err(e) => Err(e)?,
        };

        let mut parser = Parser::new(Lexer::new(&line));
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
        print!("\x1b[G\x1b[J\x1b[A");
        Yamler::new().yaml(&code);
        println!();
    }
    Ok(())
}

pub use yamler::Yamler;
pub mod yamler {
    use crate::yamlify::Yamlify;
    use std::{
        fmt::Display,
        io::Write,
        ops::{Deref, DerefMut},
    };
    #[derive(Debug, Default)]
    pub struct Yamler {
        depth: usize,
    }

    impl Yamler {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn indent(&mut self) -> Section {
            Section::new(self)
        }

        /// Prints a [Yamlify] value
        #[inline]
        pub fn yaml<T: Yamlify>(&mut self, yaml: &T) -> &mut Self {
            yaml.yaml(self);
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
                let _ = write!(writer, "  ");
            }
        }

        /// Prints a section header and increases indentation
        pub fn key(&mut self, name: impl Display) -> Section {
            println!();
            self.print_indentation(&mut std::io::stdout().lock());
            print!("- {name}:");
            self.indent()
        }

        /// Prints a yaml key value pair: `- name: "value"`
        pub fn pair<D: Display, T: Yamlify>(&mut self, name: D, value: T) -> &mut Self {
            self.key(name).yaml(&value);
            self
        }

        /// Prints a yaml scalar value: `"name"``
        pub fn value<D: Display>(&mut self, value: D) -> &mut Self {
            print!(" {value}");
            self
        }

        pub fn list<D: Yamlify>(&mut self, list: &[D]) -> &mut Self {
            for (idx, value) in list.iter().enumerate() {
                self.pair(idx, value);
            }
            self
        }
    }

    /// Tracks the start and end of an indented block (a "section")
    pub struct Section<'y> {
        yamler: &'y mut Yamler,
    }

    impl<'y> Section<'y> {
        pub fn new(yamler: &'y mut Yamler) -> Self {
            yamler.increase();
            Self { yamler }
        }
    }

    impl Deref for Section<'_> {
        type Target = Yamler;
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

pub mod yamlify {
    use super::yamler::Yamler;
    use cl_ast::*;

    pub trait Yamlify {
        fn yaml(&self, y: &mut Yamler);
    }

    impl Yamlify for File {
        fn yaml(&self, y: &mut Yamler) {
            let File { items } = self;
            y.key("File").yaml(items);
        }
    }
    impl Yamlify for Visibility {
        fn yaml(&self, y: &mut Yamler) {
            if let Visibility::Public = self {
                y.pair("vis", "pub");
            }
        }
    }
    impl Yamlify for Mutability {
        fn yaml(&self, y: &mut Yamler) {
            if let Mutability::Mut = self {
                y.pair("mut", true);
            }
        }
    }

    impl Yamlify for Attrs {
        fn yaml(&self, y: &mut Yamler) {
            let Self { meta } = self;
            y.key("Attrs").yaml(meta);
        }
    }
    impl Yamlify for Meta {
        fn yaml(&self, y: &mut Yamler) {
            let Self { name, kind } = self;
            y.key("Meta").pair("name", name).yaml(kind);
        }
    }
    impl Yamlify for MetaKind {
        fn yaml(&self, y: &mut Yamler) {
            match self {
                MetaKind::Plain => y,
                MetaKind::Equals(value) => y.pair("equals", value),
                MetaKind::Func(args) => y.pair("args", args),
            };
        }
    }

    impl Yamlify for Item {
        fn yaml(&self, y: &mut Yamler) {
            let Self { extents: _, attrs, vis, kind } = self;
            y.key("Item").yaml(attrs).yaml(vis).yaml(kind);
        }
    }
    impl Yamlify for ItemKind {
        fn yaml(&self, y: &mut Yamler) {
            match self {
                ItemKind::Alias(f) => y.yaml(f),
                ItemKind::Const(f) => y.yaml(f),
                ItemKind::Static(f) => y.yaml(f),
                ItemKind::Module(f) => y.yaml(f),
                ItemKind::Function(f) => y.yaml(f),
                ItemKind::Struct(f) => y.yaml(f),
                ItemKind::Enum(f) => y.yaml(f),
                ItemKind::Impl(f) => y.yaml(f),
                ItemKind::Use(f) => y.yaml(f),
            };
        }
    }
    impl Yamlify for Alias {
        fn yaml(&self, y: &mut Yamler) {
            let Self { to, from } = self;
            y.key("Alias").pair("to", to).pair("from", from);
        }
    }
    impl Yamlify for Const {
        fn yaml(&self, y: &mut Yamler) {
            let Self { name, ty, init } = self;
            y.key("Const")
                .pair("name", name)
                .pair("ty", ty)
                .pair("init", init);
        }
    }
    impl Yamlify for Static {
        fn yaml(&self, y: &mut Yamler) {
            let Self { mutable, name, ty, init } = self;
            y.key(name).yaml(mutable).pair("ty", ty).pair("init", init);
        }
    }
    impl Yamlify for Module {
        fn yaml(&self, y: &mut Yamler) {
            let Self { name, kind } = self;
            y.key("Module").pair("name", name).yaml(kind);
        }
    }
    impl Yamlify for ModuleKind {
        fn yaml(&self, y: &mut Yamler) {
            match self {
                ModuleKind::Inline(f) => y.yaml(f),
                ModuleKind::Outline => y,
            };
        }
    }
    impl Yamlify for Function {
        fn yaml(&self, y: &mut Yamler) {
            let Self { name, sign, bind, body } = self;
            y.key("Function")
                .pair("name", name)
                .pair("sign", sign)
                .pair("bind", bind)
                .pair("body", body);
        }
    }
    impl Yamlify for Struct {
        fn yaml(&self, y: &mut Yamler) {
            let Self { name, kind } = self;
            y.key("Struct").pair("name", name).yaml(kind);
        }
    }
    impl Yamlify for StructKind {
        fn yaml(&self, y: &mut Yamler) {
            match self {
                StructKind::Empty => y,
                StructKind::Tuple(k) => y.yaml(k),
                StructKind::Struct(k) => y.yaml(k),
            };
        }
    }
    impl Yamlify for StructMember {
        fn yaml(&self, y: &mut Yamler) {
            let Self { vis, name, ty } = self;
            y.key("StructMember").yaml(vis).pair("name", name).yaml(ty);
        }
    }
    impl Yamlify for Enum {
        fn yaml(&self, y: &mut Yamler) {
            let Self { name, kind } = self;
            y.key("Enum").pair("name", name).yaml(kind);
        }
    }
    impl Yamlify for EnumKind {
        fn yaml(&self, y: &mut Yamler) {
            match self {
                EnumKind::NoVariants => y,
                EnumKind::Variants(v) => y.yaml(v),
            };
        }
    }
    impl Yamlify for Variant {
        fn yaml(&self, y: &mut Yamler) {
            let Self { name, kind } = self;
            y.key("Variant").pair("name", name).yaml(kind);
        }
    }
    impl Yamlify for VariantKind {
        fn yaml(&self, y: &mut Yamler) {
            match self {
                VariantKind::Plain => y,
                VariantKind::CLike(v) => y.yaml(v),
                VariantKind::Tuple(v) => y.yaml(v),
                VariantKind::Struct(v) => y.yaml(v),
            };
        }
    }
    impl Yamlify for Impl {
        fn yaml(&self, y: &mut Yamler) {
            let Self { target, body } = self;
            y.key("Impl").pair("target", target).pair("body", body);
        }
    }
    impl Yamlify for ImplKind {
        fn yaml(&self, y: &mut Yamler) {
            match self {
                ImplKind::Type(t) => y.value(t),
                ImplKind::Trait { impl_trait, for_type } => {
                    y.pair("trait", impl_trait).pair("for_type", for_type)
                }
            };
        }
    }
    impl Yamlify for Use {
        fn yaml(&self, y: &mut Yamler) {
            let Self { absolute, tree } = self;
            y.key("Use").pair("absolute", absolute).yaml(tree);
        }
    }
    impl Yamlify for UseTree {
        fn yaml(&self, y: &mut Yamler) {
            match self {
                UseTree::Tree(trees) => y.pair("trees", trees),
                UseTree::Path(path, tree) => y.pair("path", path).pair("tree", tree),
                UseTree::Alias(from, to) => y.pair("from", from).pair("to", to),
                UseTree::Name(name) => y.pair("name", name),
                UseTree::Glob => y.value("Glob"),
            };
        }
    }
    impl Yamlify for Block {
        fn yaml(&self, y: &mut Yamler) {
            let Self { stmts } = self;
            y.key("Block").yaml(stmts);
        }
    }
    impl Yamlify for Stmt {
        fn yaml(&self, y: &mut Yamler) {
            let Self { extents: _, kind, semi } = self;
            y.key("Stmt").yaml(kind).yaml(semi);
        }
    }
    impl Yamlify for Semi {
        fn yaml(&self, y: &mut Yamler) {
            if let Semi::Terminated = self {
                y.pair("terminated", true);
            }
        }
    }
    impl Yamlify for StmtKind {
        fn yaml(&self, y: &mut Yamler) {
            match self {
                StmtKind::Empty => y,
                StmtKind::Item(s) => y.yaml(s),
                StmtKind::Expr(s) => y.yaml(s),
            };
        }
    }
    impl Yamlify for Expr {
        fn yaml(&self, y: &mut Yamler) {
            let Self { extents: _, kind } = self;
            y.yaml(kind);
        }
    }
    impl Yamlify for ExprKind {
        fn yaml(&self, y: &mut Yamler) {
            match self {
                ExprKind::Quote(k) => k.yaml(y),
                ExprKind::Let(k) => k.yaml(y),
                ExprKind::Assign(k) => k.yaml(y),
                ExprKind::Modify(k) => k.yaml(y),
                ExprKind::Binary(k) => k.yaml(y),
                ExprKind::Unary(k) => k.yaml(y),
                ExprKind::Cast(k) => k.yaml(y),
                ExprKind::Member(k) => k.yaml(y),
                ExprKind::Index(k) => k.yaml(y),
                ExprKind::Structor(k) => k.yaml(y),
                ExprKind::Path(k) => k.yaml(y),
                ExprKind::Literal(k) => k.yaml(y),
                ExprKind::Array(k) => k.yaml(y),
                ExprKind::ArrayRep(k) => k.yaml(y),
                ExprKind::AddrOf(k) => k.yaml(y),
                ExprKind::Block(k) => k.yaml(y),
                ExprKind::Empty => {}
                ExprKind::Group(k) => k.yaml(y),
                ExprKind::Tuple(k) => k.yaml(y),
                ExprKind::While(k) => k.yaml(y),
                ExprKind::If(k) => k.yaml(y),
                ExprKind::For(k) => k.yaml(y),
                ExprKind::Break(k) => k.yaml(y),
                ExprKind::Return(k) => k.yaml(y),
                ExprKind::Continue => {
                    y.key("Continue");
                }
            }
        }
    }
    impl Yamlify for Quote {
        fn yaml(&self, y: &mut Yamler) {
            y.key("Quote").value(self);
        }
    }
    impl Yamlify for Let {
        fn yaml(&self, y: &mut Yamler) {
            let Self { mutable, name, ty, init } = self;
            y.key("Let")
                .pair("name", name)
                .yaml(mutable)
                .pair("ty", ty)
                .pair("init", init);
        }
    }

    impl Yamlify for Pattern {
        fn yaml(&self, y: &mut Yamler) {
            match self {
                Pattern::Path(path) => y.value(path),
                Pattern::Literal(literal) => y.value(literal),
                Pattern::Ref(mutability, pattern) => {
                    y.pair("mutability", mutability).pair("subpattern", pattern)
                }
                Pattern::Tuple(patterns) => y.key("Tuple").yaml(patterns),
                Pattern::Array(patterns) => y.key("Array").yaml(patterns),
                Pattern::Struct(path, items) => {
                    {
                        let mut y = y.key("Struct");
                        y.pair("name", path);
                        for (name, item) in items {
                            y.pair(name, item);
                        }
                    }
                    y
                }
            };
        }
    }
    impl Yamlify for Match {
        fn yaml(&self, y: &mut Yamler) {
            let Self { scrutinee, arms } = self;
            y.key("Match")
                .pair("scrutinee", scrutinee)
                .pair("arms", arms);
        }
    }

    impl Yamlify for MatchArm {
        fn yaml(&self, y: &mut Yamler) {
            let Self(pat, expr) = self;
            y.pair("pat", pat).pair("expr", expr);
        }
    }
    impl Yamlify for Assign {
        fn yaml(&self, y: &mut Yamler) {
            let Self { parts } = self;
            y.key("Assign")
                .pair("head", &parts.0)
                .pair("tail", &parts.1);
        }
    }
    impl Yamlify for Modify {
        fn yaml(&self, y: &mut Yamler) {
            let Self { kind, parts } = self;
            y.key("Modify")
                .pair("kind", kind)
                .pair("head", &parts.0)
                .pair("tail", &parts.1);
        }
    }
    impl Yamlify for ModifyKind {
        fn yaml(&self, y: &mut Yamler) {
            y.value(self);
        }
    }
    impl Yamlify for Binary {
        fn yaml(&self, y: &mut Yamler) {
            let Self { kind, parts } = self;
            y.key("Binary")
                .pair("kind", kind)
                .pair("head", &parts.0)
                .pair("tail", &parts.1);
        }
    }
    impl Yamlify for BinaryKind {
        fn yaml(&self, y: &mut Yamler) {
            y.value(self);
        }
    }
    impl Yamlify for Unary {
        fn yaml(&self, y: &mut Yamler) {
            let Self { kind, tail } = self;
            y.key("Unary").pair("kind", kind).pair("tail", tail);
        }
    }
    impl Yamlify for UnaryKind {
        fn yaml(&self, y: &mut Yamler) {
            y.value(self);
        }
    }
    impl Yamlify for Cast {
        fn yaml(&self, y: &mut Yamler) {
            let Self { head, ty } = self;
            y.key("Cast").pair("head", head).pair("ty", ty);
        }
    }
    impl Yamlify for Member {
        fn yaml(&self, y: &mut Yamler) {
            let Self { head, kind } = self;
            y.key("Member").pair("head", head).pair("kind", kind);
        }
    }
    impl Yamlify for MemberKind {
        fn yaml(&self, y: &mut Yamler) {
            match self {
                MemberKind::Call(id, args) => y.pair("id", id).pair("args", args),
                MemberKind::Struct(id) => y.pair("id", id),
                MemberKind::Tuple(id) => y.pair("id", id),
            };
        }
    }
    impl Yamlify for Tuple {
        fn yaml(&self, y: &mut Yamler) {
            let Self { exprs } = self;
            y.key("Tuple").list(exprs);
        }
    }
    impl Yamlify for Index {
        fn yaml(&self, y: &mut Yamler) {
            let Self { head, indices } = self;
            y.key("Index").pair("head", head).list(indices);
        }
    }
    impl Yamlify for Structor {
        fn yaml(&self, y: &mut Yamler) {
            let Self { to, init } = self;
            y.key("Structor").pair("to", to).list(init);
        }
    }
    impl Yamlify for Fielder {
        fn yaml(&self, y: &mut Yamler) {
            let Self { name, init } = self;
            y.key("Fielder").pair("name", name).pair("init", init);
        }
    }
    impl Yamlify for Array {
        fn yaml(&self, y: &mut Yamler) {
            let Self { values } = self;
            y.key("Array").list(values);
        }
    }
    impl Yamlify for ArrayRep {
        fn yaml(&self, y: &mut Yamler) {
            let Self { value, repeat } = self;
            y.key("ArrayRep")
                .pair("value", value)
                .pair("repeat", repeat);
        }
    }
    impl Yamlify for AddrOf {
        fn yaml(&self, y: &mut Yamler) {
            let Self { mutable, expr } = self;
            y.key("AddrOf").yaml(mutable).pair("expr", expr);
        }
    }
    impl Yamlify for Group {
        fn yaml(&self, y: &mut Yamler) {
            let Self { expr } = self;
            y.key("Group").yaml(expr);
        }
    }
    impl Yamlify for While {
        fn yaml(&self, y: &mut Yamler) {
            let Self { cond, pass, fail } = self;
            y.key("While")
                .pair("cond", cond)
                .pair("pass", pass)
                .yaml(fail);
        }
    }
    impl Yamlify for Else {
        fn yaml(&self, y: &mut Yamler) {
            let Self { body } = self;
            y.key("Else").yaml(body);
        }
    }
    impl Yamlify for If {
        fn yaml(&self, y: &mut Yamler) {
            let Self { cond, pass, fail } = self;
            y.key("If").pair("cond", cond).pair("pass", pass).yaml(fail);
        }
    }
    impl Yamlify for For {
        fn yaml(&self, y: &mut Yamler) {
            let Self { bind, cond, pass, fail } = self;
            y.key("For")
                .pair("bind", bind)
                .pair("cond", cond)
                .pair("pass", pass)
                .yaml(fail);
        }
    }
    impl Yamlify for Break {
        fn yaml(&self, y: &mut Yamler) {
            let Self { body } = self;
            y.key("Break").yaml(body);
        }
    }
    impl Yamlify for Return {
        fn yaml(&self, y: &mut Yamler) {
            let Self { body } = self;
            y.key("Return").yaml(body);
        }
    }
    impl Yamlify for Literal {
        fn yaml(&self, y: &mut Yamler) {
            y.value(format_args!("\"{self}\""));
        }
    }
    impl Yamlify for Sym {
        fn yaml(&self, y: &mut Yamler) {
            y.value(self);
        }
    }
    impl Yamlify for Param {
        fn yaml(&self, y: &mut Yamler) {
            let Self { mutability, name } = self;
            y.key("Param").yaml(mutability).pair("name", name);
        }
    }
    impl Yamlify for Ty {
        fn yaml(&self, y: &mut Yamler) {
            let Self { extents: _, kind } = self;
            y.key("Ty").yaml(kind);
        }
    }
    impl Yamlify for TyKind {
        fn yaml(&self, y: &mut Yamler) {
            match self {
                TyKind::Never => y.value("Never"),
                TyKind::Empty => y.value("Empty"),
                TyKind::Path(t) => y.yaml(t),
                TyKind::Tuple(t) => y.yaml(t),
                TyKind::Ref(t) => y.yaml(t),
                TyKind::Fn(t) => y.yaml(t),
                TyKind::Slice(_) => todo!(),
                TyKind::Array(_) => todo!(),
            };
        }
    }
    impl Yamlify for Path {
        fn yaml(&self, y: &mut Yamler) {
            let Self { absolute, parts } = self;
            let mut y = y.key("Path");
            if *absolute {
                y.pair("absolute", absolute);
            }
            for part in parts {
                y.pair("part", part);
            }
        }
    }
    impl Yamlify for PathPart {
        fn yaml(&self, y: &mut Yamler) {
            match self {
                PathPart::SuperKw => y.value("super"),
                PathPart::SelfKw => y.value("self"),
                PathPart::SelfTy => y.value("Self"),
                PathPart::Ident(i) => y.yaml(i),
            };
        }
    }
    impl Yamlify for TyArray {
        fn yaml(&self, y: &mut Yamler) {
            let Self { ty, count } = self;
            y.key("TyArray").pair("ty", ty).pair("count", count);
        }
    }
    impl Yamlify for TySlice {
        fn yaml(&self, y: &mut Yamler) {
            let Self { ty } = self;
            y.key("TyArray").pair("ty", ty);
        }
    }
    impl Yamlify for TyTuple {
        fn yaml(&self, y: &mut Yamler) {
            let Self { types } = self;
            let mut y = y.key("TyTuple");
            for ty in types {
                y.yaml(ty);
            }
        }
    }
    impl Yamlify for TyRef {
        fn yaml(&self, y: &mut Yamler) {
            let Self { count, mutable, to } = self;
            y.key("TyRef")
                .pair("count", count)
                .yaml(mutable)
                .pair("to", to);
        }
    }
    impl Yamlify for TyFn {
        fn yaml(&self, y: &mut Yamler) {
            let Self { args, rety } = self;
            y.key("TyFn").pair("args", args).pair("rety", rety);
        }
    }

    impl<T: Yamlify> Yamlify for Option<T> {
        fn yaml(&self, y: &mut Yamler) {
            if let Some(v) = self {
                y.yaml(v);
            } else {
                y.value("");
            }
        }
    }
    impl<T: Yamlify> Yamlify for Box<T> {
        fn yaml(&self, y: &mut Yamler) {
            y.yaml(&**self);
        }
    }
    impl<T: Yamlify> Yamlify for Vec<T> {
        fn yaml(&self, y: &mut Yamler) {
            for thing in self {
                y.yaml(thing);
            }
        }
    }
    impl Yamlify for () {
        fn yaml(&self, _y: &mut Yamler) {}
    }

    impl<T: Yamlify> Yamlify for &T {
        fn yaml(&self, y: &mut Yamler) {
            (*self).yaml(y)
        }
    }

    macro_rules! scalar {
        ($($t:ty),*$(,)?) => {
            $(impl Yamlify for $t {
                fn yaml(&self, y: &mut Yamler) {
                    y.value(self);
                }
            })*
        };
    }

    scalar! {
        bool, char, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, &str, String
    }
}
