use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[derive(Clone)]
pub(crate) struct Indexer {
    index: Arc<AtomicUsize>,
    last_index: usize,
}

impl Indexer {
    pub(crate) fn new(count: usize) -> Self {
        Indexer {
            index: Arc::new(AtomicUsize::new(0)),
            last_index: count - 1,
        }
    }
}

impl Indexer {
    pub(crate) fn next(&self) -> usize {
        if let Ok(_) =
            self.index
                .compare_exchange(self.last_index, 0, Ordering::SeqCst, Ordering::SeqCst)
        {
            return 0;
        }
        self.index.fetch_add(1, Ordering::SeqCst)
    }
}
