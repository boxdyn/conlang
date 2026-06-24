pub use cl_ast::fmt::*;
use std::{fmt, iter};

/// Separates the items yielded by iterating the provided function
pub const fn separate<'f, 's, Item, F, W>(sep: &'s str, t: F) -> impl FnOnce(W) -> fmt::Result + 's
where
    Item: FnMut(&mut W) -> fmt::Result,
    F: FnMut() -> Option<Item> + 's,
    W: fmt::Write,
{
    move |mut f| {
        for (idx, mut disp) in iter::from_fn(t).enumerate() {
            if idx > 0 {
                f.write_str(sep)?;
            }
            disp(&mut f)?;
        }
        Ok(())
    }
}
