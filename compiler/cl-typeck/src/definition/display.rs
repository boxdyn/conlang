//! [Display] implementations for [TypeKind], [Adt], and [Intrinsic]

use super::{Adt, Def, DefKind, Intrinsic, TypeKind, ValueKind};
use crate::{format_utils::*, node::Node};
use cl_ast::format::FmtAdapter;
use std::fmt::{self, Display, Write};

impl Display for Def<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { module, node: Node { in_path: _, span: _, meta, vis, kind: source }, kind } =
            self;
        if !meta.is_empty() {
            writeln!(f, "#{meta:?}")?;
        }
        if let Some(source) = source {
            if let Some(name) = source.name() {
                writeln!(f, "{vis}{name}:")?;
            }
            writeln!(f.indent(), "source:\n{source}")?;
        } else {
            writeln!(f, "{vis}: ")?;
        }
        writeln!(f, "kind: {kind}")?;
        write!(f, "module: {module}")
    }
}

impl Display for DefKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DefKind::Undecided => write!(f, "undecided"),
            DefKind::Impl(id) => write!(f, "impl {id}"),
            DefKind::Use(id) => write!(f, "use (inside {id})"),
            DefKind::Type(kind) => write!(f, "{kind}"),
            DefKind::Value(kind) => write!(f, "{kind}"),
        }
    }
}

impl std::fmt::Display for ValueKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueKind::Const(id) => write!(f, "const ({id})"),
            ValueKind::Static(id) => write!(f, "static ({id})"),
            ValueKind::Local(id) => write!(f, "let ({id})"),
            ValueKind::Fn(id) => write!(f, "fn def ({id})"),
        }
    }
}

impl Display for TypeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeKind::Alias(def) => match def {
                Some(def) => write!(f, "alias to #{def}"),
                None => f.write_str("type"),
            },
            TypeKind::Intrinsic(i) => i.fmt(f),
            TypeKind::Adt(a) => a.fmt(f),
            TypeKind::Ref(cnt, def) => {
                for _ in 0..*cnt {
                    f.write_str("&")?;
                }
                def.fmt(f)
            }
            TypeKind::Slice(def) => write!(f, "slice [#{def}]"),
            TypeKind::Array(def, size) => write!(f, "array [#{def}; {size}]"),
            TypeKind::Tuple(defs) => {
                let mut defs = defs.iter();
                separate(", ", || {
                    let def = defs.next()?;
                    Some(move |f: &mut Delimit<_>| write!(f, "#{def}"))
                })(f.delimit_with("tuple (", ")"))
            }
            TypeKind::FnSig { args, rety } => write!(f, "fn (#{args}) -> #{rety}"),
            TypeKind::Empty => f.write_str("()"),
            TypeKind::Never => f.write_str("!"),
            TypeKind::Module => f.write_str("mod"),
        }
    }
}

impl Display for Adt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Adt::Enum(variants) => {
                let mut variants = variants.iter();
                separate(", ", || {
                    let (name, def) = variants.next()?;
                    Some(move |f: &mut Delimit<_>| match def {
                        Some(def) => write!(f, "{name}: #{def}"),
                        None => write!(f, "{name}"),
                    })
                })(f.delimit_with("enum {", "}"))
            }
            Adt::CLikeEnum(variants) => {
                let mut variants = variants.iter();
                separate(", ", || {
                    let (name, descrim) = variants.next()?;
                    Some(move |f: &mut Delimit<_>| write!(f, "{name} = {descrim}"))
                })(f.delimit_with("enum {", "}"))
            }
            Adt::FieldlessEnum => write!(f, "enum"),
            Adt::Struct(members) => {
                let mut members = members.iter();
                separate(", ", || {
                    let (name, vis, def) = members.next()?;
                    Some(move |f: &mut Delimit<_>| write!(f, "{vis}{name}: #{def}"))
                })(f.delimit_with("struct {", "}"))
            }
            Adt::TupleStruct(members) => {
                let mut members = members.iter();
                separate(", ", || {
                    let (vis, def) = members.next()?;
                    Some(move |f: &mut Delimit<_>| write!(f, "{vis}#{def}"))
                })(f.delimit_with("struct (", ")"))
            }
            Adt::UnitStruct => write!(f, "struct"),
            Adt::Union(variants) => {
                let mut variants = variants.iter();
                separate(", ", || {
                    let (name, def) = variants.next()?;
                    Some(move |f: &mut Delimit<_>| write!(f, "{name}: #{def}"))
                })(f.delimit_with("union {", "}"))
            }
        }
    }
}

impl Display for Intrinsic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Intrinsic::I8 => f.write_str("i8"),
            Intrinsic::I16 => f.write_str("i16"),
            Intrinsic::I32 => f.write_str("i32"),
            Intrinsic::I64 => f.write_str("i64"),
            Intrinsic::Isize => f.write_str("isize"),
            Intrinsic::U8 => f.write_str("u8"),
            Intrinsic::U16 => f.write_str("u16"),
            Intrinsic::U32 => f.write_str("u32"),
            Intrinsic::U64 => f.write_str("u64"),
            Intrinsic::Usize => f.write_str("usize"),
            Intrinsic::Bool => f.write_str("bool"),
            Intrinsic::Char => f.write_str("char"),
        }
    }
}
