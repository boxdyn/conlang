use super::*;
use crate::{format_utils::*, type_kind::Adt};
use std::fmt::{self, Write};

/// Printing the name of a named type stops infinite recursion
fn write_name_or(h: Entry, f: &mut impl Write) -> fmt::Result {
    match h.name() {
        Some(name) => write!(f, "{name}"),
        None => write!(f, "{h}"),
    }
}

impl fmt::Display for Entry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(&kind) = self.kind() else {
            return write!(f, "<invalid type: {}>", self.id);
        };

        if let Some(ty) = self.ty() {
            match ty {
                TypeKind::Inferred => write!(f, "<_{}>", self.id),
                TypeKind::Variable => write!(f, "<?{}>", self.id),
                TypeKind::Instance(id) => write!(f, "{}<>", self.with_id(*id)),
                TypeKind::Primitive(kind) => write!(f, "{kind}"),
                TypeKind::Adt(adt) => write_adt(adt, self, f),
                &TypeKind::Ref(id) => {
                    f.write_str("&")?;
                    let h_id = self.with_id(id);
                    write_name_or(h_id, f)
                }
                &TypeKind::Ptr(id) => {
                    f.write_str("*")?;
                    let h_id = self.with_id(id);
                    write_name_or(h_id, f)
                }
                TypeKind::Slice(id) => write_name_or(self.with_id(*id), &mut f.delimit("[", "]")),
                &TypeKind::Array(t, cnt) => {
                    let mut f = f.delimit("[", "]");
                    write_name_or(self.with_id(t), &mut f)?;
                    write!(f, "; {cnt}")
                }
                TypeKind::Tuple(ids) => {
                    let mut f = f.delimit("(", ")");
                    for (index, &id) in ids.iter().enumerate() {
                        if index > 0 {
                            write!(f, ", ")?;
                        }
                        write_name_or(self.with_id(id), &mut f)?;
                    }
                    Ok(())
                }
                TypeKind::FnSig { args, rety } => {
                    write!(f, "fn {} -> ", self.with_id(*args))?;
                    write_name_or(self.with_id(*rety), f)
                }
                TypeKind::Module => write!(f, "module?"),
            }
        } else {
            match kind {
                NodeKind::Type
                | NodeKind::Const
                | NodeKind::Static
                | NodeKind::Temporary
                | NodeKind::Let => write!(f, "{kind} {} (untyped)", self.id),
                _ => write!(f, "{kind}"),
            }
        }
    }
}

fn write_adt(adt: &Adt, h: &Entry, f: &mut impl Write) -> fmt::Result {
    match adt {
        Adt::Enum(variants) => {
            let mut variants = variants.iter();
            separate(", ", || {
                variants.next().map(|(name, def)| {
                    move |f: &mut Delimit<_>| write!(f, "{name}: {}", h.with_id(*def))
                })
            })(f.delimit("enum {", "}"))
        }
        Adt::Struct(members) => {
            let mut members = members.iter();
            separate(", ", || {
                let (name, vis, id) = members.next()?;
                Some(move |f: &mut Delimit<_>| write!(f, "{vis}{name}: {}", h.with_id(*id)))
            })(f.delimit("struct {", "}"))
        }
        Adt::TupleStruct(members) => {
            let mut members = members.iter();
            separate(", ", || {
                let (vis, def) = members.next()?;
                Some(move |f: &mut Delimit<_>| write!(f, "{vis}{}", h.with_id(*def)))
            })(f.delimit("struct (", ")"))
        }
        Adt::UnitStruct => write!(f, "struct"),
        Adt::Union(_) => todo!("Display union types"),
    }
}
