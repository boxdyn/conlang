//! An insert-only unordered tree, backed by a [Vec]
//!
//! # Examples
//! ```
//! use cl_structures::tree::{Tree, Node};
//! // A tree can be created
//! let mut tree = Tree::new();
//! // Provided with a root node
//! let root = tree.root("This is the root node").unwrap();
//!
//! // Nodes can be accessed by indexing
//! assert_eq!(*tree[root].as_ref(), "This is the root node");
//! // Nodes' data can be accessed directly by calling `get`/`get_mut`
//! assert_eq!(tree.get(root).unwrap(), &"This is the root node")
//! ```

// #![allow(unused)]

pub use self::tree_ref::Ref;
use std::ops::{Index, IndexMut};

pub mod tree_ref;

/// An insert-only unordered tree, backed by a [Vec]
#[derive(Debug)]
pub struct Tree<T> {
    nodes: Vec<Node<T>>,
}

impl<T> Default for Tree<T> {
    fn default() -> Self {
        Self { nodes: Default::default() }
    }
}

/// Getters
impl<T> Tree<T> {
    pub fn get(&self, index: Ref<T>) -> Option<&T> {
        self.get_node(index).map(|node| &node.value)
    }
    pub fn get_mut(&mut self, index: Ref<T>) -> Option<&mut T> {
        self.get_node_mut(index).map(|node| &mut node.value)
    }
    pub fn get_node(&self, index: Ref<T>) -> Option<&Node<T>> {
        self.nodes.get(usize::from(index))
    }
    pub fn get_node_mut(&mut self, index: Ref<T>) -> Option<&mut Node<T>> {
        self.nodes.get_mut(usize::from(index))
    }
}

/// Tree operations
impl<T> Tree<T> {
    pub fn new() -> Self {
        Self { nodes: Default::default() }
    }

    /// Creates a new root for the tree.
    ///
    /// If the tree already has a root, the value will be returned.
    pub fn root(&mut self, value: T) -> Result<Ref<T>, T> {
        if self.is_empty() {
            // Create an index for the new node
            let node = Ref::new_unchecked(self.nodes.len());
            // add child to tree
            self.nodes.push(Node::from(value));
            Ok(node)
        } else {
            Err(value)
        }
    }

    pub fn get_root(&mut self) -> Option<Ref<T>> {
        match self.nodes.is_empty() {
            true => None,
            false => Some(Ref::new_unchecked(0)),
        }
    }

    /// Insert a value into the tree as a child of the parent node
    ///
    /// # Panics
    /// May panic if the node [Ref] is from a different tree
    pub fn insert(&mut self, value: T, parent: Ref<T>) -> Ref<T> {
        let child = Ref::new_unchecked(self.nodes.len());
        // add child to tree before parent
        self.nodes.push(Node::with_parent(value, parent));
        // add child to parent
        self[parent].children.push(child);

        child
    }

    /// Gets the depth of a node
    ///
    /// # Panics
    /// May panic if the node [Ref] is from a different tree
    pub fn depth(&self, node: Ref<T>) -> usize {
        match self[node].parent {
            Some(node) => self.depth(node) + 1,
            None => 0,
        }
    }

    /// Gets the number of branches in the tree
    pub fn branches(&self) -> usize {
        self.nodes.iter().fold(0, |edges, node| edges + node.len())
    }
}

/// Standard data structure functions
impl<T> Tree<T> {
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl<T> Index<Ref<T>> for Tree<T> {
    type Output = Node<T>;
    fn index(&self, index: Ref<T>) -> &Self::Output {
        self.get_node(index).expect("Ref should be inside Tree")
    }
}
impl<T> IndexMut<Ref<T>> for Tree<T> {
    fn index_mut(&mut self, index: Ref<T>) -> &mut Self::Output {
        self.get_node_mut(index).expect("Ref should be inside Tree")
    }
}

/// A node in a [Tree]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Node<T> {
    value: T,
    /// The parent
    parent: Option<Ref<T>>,
    /// The children
    children: Vec<Ref<T>>,
}

impl<T> Node<T> {
    pub const fn new(value: T) -> Self {
        Self { value, parent: None, children: vec![] }
    }
    pub const fn with_parent(value: T, parent: Ref<T>) -> Self {
        Self { value, parent: Some(parent), children: vec![] }
    }

    pub fn get(&self) -> &T {
        self.as_ref()
    }
    pub fn get_mut(&mut self) -> &mut T {
        self.as_mut()
    }
    pub fn swap(&mut self, value: T) -> T {
        std::mem::replace(&mut self.value, value)
    }

    pub fn children(&self) -> &[Ref<T>] {
        &self.children
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}

impl<T> AsRef<T> for Node<T> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}
impl<T> AsMut<T> for Node<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T> From<T> for Node<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod test {
    #[allow(unused)]
    use super::*;

    #[test]
    fn add_children() {
        let mut tree = Tree::new();
        let root = tree.root(0).unwrap();
        let one = tree.insert(1, root);
        let two = tree.insert(2, root);
        assert_eq!([one, two].as_slice(), tree[root].children());
    }
    #[test]
    fn nest_children() {
        let mut tree = Tree::new();
        let root = tree.root(0).unwrap();
        let one = tree.insert(1, root);
        let two = tree.insert(2, one);
        assert_eq!(&[one], tree[root].children());
        assert_eq!(&[two], tree[one].children());
        assert_eq!(tree[two].children(), &[]);
    }

    #[test]
    fn compares_equal() {}
}
