//! # The Conlang Type Checker
//!
//! As a statically typed language, Conlang requires a robust type checker to enforce correctness.
//!
//! This crate is a major work-in-progress.
//!
//! # The [Table](table::Table)™
//! A directed graph of nodes and their dependencies.
//!
//! Contains [item definitions](handle) and [type expression](type_expression) information.
//!
//! *Every* item is itself a module, and can contain arbitrarily nested items
//! as part of the item graph
//!
//! The table, additionally, has some queues for use in external algorithms,
//! detailed in the [stage] module.
//!
//! # Namespaces
//! Each item in the graph is given its own namespace, which is further separated into
//! two distinct parts:
//! - Children of an item are direct descendents (i.e. their `parent` is a handle to the item)
//! - Imports of an item are indirect descendents created by `use` or `impl` directives. They are
//!   shadowed by Children with the same name.
//!
//! # Order of operations:
//! For order-of-operations information, see the [stage] module.
#![warn(clippy::all)]

pub(crate) mod format_utils;

pub mod consteval;

pub mod table;

pub mod handle;

pub mod entry;

pub mod type_kind;

pub mod type_expression;

pub mod stage {
    //! Type collection, evaluation, checking, and inference passes.
    //!
    //! # Order of operations
    //! 1. [mod@populate]: Populate the graph with nodes for every named item.
    //! 2. [mod@categorize]: Categorize the nodes according to textual type information.
    //!    - Creates anonymous types (`fn(T) -> U`, `&T`, `[T]`, etc.) as necessary to fill in the
    //!      type graph
    //!    - Creates a new struct type for every enum struct-variant.
    //! 3. [mod@implement]: Import members of implementation modules into types.
    //! 4. [mod@infer]: Infer the types of the AST using HM type inference

    pub use populate::Populator;
    /// Stage 1: Populate the graph with nodes.
    pub mod populate;

    /// Stage 2: Categorize the nodes according to textual type information.
    pub mod categorize;
    pub use categorize::categorize;

    /// Stage 3: Import members of `impl` blocks into their corresponding types.
    pub mod implement;
    pub use implement::implement;

    // TODO: Make type inference stage 5
    // TODO: Use the type information stored in the [table]
    pub mod infer;
}
