use cl_structures::index_map::*;

// define the index types
make_index! {
    /// Uniquely represents a [Def][1] in the [Def][1] [Pool]
    ///
    /// [1]: crate::definition::Def
    DefID,
}

impl std::fmt::Display for DefID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
