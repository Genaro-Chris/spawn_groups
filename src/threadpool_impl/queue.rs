use std::collections::VecDeque;

pub(crate) struct PriorityQueue<T> {
    storage: VecDeque<T>,
    compare_fn: fn(&T, &T) -> bool,
}

impl<T> PriorityQueue<T> {
    pub(crate) fn new(compare: fn(&T, &T) -> bool) -> Self {
        Self {
            storage: VecDeque::new(),
            compare_fn: compare,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.storage.clear();
    }

    pub(crate) fn push(&mut self, val: T) {
        self.storage.push_back(val);
        self.up_heap(self.storage.len() - 1);
    }

    pub(crate) fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let result = self.storage.pop_front();
        if !self.is_empty() {
            self.down_heap(0);
        }
        result
    }

    fn up_heap(&mut self, idx: usize) {
        let mut the_idx = idx;
        while the_idx > 0 {
            let parent_idx = the_idx / 2;

            if !(self.compare_fn)(&self.storage[the_idx], &self.storage[parent_idx]) {
                break;
            }

            self.storage.swap(the_idx, parent_idx);
            the_idx = parent_idx
        }
    }

    fn down_heap(&mut self, idx: usize) {
        let mut the_idx = idx;
        loop {
            let left_idx = 2 * the_idx;
            if left_idx >= self.storage.len() {
                break;
            }

            let right_idx = 2 * the_idx + 1;
            let mut largest_idx = the_idx;

            if (self.compare_fn)(&self.storage[left_idx], &self.storage[largest_idx]) {
                largest_idx = left_idx;
            }

            if right_idx < self.storage.len()
                && (self.compare_fn)(&self.storage[right_idx], &self.storage[largest_idx])
            {
                largest_idx = right_idx;
            }

            if largest_idx == the_idx {
                break;
            }

            self.storage.swap(the_idx, largest_idx);
            the_idx = largest_idx
        }
    }
}
