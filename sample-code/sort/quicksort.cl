

fn recursive_qsort<T>(arr: &[i128],head: usize,tail: usize) if head < tail {
    fn partition(arr: &[i128], head: usize, tail: usize) -> usize {
        fn swap(a, b) arr[a], arr[b] = arr[b], arr[a];

        let pivot = arr[tail];
        let i = head;
        for j in i..tail if arr[j] < pivot {
            swap(i, j);
            i += 1
        }
        swap(i, tail);
        i
    }

    let pi = arr.partition(head, tail);
    recursive_qsort(&arr, head, pi - 1);
    recursive_qsort(&arr, pi + 1, tail);
}


impl Array fn quicksort(self: &Self) {
    fn hoare_partition(arr, head: usize, tail: usize) {
        let pivot = arr[head + tail >> 1];
        let i, j = head - 1, tail + 1;
        loop {
            while (i += 1; arr[i] < pivot);
            while (j -= 1; arr[j] > pivot);
            if i >= j break j;
            arr[i], arr[j] = arr[j], arr[i]
        }
    }

    fn quicksort(arr, head: usize, tail: usize) while head < tail {
        let belly = hoare_partition(&arr, head, tail);

        if belly - head < tail - belly {
            quicksort(&arr, head, belly);
            head = belly + 1
        } else {
            quicksort(&arr, belly + 1, tail);
            tail = belly;
        }
    }

    quicksort(&self, 0, self.len() - 1);
    self
}

impl Array<T> fn quicksort_by(self: &Self, f: (T) -> Cmp) {
    fn hoare_partition(arr, head: usize, tail: usize) {
        let pivot = f(arr[head + tail >> 1]);
        let i, j = head - 1, tail + 1;
        loop {
            while (i += 1; f(arr[i]) < pivot);
            while (j -= 1; f(arr[j]) > pivot);
            if i >= j break j;
            arr[i], arr[j] = arr[j], arr[i]
        }
    }

    fn quicksort(arr, head: usize, tail: usize) while head < tail {
        let belly = hoare_partition(&arr, head, tail);

        if belly - head < tail - belly {
            quicksort(&arr, head, belly);
            head = belly + 1
        } else {
            quicksort(&arr, belly + 1, tail);
            tail = belly;
        }
    }

    quicksort(&self, 0, self.len() - 1);
    self
}
