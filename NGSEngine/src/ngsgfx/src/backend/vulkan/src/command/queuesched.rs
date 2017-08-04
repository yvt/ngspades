//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Tracks dependencies between internal queues and schedules the submission
//! in the right order.
//!
//! On Vulkan, command buffers are submitted to a device queue in a batch.
//! Inter-queue execution dependencies can be expressed using semaphores
//! specified as a part of the submission batch.
//! However, in order to make them actually work, a batch that signals a
//! semaphore must be submitted *before* another batch that waits on the same
//! semaphore.
//! This means we have to choose an appropriate order to submit batches, and we
//! occasionally need to split the batch to break up the circular reference.
//!
//! To decide the right order, we use a greedy algorithm for the sake of
//! runtime efficiency. Basically, we go through the list of command passes,
//! and whenever we encounter a fence that waits on an other internal queue,
//! we add an edge to the dependency graph. If the addition would create a
//! strongly connected component, we first split the destination batch at that
//! point.
//!
use std::mem;

use ngsgfx_common::int::BinaryInteger;
use imp::MAX_NUM_QUEUES;

#[derive(Debug, Clone)]
pub struct QueueScheduler {
    /// An adjacency matrix of a directed graph.
    ///
    /// `dep[i].get_bit(j)` iff the batch `i` has an execution dependency to
    /// the batch `j`, hence must be submitted later than `j`.
    ///
    /// Invariant: the graph must not contain a strongly connected component.
    dep: [u32; MAX_NUM_QUEUES],
}

impl QueueScheduler {
    pub fn new() -> Self {
        QueueScheduler { dep: Default::default() }
    }

    /// Add a dependency from the batch `from` to the ones `to_bits`.
    ///
    /// Returns whether the addition was successful.
    pub fn insert(&mut self, from: u32, to_bits: u32) -> bool {
        assert!(!to_bits.get_bit(from));
        assert!(from < MAX_NUM_QUEUES as u32);

        if to_bits == 0 {
            return true;
        }

        // Checks if a path from `to` to `from` exists
        let mut reachable = to_bits;
        let mut old_reachable = 0u32;

        loop {
            if reachable.get_bit(from) {
                return false;
            }
            let mut new_reachable = reachable;
            for i in (reachable & !old_reachable).one_digits() {
                new_reachable |= self.dep[i as usize];
            }
            if new_reachable == reachable {
                break;
            }
            old_reachable = reachable;
            reachable = new_reachable;
        }

        self.dep[from as usize] |= to_bits;
        true
    }

    /// Enumerate batches on which the batch `queue` has the execution
    /// dependency and clear all relevant dependencies.
    ///
    /// `cb` is called for all batches `queue` depends on as well as for `queue`.
    pub fn resolve<F>(&mut self, queue: u32, mut cb: F)
    where
        F: FnMut(u32),
    {
        let mut visited = 0u32;
        visited.set_bit(queue);

        let mut list = [0; MAX_NUM_QUEUES];
        let mut list_len = 0;

        let mut stack = [0; MAX_NUM_QUEUES];
        let mut stack_len = 1;
        stack[0] = queue;

        while stack_len > 0 {
            stack_len -= 1;
            let cur = stack[stack_len];

            list[list_len] = cur;
            list_len += 1;

            let dep = mem::replace(&mut self.dep[cur as usize], 0);
            for i in dep.one_digits() {
                if !visited.get_bit(i) {
                    visited.set_bit(i);

                    stack[stack_len] = i;
                    stack_len += 1;
                }
            }
        }

        while list_len > 0 {
            list_len -= 1;
            cb(list[list_len]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create() {
        QueueScheduler::new();
    }

    #[test]
    fn well_founded_order1() {
        let mut s = QueueScheduler::new();
        for i in 0..MAX_NUM_QUEUES - 1 {
            for j in i + 1..MAX_NUM_QUEUES {
                assert!(
                    s.insert(i as u32, 1u32 << j),
                    "adding the edge {} -> [ {} ] failed",
                    i,
                    j
                );
            }
        }
        println!("well_founded_order1: {:#?}", &s);
    }

    #[test]
    fn well_founded_order2() {
        let mut s = QueueScheduler::new();
        for i in 1..MAX_NUM_QUEUES {
            for j in 0..i {
                assert!(
                    s.insert(i as u32, 1u32 << j),
                    "adding the edge {} -> [ {} ] failed",
                    i,
                    j
                );
            }
        }
        println!("well_founded_order2: {:#?}", &s);
    }

    #[test]
    fn detect_cycle() {
        let mut s = QueueScheduler::new();
        assert!(s.insert(0, 0b0010), "adding the edge 0 -> { 1 } failed");

        println!("detect_cycle: 0: {:#?}", &s);

        assert!(
            !s.insert(1, 0b0001),
            "adding the edge 1 -> { 0 } didn't fail"
        );

        println!("detect_cycle: 1: {:#?}", &s);

        let mut v = Vec::new();
        s.resolve(0, |iq| { v.push(iq); });
        assert_eq!(v, vec![1, 0]);

        println!("detect_cycle: 2: {:#?}", &s);

        assert!(s.insert(1, 0b0001), "adding the edge 1 -> { 0 } failed");

        println!("detect_cycle: 3: {:#?}", &s);
    }
}
