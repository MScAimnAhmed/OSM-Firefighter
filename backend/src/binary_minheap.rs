/// Efficient binary min-heap to be used as Dijkstra PQ on FMI graph data
pub struct BinaryMinHeap {
    heap: Vec<usize>,
    positions: Vec<usize>,
}

/// Get the left child index of `index`
fn get_left(index: usize) -> usize {
    2 * index + 1
}

/// Get the right child index of `index`
fn get_right(index: usize) -> usize {
    2 * index + 2
}

/// Get the parent index of `index`
fn get_parent(index: usize) -> usize {
    if index > 0 {
        (index - (1 - index % 2)) / 2
    } else {
        0
    }
}

impl BinaryMinHeap {
    /// Create a new `BinaryMinHeap` with given capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            heap: Vec::with_capacity(capacity),
            positions: vec![usize::MAX; capacity],
        }
    }

    /// Set key `key` at position `index`
    fn set_key_and_pos(&mut self, key: usize, index: usize) {
        self.heap[index] = key;
        self.positions[key] = index;
    }

    /// Swap positions of the keys at `parent` and `child`
    fn swap(&mut self, parent: usize, child: usize) {
        let p_key = self.heap[parent];
        let ch_key = self.heap[child];

        self.set_key_and_pos(p_key, child);
        self.set_key_and_pos(ch_key, parent);
    }

    /// Fixes the heap structure at `index`
    fn reheap(&mut self, index: usize, priorities: &Vec<usize>) {
        let len = self.heap.len();
        let left = get_left(index);
        let right = get_right(index);

        let mut smallest;
        if left < len && priorities[self.heap[left]] < priorities[self.heap[index]] {
            smallest = left;
        } else {
            smallest = index;
        }
        if right < len && priorities[self.heap[right]] < priorities[self.heap[smallest]] {
            smallest = right;
        }

        if smallest != index {
            self.swap(index, smallest);
            self.reheap(smallest, priorities);
        }
    }

    /// Push a key on the heap
    pub fn push(&mut self, key: usize, priorities: &Vec<usize>) {
        self.heap.push(key);
        let mut index = self.heap.len() - 1;
        self.positions[key] = index;

        let mut parent = get_parent(index);
        while parent != index && priorities[self.heap[index]] < priorities[self.heap[parent]] {
            self.swap(parent, index);
            index = parent;
            parent = get_parent(index);
        }
    }

    /// Pop the minimum key from the heap
    pub fn pop(&mut self, priorities: &Vec<usize>) -> usize {
        let min_key = self.heap[0];
        self.positions[min_key] = usize::MAX;

        let tail_key = self.heap.pop().unwrap();
        if !self.is_empty() {
            self.set_key_and_pos(tail_key, 0);
            self.reheap(0, priorities);
        }

        min_key
    }

    /// Decrease the position of a key.
    /// This method must be called iff the priority of a key
    /// decreases after the heap creation.
    pub fn decrease_key(&mut self, key: usize, priorities: &Vec<usize>) {
        let mut index = self.positions[key];
        let mut parent = get_parent(index);
        while index > 0 && priorities[self.heap[parent]] > priorities[self.heap[index]] {
            self.swap(parent, index);
            index = parent;
            parent = get_parent(index);
        }
    }

    /// Returns `true` if the heap contains `key`
    pub fn contains(&self, key: usize) -> bool {
        self.positions[key] != usize::MAX
    }

    /// Returns `true` if the heap is empty
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }
}