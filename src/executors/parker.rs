use std::{
    cell::Cell,
    marker::PhantomData,
    sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        Arc, Condvar, Mutex,
    },
    task::{Wake, Waker},
};

pub(crate) fn pair() -> (Parker, Unparker) {
    let p = Parker::new();
    let u = p.unparker();
    (p, u)
}

pub(crate) struct Parker {
    unparker: Unparker,
    _marker: PhantomData<Cell<()>>,
}

impl Parker {
    pub(crate) fn new() -> Parker {
        Parker {
            unparker: Unparker {
                inner: Arc::new(Inner {
                    state: AtomicUsize::new(EMPTY),
                    lock: Mutex::new(()),
                    cvar: Condvar::new(),
                }),
            },
            _marker: PhantomData,
        }
    }

    pub(crate) fn park(&self) {
        self.unparker.inner.park();
    }

    pub(crate) fn unparker(&self) -> Unparker {
        self.unparker.clone()
    }
}

pub(crate) struct Unparker {
    inner: Arc<Inner>,
}

impl Unparker {
    pub(crate) fn unpark(&self) {
        self.inner.unpark();
    }
}

impl Clone for Unparker {
    fn clone(&self) -> Unparker {
        Unparker {
            inner: self.inner.clone(),
        }
    }
}

impl From<Unparker> for Waker {
    fn from(up: Unparker) -> Self {
        Waker::from(up.inner)
    }
}

const EMPTY: usize = 0;
const PARKED: usize = 1;
const NOTIFIED: usize = 2;

struct Inner {
    state: AtomicUsize,
    lock: Mutex<()>,
    cvar: Condvar,
}

impl Inner {
    fn park(&self) {
        if self
            .state
            .compare_exchange(NOTIFIED, EMPTY, SeqCst, SeqCst)
            .is_ok()
        {
            return;
        }

        let mut m = self.lock.lock().unwrap();

        match self.state.compare_exchange(EMPTY, PARKED, SeqCst, SeqCst) {
            Ok(_) => {}
            Err(NOTIFIED) => {
                let old = self.state.swap(EMPTY, SeqCst);
                assert_eq!(old, NOTIFIED, "park state changed unexpectedly");
                return;
            }
            Err(n) => panic!("inconsistent park_timeout state: {}", n),
        }

        loop {
            m = self.cvar.wait(m).unwrap();

            if self
                .state
                .compare_exchange(NOTIFIED, EMPTY, SeqCst, SeqCst)
                .is_ok()
            {
                return;
            }
        }
    }

    pub(crate) fn unpark(&self) {
        match self.state.swap(NOTIFIED, SeqCst) {
            EMPTY => return,
            NOTIFIED => return,
            PARKED => {}
            _ => panic!("inconsistent state in unpark"),
        }

        drop(self.lock.lock().unwrap());
        self.cvar.notify_one();
    }
}

impl Wake for Inner {
    fn wake(self: Arc<Self>) {
        self.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.unpark();
    }
}
