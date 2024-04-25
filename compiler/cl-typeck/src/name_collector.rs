//! Performs step 1 of type checking: Collecting all the names of things into [Module] units
use crate::{
    definition::{Def, DefKind},
    key::DefID,
    module::Module as Mod,
    project::Project as Prj,
};
use cl_ast::*;

pub trait NameCollectable<'a> {
    /// Collects the identifiers within this node,
    /// returning a new [DefID] if any were allocated,
    /// else returning the parent [DefID]
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str>;

    fn collect_in_root(&'a self, c: &mut Prj<'a>) -> Result<DefID, &'static str> {
        self.collect(c, c.root)
    }
}

impl<'a> NameCollectable<'a> for File {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        for item in &self.items {
            item.collect(c, parent)?;
        }
        Ok(parent)
    }
}
impl<'a> NameCollectable<'a> for Item {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        let Item { attrs: Attrs { meta }, vis, kind, .. } = self;
        let id = match kind {
            ItemKind::Impl(i) => i.collect(c, parent),
            ItemKind::Module(i) => i.collect(c, parent),
            ItemKind::Alias(i) => i.collect(c, parent),
            ItemKind::Enum(i) => i.collect(c, parent),
            ItemKind::Struct(i) => i.collect(c, parent),
            ItemKind::Const(i) => i.collect(c, parent),
            ItemKind::Static(i) => i.collect(c, parent),
            ItemKind::Function(i) => i.collect(c, parent),
            ItemKind::Use(i) => i.collect(c, parent),
        }?;
        c[id].set_meta(meta).set_vis(*vis).set_source(self);

        Ok(id)
    }
}
impl<'a> NameCollectable<'a> for Module {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        let Self { name: Identifier(name), kind } = self;
        let module =
            c.pool
                .insert(Def { name: *name, module: Mod::new(parent), ..Default::default() });
        c[parent].module.types.insert(*name, module);

        match kind {
            ModuleKind::Inline(file) => file.collect(c, module)?,
            ModuleKind::Outline => todo!("Out of line modules: {name}"),
        };
        Ok(module)
    }
}
impl<'a> NameCollectable<'a> for Impl {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        let Self { target: _, body } = self;
        let def = Def { module: Mod::new(parent), ..Default::default() };
        let id = c.pool.insert(def);

        // items will get reparented after name collection, when target is available
        body.collect(c, id)?;

        Ok(id)
    }
}
impl<'a> NameCollectable<'a> for Use {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        let def =
            Def { module: Mod::new(parent), kind: DefKind::Use(parent), ..Default::default() };

        let id = c.pool.insert(def);
        c[parent].module.imports.push(id);

        Ok(id)
    }
}
impl<'a> NameCollectable<'a> for Alias {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        let Alias { to: Identifier(name), .. } = self;

        let def = Def { name: *name, module: Mod::new(parent), ..Default::default() };
        let id = c.pool.insert(def);

        c[parent].module.types.insert(*name, id);
        Ok(id)
    }
}
impl<'a> NameCollectable<'a> for Enum {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        let Enum { name: Identifier(name), .. } = self;

        let def = Def { name: *name, module: Mod::new(parent), ..Default::default() };
        let id = c.pool.insert(def);

        c[parent].module.types.insert(*name, id);
        Ok(id)
    }
}
impl<'a> NameCollectable<'a> for Struct {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        let Struct { name: Identifier(name), .. } = self;

        let def = Def { name: *name, module: Mod::new(parent), ..Default::default() };
        let id = c.pool.insert(def);

        c[parent].module.types.insert(*name, id);
        Ok(id)
    }
}
impl<'a> NameCollectable<'a> for Const {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        let Self { name: Identifier(name), init, .. } = self;

        let kind = DefKind::Undecided;

        let def = Def { name: *name, kind, module: Mod::new(parent), ..Default::default() };
        let id = c.pool.insert(def);

        c[parent].module.values.insert(*name, id);
        init.collect(c, id)?;

        Ok(id)
    }
}
impl<'a> NameCollectable<'a> for Static {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        let Self { name: Identifier(name), init, .. } = self;

        let kind = DefKind::Undecided;

        let def = Def { name: *name, kind, module: Mod::new(parent), ..Default::default() };
        let id = c.pool.insert(def);

        c[parent].module.values.insert(*name, id);
        init.collect(c, id)?;

        Ok(id)
    }
}
impl<'a> NameCollectable<'a> for Function {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        let Self { name: Identifier(name), body, .. } = self;

        let kind = DefKind::Undecided;

        let def = Def { name: *name, kind, module: Mod::new(parent), ..Default::default() };
        let id = c.pool.insert(def);

        c[parent].module.values.insert(*name, id);
        body.collect(c, id)?;

        Ok(id)
    }
}
impl<'a> NameCollectable<'a> for Block {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        self.stmts.as_slice().collect(c, parent)
    }
}
impl<'a> NameCollectable<'a> for Stmt {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        self.kind.collect(c, parent)
    }
}
impl<'a> NameCollectable<'a> for StmtKind {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        match self {
            StmtKind::Empty => Ok(parent),
            StmtKind::Local(Let { init, .. }) => init.collect(c, parent),
            StmtKind::Item(item) => item.collect(c, parent),
            StmtKind::Expr(expr) => expr.collect(c, parent),
        }
    }
}
impl<'a> NameCollectable<'a> for Expr {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        self.kind.collect(c, parent)
    }
}
impl<'a> NameCollectable<'a> for ExprKind {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        match self {
            ExprKind::Assign(Assign { parts, .. }) | ExprKind::Binary(Binary { parts, .. }) => {
                parts.0.collect(c, parent)?;
                parts.1.collect(c, parent)
            }
            ExprKind::Unary(Unary { tail, .. }) => tail.collect(c, parent),
            ExprKind::Index(Index { head, indices }) => {
                indices.collect(c, parent)?;
                head.collect(c, parent)
            }
            ExprKind::Array(Array { values }) => values.collect(c, parent),
            ExprKind::ArrayRep(ArrayRep { value, repeat }) => {
                value.collect(c, parent)?;
                repeat.collect(c, parent)
            }
            ExprKind::AddrOf(AddrOf { expr, .. }) => expr.collect(c, parent),
            ExprKind::Block(block) => block.collect(c, parent),
            ExprKind::Group(Group { expr }) => expr.collect(c, parent),
            ExprKind::Tuple(Tuple { exprs }) => exprs.collect(c, parent),
            ExprKind::While(While { cond, pass, fail })
            | ExprKind::If(If { cond, pass, fail })
            | ExprKind::For(For { cond, pass, fail, .. }) => {
                cond.collect(c, parent)?;
                pass.collect(c, parent)?;
                fail.body.collect(c, parent)
            }
            ExprKind::Break(Break { body }) | ExprKind::Return(Return { body }) => {
                body.collect(c, parent)
            }
            _ => Ok(parent),
        }
    }
}
impl<'a, T: NameCollectable<'a>> NameCollectable<'a> for [T] {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        let mut last = parent;
        for expr in self {
            last = expr.collect(c, parent)?;
        }
        Ok(last)
    }
}
impl<'a, T: NameCollectable<'a>> NameCollectable<'a> for Option<T> {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        match self {
            Some(body) => body.collect(c, parent),
            None => Ok(parent),
        }
    }
}
impl<'a, T: NameCollectable<'a>> NameCollectable<'a> for Box<T> {
    fn collect(&'a self, c: &mut Prj<'a>, parent: DefID) -> Result<DefID, &'static str> {
        (**self).collect(c, parent)
    }
}
