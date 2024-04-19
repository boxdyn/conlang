//! Contains an [immutable visitor](Visit) and an [owned folder](Fold) trait,
//! with default implementations across the entire AST

pub mod fold;
pub mod visit;

pub use fold::Fold;
pub use visit::Visit;
