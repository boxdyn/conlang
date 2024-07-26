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

impl fmt::Display for Entry<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(&kind) = self.kind() else {
            return write!(f, "<invalid type: {}>", self.id);
        };

        if let Some(ty) = self.ty() {
            match ty {
                TypeKind::Instance(id) => write!(f, "{}", self.with_id(*id)),
                TypeKind::Intrinsic(kind) => write!(f, "{kind}"),
                TypeKind::Adt(adt) => write_adt(adt, self, f),
                &TypeKind::Ref(id) => {
                    f.write_str("&")?;
                    let h_id = self.with_id(id);
                    write_name_or(h_id, f)
                }
                TypeKind::Slice(id) => {
                    write_name_or(self.with_id(*id), &mut f.delimit_with("[", "]"))
                }
                &TypeKind::Array(t, cnt) => {
                    let mut f = f.delimit_with("[", "]");
                    write_name_or(self.with_id(t), &mut f)?;
                    write!(f, "; {cnt}")
                }
                TypeKind::Tuple(ids) => {
                    let mut f = f.delimit_with("(", ")");
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
                TypeKind::Empty => write!(f, "()"),
                TypeKind::Never => write!(f, "!"),
                TypeKind::Module => write!(f, "module?"),
            }
        } else {
            write!(f, "{kind}")
        }
    }
}

fn write_adt(adt: &Adt, h: &Entry, f: &mut impl Write) -> fmt::Result {
    match adt {
        Adt::Enum(variants) => {
            let mut variants = variants.iter();
            separate(", ", || {
                variants.next().map(|(name, def)| {
                    move |f: &mut Delimit<_>| match def {
                        Some(def) => {
                            write!(f, "{name}: ")?;
                            write_name_or(h.with_id(*def), f)
                        }
                        None => write!(f, "{name}"),
                    }
                })
            })(f.delimit_with("enum {", "}"))
        }
        Adt::Struct(members) => {
            let mut members = members.iter();
            separate(", ", || {
                let (name, vis, id) = members.next()?;
                Some(move |f: &mut Delimit<_>| {
                    write!(f, "{vis}{name}: ")?;
                    write_name_or(h.with_id(*id), f)
                })
            })(f.delimit_with("struct {", "}"))
        }
        Adt::TupleStruct(members) => {
            let mut members = members.iter();
            separate(", ", || {
                let (vis, def) = members.next()?;
                Some(move |f: &mut Delimit<_>| {
                    write!(f, "{vis}")?;
                    write_name_or(h.with_id(*def), f)
                })
            })(f.delimit_with("struct (", ")"))
        }
        Adt::UnitStruct => write!(f, "struct"),
        Adt::Union(_) => todo!("Display union types"),
    }
}
