//! Desugaring passes for Conlang

pub mod path_absoluter;
pub mod squash_groups;
pub mod while_else;

pub use path_absoluter::NormalizePaths;
pub use squash_groups::SquashGroups;
pub use while_else::WhileElseDesugar;
