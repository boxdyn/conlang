use crate::{definition::Def, path::Path, project::Project};
use cl_structures::index_map::*;

// define the index types
make_index! {
    /// Uniquely represents a [Def][1] in the [Def][1] [Pool]
    ///
    /// [1]: crate::definition::Def
    DefID,
}

/// A handle to a certain [Def] within a [Project]
#[derive(Clone, Copy, Debug)]
pub struct Handle<'prj, 'code> {
    id: DefID,
    prj: &'prj Project<'code>,
}

impl DefID {
    /// Constructs a new [Handle] from this DefID and the provided [Project].
    pub fn handle<'p, 'c>(self, prj: &'p Project<'c>) -> Option<Handle<'p, 'c>> {
        Handle::new(self, prj)
    }

    /// Constructs a new [Handle] from this DefID and the provided [Project]
    pub fn handle_unchecked<'p, 'c>(self, prj: &'p Project<'c>) -> Handle<'p, 'c> {
        Handle::new_unchecked(self, prj)
    }
}

impl<'p, 'c> Handle<'p, 'c> {
    /// Constructs a new Handle from the provided [DefID] and [Project].
    /// Returns [Some]\(Handle) if the [DefID] exists in the [Project].
    pub fn new(id: DefID, prj: &'p Project<'c>) -> Option<Self> {
        prj.pool.get(id).is_some().then_some(Self { id, prj })
    }

    /// Constructs a new Handle from the provided [DefID] and [Project] without checking membership.
    /// Using the handle may cause panics or other unwanted (but defined) behavior.
    pub fn new_unchecked(id: DefID, prj: &'p Project<'c>) -> Self {
        Self { id, prj }
    }

    /// Gets the [Def] this handle points to.
    pub fn get(self) -> Option<&'p Def<'c>> {
        self.prj.pool.get(self.id)
    }

    pub fn navigate(self, path: Path) -> (Option<Self>, Option<Self>) {
        match self.prj.get(path, self.id) {
            Some((Some(ty), Some(vl), _)) => (Some(self.with(ty)), Some(self.with(vl))),
            Some((_, Some(vl), _)) => (None, Some(self.with(vl))),
            Some((Some(ty), _, _)) => (Some(self.with(ty)), None),
            _ => (None, None),
        }
    }

    /// Gets the [Project] this handle points to.
    pub fn project(self) -> &'p Project<'c> {
        self.prj
    }

    pub fn id(self) -> DefID {
        self.id
    }

    // TODO: get parent, children, etc.

    /// Gets a handle to the other ID within the same project
    pub fn with(self, id: DefID) -> Self {
        Self { id, ..self }
    }
}

mod emulate_def {
    use super::*;
    use cl_ast::{Sym, Visibility};
    use cl_structures::span::Span;
    use std::collections::HashMap;

    impl<'p, 'c> Handle<'p, 'c> {
        pub fn name(self) -> Option<Sym> {
            self.get()?.name()
        }
        pub fn vis(self) -> Option<Visibility> {
            Some(self.get()?.node.vis)
        }
        pub fn span(self) -> Option<Span> {
            Some(*self.get()?.node.span)
        }
        pub fn parent(self) -> Option<Self> {
            self.get()?.module.parent.map(|id| self.with(id))
        }
        pub fn types(self) -> Option<&'p HashMap<Sym, DefID>> {
            let types = &self.get()?.module.types;
            (!types.is_empty()).then_some(types)
        }
        pub fn values(self) -> Option<&'p HashMap<Sym, DefID>> {
            let values = &self.get()?.module.values;
            (!values.is_empty()).then_some(values)
        }
    }
}

mod display {
    use super::*;
    use crate::{definition::*, format_utils::*};
    use std::fmt::{self, Display, Write};

    impl Display for DefID {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    impl Display for Handle<'_, '_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let Self { id, prj } = *self;
            let Some(def) = prj.pool.get(id) else {
                return write!(f, "<invalid type: {id}>");
            };

