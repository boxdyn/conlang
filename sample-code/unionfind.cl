//! Disjoint Set Forest implementation of the Union Find algorithm

enum TreeNode {
    Empty,
    Filled { parent: usize, rank: usize }
}

// TODO: Namespace based on type of first argument

fn makeset(f, x) {
    match (*f)[x] {
        TreeNode::Empty => {(*f)[x] = TreeNode::Filled { parent: x, rank: 0 }};
        _ => {}
    }
}

fn union(f, a, b) {
    let (a, b) = (find(f, a), find(f, b));
    if a == b { return; }
    match ((*f)[a], (*f)[b]) {
        (TreeNode::Filled {parent: _, rank: r_a}, TreeNode::Filled {parent: _, rank: r_b}) => {
            if r_a < r_b {
                union(f, b, a)
            } else {
                (*f)[b].parent = a;
                (*f)[a].rank += 1;
            }
        };
        _ => {}
    }
}

fn find(f, x) {
    match (*f)[x] {
        TreeNode::Filled { parent, rank } => if parent == x {
            x
        } else {
            let parent = find(f, parent);
            (*f)[x].parent = parent;
            parent
        };
        _ => x;
    }
}

fn show(f) {
    for node in 0..(len((*f))) {
        match (*f)[node] {
            TreeNode::Filled { parent, rank } => println(node, ": { parent: ", parent, ", rank: ", rank, " }");
            _ => {}
        }
    }
}

fn test(f) {
    "Union Find on Disjoint Set Forest".println();
    for i in 0..10 { makeset(f, i) }
    for i in 10..20 { makeset(f, i) }
    show(f);
    println();
    
    for i in 1..10 { union(f, i*2-1, i*2) }
    for i in 5..10 { union(f, i*2+1, i*2) }
    show(f)
}

fn main() {
    let f = [TreeNode::Empty;20];
    f.test()
}
