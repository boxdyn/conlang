
fn merge(src: &[T], dst: &[T], head: usize, belly: usize, tail: usize) {
    let i, j = head, belly;
    for k in head..tail {
        if i < belly && (j >= tail || src[i] <= src[j]) {
            dst[k] = src[i];
            i += 1;
        } else {
            dst[k] = src[j];
            j += 1;
        }
    }
}

fn recursive_mergesort(self: [T]) -> [T] {
    fn split_merge(a: [T], b: [T], head: usize, tail: usize) {
        if tail - head <= 1 return;
        let belly = head + tail >> 1;
        split_merge(b, a, head, belly);
        split_merge(b, a, belly, tail);
        merge(b, a, head, belly, tail);
    }

    let other = [_; self.len()];
    for idx in 0..self.len() other[idx] = self[idx];
    split_merge(&self, &other, 0, self.len());
    self
}

fn iterative_mergesort(self: [T]) -> [T] {
    let other = [(); self.len()];
    let a, b, n = &self, &other, self.len();

    let width = 1;
    while width < n {
        let head = 0;
        while head < n {
            let belly, tail = head + width, head + width * 2;
            merge(a, b, head, min(belly, n), min(tail, n));
            head = tail;
        }
        a, b = b, a;
        width *= 2;
    }
    // if odd number of iterations, the answer is in b; copy it into a
    if width.ilog(2) % 2 for i in 0..n b[i] = a[i];
    self
}
