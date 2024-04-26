use cl_structures::deprecated_intern_pool::*;

// define the index types
make_intern_key! {
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
