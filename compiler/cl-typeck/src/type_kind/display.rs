//! [Display] implementations for [TypeKind], [Adt], and [Intrinsic]

use super::{Adt, Intrinsic, TypeKind};
use crate::format_utils::*;
use cl_ast::format::FmtAdapter;
use std::fmt::{self, Display, Write};

impl Display for TypeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeKind::Uninferred => write!(f, "_"),
            TypeKind::Variable => write!(f, "?"),
            TypeKind::Instance(def) => write!(f, "alias to #{def}"),
            TypeKind::Intrinsic(i) => i.fmt(f),
            TypeKind::Adt(a) => a.fmt(f),
            TypeKind::Ref(def) => write!(f, "&{def}"),
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
            Intrinsic::I128 => f.write_str("i128"),
            Intrinsic::Isize => f.write_str("isize"),
            Intrinsic::U8 => f.write_str("u8"),
            Intrinsic::U16 => f.write_str("u16"),
            Intrinsic::U32 => f.write_str("u32"),
            Intrinsic::U64 => f.write_str("u64"),
            Intrinsic::U128 => f.write_str("u128"),
            Intrinsic::Usize => f.write_str("usize"),
            Intrinsic::F8 => f.write_str("f8"),
            Intrinsic::F16 => f.write_str("f16"),
            Intrinsic::F32 => f.write_str("f32"),
            Intrinsic::F64 => f.write_str("f64"),
            Intrinsic::F128 => f.write_str("f128"),
            Intrinsic::Fsize => f.write_str("fsize"),
            Intrinsic::Bool => f.write_str("bool"),
            Intrinsic::Char => f.write_str("char"),
        }
    }
}
