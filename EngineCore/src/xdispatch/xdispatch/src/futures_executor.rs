extern crate futures;

use self::futures::{
    future::FutureObj,
    prelude::*,
    task::{local_waker, Poll, Spawn, SpawnError, Wake},
};
use std::{
    cell::UnsafeCell,
    pin::Pin,
    sync::{
        atomic::{fence, AtomicUsize, Ordering},
        Arc,
    },
};

use crate::{Queue, RunOnce};

struct Task {
    future: UnsafeCell<FutureObj<'static, ()>>,
    state: AtomicUsize,
    queue: Queue,
}

const STATE_POLLING: usize = 1;
const STATE_WOKEN: usize = 2;

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl RunOnce<()> for Arc<Task> {
    type Output = ();

    fn run_once(self, _: ()) {
        // Keep a reference to fields after `self` is converted to `LocalWaker`.
        // This is okay since the `LocalWaker` is like `Arc<Self>` except
        // for type ereasure.
        let state = unsafe { &*((&self.state) as *const AtomicUsize) };
        let future = unsafe { &mut *self.future.get() };

        // Convert `self` to `LocalWaker`. This is okay because
        // `<Task as Wake>::wake_local>` uses a provided implementation (we don't
        // handle local thread cases specially).
        let waker = unsafe { local_waker(self) };

        loop {
            state.store(STATE_POLLING, Ordering::Relaxed);

            match Pin::new(&mut *future).poll(&waker) {
                Poll::Ready(()) => return,
                Poll::Pending => {}
            }

            let old = state.compare_and_swap(STATE_POLLING, 0, Ordering::Relaxed);
            debug_assert!((old & STATE_POLLING) != 0);

            if old == STATE_POLLING {
                debug_assert!((old & STATE_WOKEN) == 0);
                break;
            }

            // The task was woken up while `poll` is being called. Call it again.
            debug_assert_eq!(old, STATE_POLLING | STATE_WOKEN);
            fence(Ordering::Acquire); // Synchronize with `fetch_or` that woke up the task
        }
    }
}

impl Wake for Task {
    fn wake(arc_self: &Arc<Self>) {
        if arc_self.state.fetch_or(STATE_WOKEN, Ordering::Release) == 0 {
            arc_self.queue.async(Arc::clone(arc_self));
        }
    }
}

impl Spawn for &Queue {
    fn spawn_obj(&mut self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
        self.async(Arc::new(Task {
            future: UnsafeCell::new(future),
            state: AtomicUsize::new(STATE_WOKEN),
            queue: Queue::clone(self),
        }));
        Ok(())
    }
}

impl Spawn for Queue {
    fn spawn_obj(&mut self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
        (&*self).spawn_obj(future)
    }
}
