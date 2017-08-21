//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Generalization of dispatch queue systems like Apple Grand Central Dispatch.
use xdispatch;

/// Generalization of a dispatch queue.
pub trait Queue: Send + Sync {
    fn apply<F>(&self, num_iterations: usize, work: F)
    where
        F: Sync + Fn(usize);

    fn foreach<T, F>(&self, slice: &mut [T], work: F)
    where
        F: Sync + Fn(&mut T),
        T: Send;
}

/// Serial implementation of `Queue`.
pub struct SerialQueue;

impl Queue for SerialQueue {
    fn apply<F>(&self, num_iterations: usize, work: F)
    where
        F: Sync + Fn(usize)
    {
        for i in 0..num_iterations {
            work(i);
        }
    }

    fn foreach<T, F>(&self, slice: &mut [T], work: F)
    where
        F: Sync + Fn(&mut T),
        T: Send
    {
        for i in slice.iter_mut() {
            work(i);
        }
    }
}

impl Queue for xdispatch::Queue {
    fn apply<F>(&self, num_iterations: usize, work: F)
    where
        F: Sync + Fn(usize)
    {
        self.apply(num_iterations, work)
    }

    fn foreach<T, F>(&self, slice: &mut [T], work: F)
    where
        F: Sync + Fn(&mut T),
        T: Send
    {
        self.foreach(slice, work)
    }
}
