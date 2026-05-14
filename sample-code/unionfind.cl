//! Disjoint Set Forest implementation of the Union Find algorithm

enum TreeNode {
    Empty,
    Filled { parent: usize, rank: usize }
}

// TODO: Namespace based on type of first argument

fn makeset(f: &[TreeNode], x: usize) {
    match f[x] {
        TreeNode::Empty => f[x] = TreeNode::Filled { parent: x, rank: 0 };
        _ => {}
    }
}

fn union(f: &[TreeNode], a: usize, b: usize) {
    let a, b = f.find(a), f.find(b);
    if a == b return;

    match f[a], f[b] {
        TreeNode::Empty => ();
        TreeNode::Filled { rank: ra }, TreeNode::Filled { rank: rb } if ra < rb => union(f, b, a);
        TreeNode::Filled {..}, TreeNode::Filled {..} => {
            f[b].parent = a;
            f[a].rank += 1;
        }
    }
}

fn find(f: &[TreeNode], x: usize) {
    match f[x] {
        TreeNode::Filled { parent if parent != x, rank } => {
            let parent = find(f, parent);
            f[x].parent = parent;
            parent
        };
        _ => x;
    }
}

fn show(f: &[TreeNode]) {
    for node in 0..f.len() match f[node] {
        TreeNode::Filled { parent, rank } => println(node, ": { parent: ", parent, ", rank: ", rank, " }");
        _ => {}
    }
}

fn test(f: &[TreeNode]) {
    "Union Find on Disjoint Set Forest".println();
    for i in 0..f.len() makeset(f, i);
    f.show();
    println();
    

    // Disjoint fizzbuzz sets
    for i in 1..f.len() match i % 3, i % 5 {
        0, 0 => union(f, 15, i);
        0, _ => union(f, 3, i);
        _, 0 => union(f, 5, i);
        _, _ => union(f, 0, i);
    }

    f.show()
}

fn main() {
    [TreeNode::Empty;40].test()
}
