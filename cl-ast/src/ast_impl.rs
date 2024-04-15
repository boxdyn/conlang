//! Implementations of AST nodes and traits
use super::*;

mod display {
    //! Implements [Display] for [AST](super::super) Types
    use super::*;
    pub use delimiters::*;
    use std::{
        borrow::Borrow,
        fmt::{Display, Write},
    };
    mod delimiters {
        #![allow(dead_code)]
        #[derive(Clone, Copy, Debug)]
        pub struct Delimiters<'t> {
            pub open: &'t str,
            pub close: &'t str,
        }
        /// Delimits with braces decorated with spaces  `" {n"`, ..., `"\n}"`
        pub const SPACED_BRACES: Delimiters = Delimiters { open: " {\n", close: "\n}" };
        /// Delimits with braces on separate lines `{\n`, ..., `\n}`
        pub const BRACES: Delimiters = Delimiters { open: "{\n", close: "\n}" };
        /// Delimits with parentheses on separate lines `{\n`, ..., `\n}`
        pub const PARENS: Delimiters = Delimiters { open: "(\n", close: "\n)" };
        /// Delimits with square brackets on separate lines `{\n`, ..., `\n}`
        pub const SQUARE: Delimiters = Delimiters { open: "[\n", close: "\n]" };
        /// Delimits with braces on the same line `{ `, ..., ` }`
        pub const INLINE_BRACES: Delimiters = Delimiters { open: "{ ", close: " }" };
        /// Delimits with parentheses on the same line `( `, ..., ` )`
        pub const INLINE_PARENS: Delimiters = Delimiters { open: "(", close: ")" };
        /// Delimits with square brackets on the same line `[ `, ..., ` ]`
        pub const INLINE_SQUARE: Delimiters = Delimiters { open: "[", close: "]" };
    }
    fn delimit<'a>(
        func: impl Fn(&mut std::fmt::Formatter<'_>) -> std::fmt::Result + 'a,
        delim: Delimiters<'a>,
    ) -> impl Fn(&mut std::fmt::Formatter<'_>) -> std::fmt::Result + 'a {
        move |f| {
            write!(f, "{}", delim.open)?;
            func(f)?;
            write!(f, "{}", delim.close)
        }
    }
    fn separate<'iterable, I>(
        iterable: &'iterable [I],
        sep: impl Display + 'iterable,
    ) -> impl Fn(&mut std::fmt::Formatter<'_>) -> std::fmt::Result + 'iterable
    where
        I: Display,
    {
        move |f| {
            for (idx, item) in iterable.iter().enumerate() {
                if idx > 0 {
                    write!(f, "{sep}")?;
                }
                item.fmt(f)?;
            }
            Ok(())
        }
    }

    impl Display for Mutability {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Mutability::Not => Ok(()),
                Mutability::Mut => "mut ".fmt(f),
            }
        }
    }
    impl Display for Visibility {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Visibility::Private => Ok(()),
                Visibility::Public => "pub ".fmt(f),
            }
        }
    }

    impl Display for File {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            separate(&self.items, "\n")(f)
        }
    }

    impl Display for Attrs {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { meta } = self;
            if meta.is_empty() {
                return Ok(());
            }
            "#".fmt(f)?;
            delimit(separate(meta, ", "), INLINE_SQUARE)(f)?;
            "\n".fmt(f)
        }
    }
    impl Display for Meta {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { name, kind } = self;
            write!(f, "{name}{kind}")
        }
    }
    impl Display for MetaKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                MetaKind::Plain => Ok(()),
                MetaKind::Equals(v) => write!(f, " = {v}"),
                MetaKind::Func(args) => delimit(separate(args, ", "), INLINE_PARENS)(f),
            }
        }
    }

    impl Display for Item {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { extents: _, attrs, vis, kind } = self;
            attrs.fmt(f)?;
            vis.fmt(f)?;
            match kind {
                ItemKind::Alias(v) => v.fmt(f),
                ItemKind::Const(v) => v.fmt(f),
                ItemKind::Static(v) => v.fmt(f),
                ItemKind::Module(v) => v.fmt(f),
                ItemKind::Function(v) => v.fmt(f),
                ItemKind::Struct(v) => v.fmt(f),
                ItemKind::Enum(v) => v.fmt(f),
                ItemKind::Impl(v) => v.fmt(f),
            }
        }
    }
    impl Display for Alias {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { to, from } = self;
            match from {
                Some(from) => write!(f, "type {to} = {from};"),
                None => write!(f, "type {to};"),
            }
        }
    }
    impl Display for Const {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { name, ty, init } = self;
            write!(f, "const {name}: {ty} = {init}")
        }
    }
    impl Display for Static {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { mutable, name, ty, init } = self;
            write!(f, "static {mutable}{name}: {ty} = {init}")
        }
    }
    impl Display for Module {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { name, kind } = self;
            write!(f, "mod {name}{kind}")
        }
    }
    impl Display for ModuleKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ModuleKind::Inline(items) => {
                    ' '.fmt(f)?;
                    delimit(|f| items.fmt(f), BRACES)(f)
                }
                ModuleKind::Outline => ';'.fmt(f),
            }
        }
    }
    impl Display for Function {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { name, args, body, rety } = self;
            write!(f, "fn {name} ")?;
            delimit(separate(args, ", "), INLINE_PARENS)(f)?;
            if let Some(rety) = rety {
                write!(f, " -> {rety}")?;
            }
            match body {
                Some(body) => write!(f, " {body}"),
                None => ';'.fmt(f),
            }
        }
    }
    impl Display for Param {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { mutability, name, ty } = self;
            write!(f, "{mutability}{name}: {ty}")
        }
    }
    impl Display for Struct {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { name, kind } = self;
            write!(f, "struct {name}{kind}")
        }
    }
    impl Display for StructKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                StructKind::Empty => ';'.fmt(f),
                StructKind::Tuple(v) => delimit(separate(v, ", "), INLINE_PARENS)(f),
                StructKind::Struct(v) => delimit(separate(v, ",\n"), SPACED_BRACES)(f),
            }
        }
    }
    impl Display for StructMember {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { vis, name, ty } = self;
            write!(f, "{vis}{name}: {ty}")
        }
    }
    impl Display for Enum {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { name, kind } = self;
            write!(f, "enum {name}{kind}")
        }
    }
    impl Display for EnumKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                EnumKind::NoVariants => ';'.fmt(f),
                EnumKind::Variants(v) => delimit(separate(v, ",\n"), SPACED_BRACES)(f),
            }
        }
    }
    impl Display for Variant {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { name, kind } = self;
            write!(f, "{name}{kind}")
        }
    }
    impl Display for VariantKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                VariantKind::Plain => Ok(()),
                VariantKind::CLike(n) => write!(f, " = {n}"),
                VariantKind::Tuple(v) => delimit(separate(v, ", "), INLINE_PARENS)(f),
                VariantKind::Struct(v) => delimit(separate(v, ", "), INLINE_BRACES)(f),
            }
        }
    }
    impl Display for Impl {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { target, body } = self;
            write!(f, "impl {target} ")?;
            delimit(|f| body.fmt(f), BRACES)(f)
        }
    }
    impl Display for ImplKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ImplKind::Type(t) => t.fmt(f),
                ImplKind::Trait { impl_trait, for_type } => {
                    write!(f, "{impl_trait} for {for_type}")
                }
            }
        }
    }

    impl Display for Ty {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.kind.fmt(f)
        }
    }
    impl Display for TyKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                TyKind::Never => "!".fmt(f),
                TyKind::Empty => "()".fmt(f),
                TyKind::SelfTy => "Self".fmt(f),
                TyKind::Path(v) => v.fmt(f),
                TyKind::Tuple(v) => v.fmt(f),
                TyKind::Ref(v) => v.fmt(f),
                TyKind::Fn(v) => v.fmt(f),
            }
        }
    }
    impl Display for TyTuple {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            delimit(separate(&self.types, ", "), INLINE_PARENS)(f)
        }
    }
    impl Display for TyRef {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { count: _, to } = self;
            for _ in 0..self.count {
                f.write_char('&')?;
            }
            write!(f, "{to}")
        }
    }
    impl Display for TyFn {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { args, rety } = self;
            write!(f, "fn {args}")?;
            match rety {
                Some(v) => write!(f, " -> {v}"),
                None => Ok(()),
            }
        }
    }

    impl Display for Stmt {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Stmt { extents: _, kind, semi } = self;
            match kind {
                StmtKind::Empty => Ok(()),
                StmtKind::Local(v) => v.fmt(f),
                StmtKind::Item(v) => v.fmt(f),
                StmtKind::Expr(v) => v.fmt(f),
            }?;
            match semi {
                Semi::Terminated => ';'.fmt(f),
                Semi::Unterminated => Ok(()),
            }
        }
    }
    impl Display for Let {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { mutable, name, ty, init } = self;
            write!(f, "let {mutable}{name}")?;
            if let Some(value) = ty {
                write!(f, ": {value}")?;
            }
            if let Some(value) = init {
                write!(f, " = {value}")?;
            }
            Ok(())
        }
    }

    impl Display for Expr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.kind.fmt(f)
        }
    }
    impl Display for ExprKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ExprKind::Assign(v) => v.fmt(f),
                ExprKind::Binary(v) => v.fmt(f),
                ExprKind::Unary(v) => v.fmt(f),
                ExprKind::Index(v) => v.fmt(f),
                ExprKind::Path(v) => v.fmt(f),
                ExprKind::Literal(v) => v.fmt(f),
                ExprKind::Array(v) => v.fmt(f),
                ExprKind::ArrayRep(v) => v.fmt(f),
                ExprKind::AddrOf(v) => v.fmt(f),
                ExprKind::Block(v) => v.fmt(f),
                ExprKind::Empty => "()".fmt(f),
                ExprKind::Group(v) => v.fmt(f),
                ExprKind::Tuple(v) => v.fmt(f),
                ExprKind::While(v) => v.fmt(f),
                ExprKind::If(v) => v.fmt(f),
                ExprKind::For(v) => v.fmt(f),
                ExprKind::Break(v) => v.fmt(f),
                ExprKind::Return(v) => v.fmt(f),
                ExprKind::Continue(_) => "continue".fmt(f),
            }
        }
    }
    impl Display for Assign {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { kind, parts } = self;
            write!(f, "{} {kind} {}", parts.0, parts.1)
        }
    }
    impl Display for AssignKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                AssignKind::Plain => "=",
                AssignKind::Mul => "*=",
                AssignKind::Div => "/=",
                AssignKind::Rem => "%=",
                AssignKind::Add => "+=",
                AssignKind::Sub => "-=",
                AssignKind::And => "&=",
                AssignKind::Or => "|=",
                AssignKind::Xor => "^=",
                AssignKind::Shl => "<<=",
                AssignKind::Shr => ">>=",
            }
            .fmt(f)
        }
    }
    impl Display for Binary {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { kind, parts } = self;
            let (head, tail) = parts.borrow();
            match kind {
                BinaryKind::Dot => write!(f, "{head}{kind}{tail}"),
                BinaryKind::Call => write!(f, "{head}{tail}"),
                _ => write!(f, "{head} {kind} {tail}"),
            }
        }
    }
    impl Display for BinaryKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                BinaryKind::Lt => "<",
                BinaryKind::LtEq => "<=",
                BinaryKind::Equal => "==",
                BinaryKind::NotEq => "!=",
                BinaryKind::GtEq => ">=",
                BinaryKind::Gt => ">",
                BinaryKind::RangeExc => "..",
                BinaryKind::RangeInc => "..=",
                BinaryKind::LogAnd => "&&",
                BinaryKind::LogOr => "||",
                BinaryKind::LogXor => "^^",
                BinaryKind::BitAnd => "&",
                BinaryKind::BitOr => "|",
                BinaryKind::BitXor => "^",
                BinaryKind::Shl => "<<",
                BinaryKind::Shr => ">>",
                BinaryKind::Add => "+",
                BinaryKind::Sub => "-",
                BinaryKind::Mul => "*",
                BinaryKind::Div => "/",
                BinaryKind::Rem => "%",
                BinaryKind::Dot => ".",
                BinaryKind::Call => "()",
            }
            .fmt(f)
        }
    }
    impl Display for Unary {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { kind, tail } = self;
            write!(f, "{kind}{tail}")
        }
    }
    impl Display for UnaryKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                UnaryKind::Deref => "*",
                UnaryKind::Neg => "-",
                UnaryKind::Not => "!",
                UnaryKind::At => "@",
                UnaryKind::Tilde => "~",
            }
            .fmt(f)
        }
    }
    impl Display for Tuple {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            delimit(separate(&self.exprs, ", "), INLINE_PARENS)(f)
        }
    }
    impl Display for Index {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { head, indices } = self;
            write!(f, "{head}")?;
            delimit(separate(indices, ", "), INLINE_SQUARE)(f)
        }
    }
    impl Display for Path {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { absolute, parts } = self;
            if *absolute {
                "::".fmt(f)?;
            }
            separate(parts, "::")(f)
        }
    }
    impl Display for PathPart {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                PathPart::SuperKw => "super".fmt(f),
                PathPart::SelfKw => "self".fmt(f),
                PathPart::Ident(id) => id.fmt(f),
            }
        }
    }
    impl Display for Identifier {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }
    impl Display for Literal {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Literal::Bool(v) => v.fmt(f),
                Literal::Char(v) => write!(f, "'{v}'"),
                Literal::Int(v) => v.fmt(f),
                Literal::String(v) => write!(f, "\"{v}\""),
            }
        }
    }
    impl Display for Array {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            delimit(separate(&self.values, ", "), INLINE_SQUARE)(f)
        }
    }
    impl Display for ArrayRep {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { value, repeat } = self;
            write!(f, "[{value}; {repeat}]")
        }
    }
    impl Display for AddrOf {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { count, mutable, expr } = self;
            for _ in 0..*count {
                f.write_char('&')?;
            }
            write!(f, "{mutable}{expr}")
        }
    }
    impl Display for Block {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            delimit(separate(&self.stmts, "\n"), BRACES)(f)
        }
    }
    impl Display for Group {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "({})", self.expr)
        }
    }
    impl Display for While {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { cond, pass, fail } = self;
            write!(f, "while {cond} {pass}{fail}")
        }
    }
    impl Display for If {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { cond, pass, fail } = self;
            write!(f, "if {cond} {pass}{fail}")
        }
    }
    impl Display for For {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { bind, cond, pass, fail } = self;
            write!(f, "for {bind} in {cond} {pass}{fail}")
        }
    }
    impl Display for Else {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match &self.body {
                Some(body) => write!(f, " else {body}"),
                _ => Ok(()),
            }
        }
    }
    impl Display for Break {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "break")?;
            match &self.body {
                Some(body) => write!(f, " {body}"),
                _ => Ok(()),
            }
        }
    }
    impl Display for Return {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "return")?;
            match &self.body {
                Some(body) => write!(f, " {body}"),
                _ => Ok(()),
            }
        }
    }
}

