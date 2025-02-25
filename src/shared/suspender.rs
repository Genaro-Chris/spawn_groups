use std::{
    sync::{Arc, Condvar},
    task::Waker,
};

use crate::shared::mutex::StdMutex;

use super::waker::waker_helper;

pub(crate) fn pair() -> (Arc<Suspender>, Waker) {
    let suspender = Arc::new(Suspender::new());
    let resumer = suspender.clone();
    (suspender, waker_helper(resumer))
}

pub(crate) struct Suspender {
    inner: Inner,
}

impl Suspender {
    pub(crate) fn new() -> Suspender {
        Suspender {
            inner: Inner {
                lock: StdMutex::new(State::Initial),
                cvar: Condvar::new(),
            },
        }
    }

    pub(crate) fn suspend(&self) {
        self.inner.suspend();
    }

    pub(crate) fn resume(&self) {
        self.inner.resume();
    }
}

#[derive(PartialEq)]
enum State {
    Initial,
    Notified,
    Suspended,
}

struct Inner {
    lock: StdMutex<State>,
    cvar: Condvar,
}

impl Inner {
    fn suspend(&self) {
        // Acquire the lock first
        let mut lock = self.lock.lock();

        // check the state the lock is in right now
        match *lock {
            // suspend the thread
            State::Initial => {
                *lock = State::Suspended;
                // suspend this thread until we get a notification
                while *lock == State::Suspended {
                    lock = self.cvar.wait(lock).unwrap();
                }
            }
            // already notified this thread so just revert state back to empty
            // then return
            State::Notified => {
                *lock = State::Initial;
            }
            State::Suspended => {
                panic!("cannot suspend a thread that is already in a suspended state")
            }
        }
    }

    fn resume(&self) {
        // Acquire the lock first
        let mut lock = self.lock.lock();

        // check if the state is empty or suspended
        match *lock {
            State::Initial => {
                // send notification
                *lock = State::Notified;
            }
            State::Suspended => {
                // send notification
                *lock = State::Notified;
                // resume the suspended thread
                self.cvar.notify_one();
            }
            _ => {}
        }
    }
}
