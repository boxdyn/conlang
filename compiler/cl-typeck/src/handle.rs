//! A [Handle] uniquely represents an entry in the [Table](crate::table::Table)

use cl_structures::index_map::*;

// define the index types
make_index! {
    /// Uniquely represents an entry in the [Table](crate::table::Table)
    Handle,
}

impl std::fmt::Display for Handle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