mod convert {
    //! Converts between major enums and enum variants
    use super::*;

    impl<T: AsRef<str>> From<T> for Identifier {
        fn from(value: T) -> Self {
            Identifier(value.as_ref().into())
        }
    }
    impl<T: AsRef<str>> From<T> for PathPart {
        fn from(value: T) -> Self {
            match value.as_ref() {
                "Self" => PathPart::SelfKw,
                "super" => PathPart::SuperKw,
                ident => PathPart::Ident(ident.into()),
            }
        }
    }

    macro impl_from ($(impl From for $T:ty {$($from:ty => $to:expr),*$(,)?})*) {$($(
        impl From<$from> for $T {
            fn from(value: $from) -> Self {
                $to(value.into()) // Uses *tuple constructor*
            }
        }
        impl From<Box<$from>> for $T {
            fn from(value: Box<$from>) -> Self {
                $to((*value).into())
            }
        }
    )*)*}

    impl_from! {
        impl From for ItemKind {
            Alias => ItemKind::Alias,
            Const => ItemKind::Const,
            Static => ItemKind::Static,
            Module => ItemKind::Module,
            Function => ItemKind::Function,
            Struct => ItemKind::Struct,
            Enum => ItemKind::Enum,
            Impl => ItemKind::Impl,
        }
        impl From for StructKind {
            Vec<Ty> => StructKind::Tuple,
            // TODO: Struct members in struct
        }
        impl From for EnumKind {
            Vec<Variant> => EnumKind::Variants,
        }
        impl From for VariantKind {
            u128 => VariantKind::CLike,
            Vec<Ty> => VariantKind::Tuple,
            // TODO: enum struct variants
        }
        impl From for TyKind {
            Path => TyKind::Path,
            TyTuple => TyKind::Tuple,
            TyRef => TyKind::Ref,
            TyFn => TyKind::Fn,
        }
        impl From for StmtKind {
            Let => StmtKind::Local,
            Item => StmtKind::Item,
            Expr => StmtKind::Expr,
        }
        impl From for ExprKind {
            Assign => ExprKind::Assign,
            Binary => ExprKind::Binary,
            Unary => ExprKind::Unary,
            Index => ExprKind::Index,
            Path => ExprKind::Path,
            Literal => ExprKind::Literal,
            Array => ExprKind::Array,
            ArrayRep => ExprKind::ArrayRep,
            AddrOf => ExprKind::AddrOf,
            Block => ExprKind::Block,
            Group => ExprKind::Group,
            Tuple => ExprKind::Tuple,
            While => ExprKind::While,
            If => ExprKind::If,
            For => ExprKind::For,
            Break => ExprKind::Break,
            Return => ExprKind::Return,
            Continue => ExprKind::Continue,
        }
        impl From for Literal {
            bool => Literal::Bool,
            char => Literal::Char,
            u128 => Literal::Int,
            String => Literal::String,
        }
    }

    impl From<Option<Expr>> for Else {
        fn from(value: Option<Expr>) -> Self {
            Self { body: value.map(Into::into) }
        }
    }
    impl From<Expr> for Else {
        fn from(value: Expr) -> Self {
            Self { body: Some(value.into()) }
        }
    }
}
