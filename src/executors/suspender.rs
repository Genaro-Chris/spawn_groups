use std::{
    sync::{Arc, Condvar, Mutex},
    task::Waker,
};

use super::waker::waker_helper;

pub(crate) fn pair() -> (Suspender, Waker) {
    let suspender = Suspender::new();
    let resumer = suspender.resumer();
    (suspender, waker_helper(move || resumer.resume()))
}

pub(crate) struct Suspender {
    resumer: Resumer,
}

impl Suspender {
    pub(crate) fn new() -> Suspender {
        Suspender {
            resumer: Resumer {
                inner: Arc::new(Inner {
                    lock: Mutex::new(State::Empty),
                    cvar: Condvar::new(),
                }),
            },
        }
    }

    pub(crate) fn suspend(&self) {
        self.resumer.inner.suspend();
    }

    pub(crate) fn resumer(&self) -> Resumer {
        Resumer {
            inner: self.resumer.inner.clone(),
        }
    }
}

pub(crate) struct Resumer {
    inner: Arc<Inner>,
}

impl Resumer {
    pub(crate) fn resume(&self) {
        self.inner.resume();
    }
}

#[derive(PartialEq)]
enum State {
    Empty,
    Notified,
    Suspended,
}

struct Inner {
    lock: Mutex<State>,
    cvar: Condvar,
}

impl Inner {
    #[inline]
    fn suspend(&self) {
        // Acquire the lock first
        let Ok(mut lock) = self.lock.lock() else {
            return;
        };

        // check the state the lock is in right now
        match *lock {
            // suspend the thread
            State::Empty => *lock = State::Suspended,
            // already notified this thread so just revert state back to empty
            // then return
            State::Notified => {
                *lock = State::Empty;
                return;
            }
            State::Suspended => panic!("cannot suspend a thread that is already in a suspended state"),
        }

        // suspend this thread until we get a notification
        while *lock == State::Suspended {
            lock = self.cvar.wait(lock).unwrap();
        }

        // revert state back to empty so the next time this method is called
        // it can suspend this callee thread
        *lock = State::Empty;
    }

    #[inline]
    fn resume(&self) {
        // Acquire the lock first
        let Ok(mut lock) = self.lock.lock() else {
            return;
        };
        
        // check if the state is not in a notified state yet
        if *lock != State::Notified {
            // send notification
            *lock = State::Notified;
            // resume the suspended thread
            self.cvar.notify_one();
        }
    }
}