            // Match the type
            match &def.kind {
                DefKind::Undecided => write!(f, "<undecided>"),
                DefKind::Impl(id) => write!(f, "impl {}", self.with(*id)),
                DefKind::Use(id) => write!(f, "use inside {}", self.with(*id)),
                DefKind::Type(kind) => match kind {
                    TypeKind::Alias(None) => write!(f, "type"),
                    TypeKind::Alias(Some(id)) => write!(f, "{}", self.with(*id)),
                    TypeKind::Intrinsic(intrinsic) => write!(f, "{intrinsic}"),
                    TypeKind::Adt(adt) => display_adt(self, adt, f),
                    TypeKind::Ref(count, id) => {
                        for _ in 0..*count {
                            f.write_char('&')?;
                        }
                        self.with(*id).fmt(f)
                    }
                    TypeKind::Array(id, size) => {
                        write!(f.delimit_with("[", "]"), "{}; {size}", self.with(*id))
                    }
                    TypeKind::Slice(id) => write!(f.delimit_with("[", "]"), "{}", self.with(*id)),
                    TypeKind::Tuple(ids) => {
                        let mut ids = ids.iter();
                        separate(", ", || {
                            let id = ids.next()?;
                            Some(move |f: &mut Delimit<_>| write!(f, "{}", self.with(*id)))
                        })(f.delimit_with("(", ")"))
                    }
                    TypeKind::FnSig { args, rety } => {
                        write!(f, "fn {} -> {}", self.with(*args), self.with(*rety))
                    }
                    TypeKind::Empty => write!(f, "()"),
                    TypeKind::Never => write!(f, "!"),
                    TypeKind::Module => match def.name() {
                        Some(name) => write!(f, "mod {name}"),
                        None => write!(f, "mod"),
                    },
                },

                DefKind::Value(kind) => match kind {
                    ValueKind::Const(id) => write!(f, "const {}", self.with(*id)),
                    ValueKind::Static(id) => write!(f, "static {}", self.with(*id)),
                    ValueKind::Local(id) => write!(f, "local {}", self.with(*id)),
                    ValueKind::Fn(id) => write!(f, "{}", self.with(*id)),
                },
            }
        }
    }

    /// Formats an ADT: a continuation of [Handle::fmt]
    fn display_adt(handle: &Handle, adt: &Adt, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match adt {
            Adt::Enum(variants) => {
                let mut variants = variants.iter();
                separate(",", || {
                    variants.next().map(|(name, def)| {
                        move |f: &mut Delimit<_>| match def {
                            Some(def) => write!(f, "\n{name}: {}", handle.with(*def)),
                            None => write!(f, "\n{name}"),
                        }
                    })
                })(f.delimit_with("enum {", "\n}"))
            }
            Adt::CLikeEnum(variants) => {
                let mut variants = variants.iter();
                separate(",", || {
                    let (name, descrim) = variants.next()?;
                    Some(move |f: &mut Delimit<_>| write!(f, "\n{name} = {descrim}"))
                })(f.delimit_with("enum {", "\n}"))
            }
            Adt::FieldlessEnum => write!(f, "enum"),
            Adt::Struct(members) => {
                let mut members = members.iter();
                separate(",", || {
                    let (name, vis, id) = members.next()?;
                    Some(move |f: &mut Delimit<_>| write!(f, "\n{vis}{name}: {}", handle.with(*id)))
                })(f.delimit_with("struct {", "\n}"))
            }
            Adt::TupleStruct(members) => {
                let mut members = members.iter();
                separate(", ", || {
                    let (vis, def) = members.next()?;
                    Some(move |f: &mut Delimit<_>| write!(f, "{vis}{}", handle.with(*def)))
                })(f.delimit_with("struct (", ")"))
            }
            Adt::UnitStruct => write!(f, "struct;"),
            Adt::Union(variants) => {
                let mut variants = variants.iter();
                separate(",", || {
                    let (name, def) = variants.next()?;
                    Some(move |f: &mut Delimit<_>| write!(f, "\n{name}: {}", handle.with(*def)))
                })(f.delimit_with("union {", "\n}"))
            }
        }
    }
}
