//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Generalization of dispatch queue systems like Apple Grand Central Dispatch.
#[cfg(feature = "xdispatch")]
use xdispatch;
#[cfg(feature = "xdispatch")]
use num_cpus;

struct SendPtr<T>(*mut T);

unsafe impl<T> Sync for SendPtr<T> {}
unsafe impl<T> Send for SendPtr<T> {}

/// Generalization of a dispatch queue.
pub unsafe trait Queue: Send + Sync {
    fn apply<F>(&self, num_iterations: usize, work: F)
    where
        F: Sync + Fn(usize);

    fn foreach<T, F>(&self, slice: &mut [T], work: F)
    where
        F: Sync + Fn(usize, &mut T),
        T: Send,
    {
        let ptr = SendPtr(slice.as_mut_ptr());
        self.apply(slice.len(), |i| {
            work(i, unsafe { &mut *ptr.0.offset(i as isize) });
        });
    }

    fn hardware_concurrency(&self) -> usize;
}

/// Serial implementation of `Queue`.
pub struct SerialQueue;

unsafe impl Queue for SerialQueue {
    fn apply<F>(&self, num_iterations: usize, work: F)
    where
        F: Sync + Fn(usize),
    {
        for i in 0..num_iterations {
            work(i);
        }
    }

    fn hardware_concurrency(&self) -> usize {
        1
    }
}

#[cfg(feature = "xdispatch")]
unsafe impl Queue for xdispatch::Queue {
    fn apply<F>(&self, num_iterations: usize, work: F)
    where
        F: Sync + Fn(usize),
    {
        self.apply(num_iterations, work)
    }

    fn hardware_concurrency(&self) -> usize {
        // This is actually true only for concurrent queues...
        num_cpus::get()
    }
}
