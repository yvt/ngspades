//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
// (copied and modified from NgsGFX)
//! High-performance non-thread safe object pool.
//!
//! It also provides a type akin to pointers so you can realize linked list
//! data structures on it within the "safe" Rust. Memory safety is guaranteed by
//! runtime checks.

/// High-performance non-thread safe object pool.
use std::mem;
#[derive(Debug, Clone)]
pub struct Pool<T> {
    storage: Vec<Entry<T>>,
    first_free: Option<usize>,
}

/// A (potentially invalid) pointer to an object in `Pool`, but without
/// information about which specific `Pool` this is associated with.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct PoolPtr(pub usize);

#[derive(Debug, Clone)]
enum Entry<T> {
    Used(T),

    /// This entry is free. Points the next free entry.
    Free(Option<usize>),
}

impl PoolPtr {
    /// Return an uninitialized pointer that has no guarantee regarding its
    /// usage with any `Pool`.
    ///
    /// This value can be used as a memory-efficient replacement for
    /// `Option<PoolPtr>` without a tag indicating whether it has a
    /// valid value or not.
    ///
    /// The returned pointer actually has a well-defined initialized value so
    /// using it will never result in an undefined behavior, hence this function
    /// is not marked with `unsafe`. It is just that it has no specific object
    /// or pool associated with it in a meaningful way.
    #[inline]
    pub fn uninitialized() -> Self {
        PoolPtr(0)
    }
}

impl<T> Entry<T> {
    fn as_ref(&self) -> Option<&T> {
        match self {
            &Entry::Used(ref value) => Some(value),
            &Entry::Free(_) => None,
        }
    }
    fn as_mut(&mut self) -> Option<&mut T> {
        match self {
            &mut Entry::Used(ref mut value) => Some(value),
            &mut Entry::Free(_) => None,
        }
    }
    fn next_free_index(&self) -> Option<usize> {
        match self {
            &Entry::Used(_) => unreachable!(),
            &Entry::Free(i) => i,
        }
    }
}

impl<T> Pool<T> {
    pub fn new() -> Self {
        Pool::with_capacity(0)
    }
    pub fn with_capacity(capacity: usize) -> Self {
        let mut pool = Self {
            storage: Vec::with_capacity(capacity),
            first_free: None,
        };
        if capacity > 0 {
            for i in 0..capacity - 1 {
                pool.storage.push(Entry::Free(Some(i + 1)));
            }
            pool.storage.push(Entry::Free(None));
            pool.first_free = Some(0);
        }
        pool
    }
    pub fn reserve(&mut self, additional: usize) {
        if additional == 0 {
            return;
        }
        let existing_surplus = if self.first_free.is_some() {
            1 // at least one
        } else {
            0
        } + self.storage.capacity() - self.storage.len();
        if additional > existing_surplus {
            let needed_surplus = self.storage.capacity() - self.storage.len() +
                (additional - existing_surplus);
            self.storage.reserve(needed_surplus);
        }
    }
    pub fn allocate(&mut self, x: T) -> PoolPtr {
        match self.first_free {
            None => {
                self.storage.push(Entry::Used(x));
                PoolPtr(self.storage.len() - 1)
            }
            Some(i) => {
                let next_free = self.storage[i].next_free_index();
                self.first_free = next_free;
                self.storage[i] = Entry::Used(x);
                PoolPtr(i)
            }
        }
    }
    pub fn deallocate<S: Into<PoolPtr>>(&mut self, i: S) -> Option<T> {
        let i = i.into().0;
        let ref mut e = self.storage[i];
        match e {
            &mut Entry::Used(_) => {}
            &mut Entry::Free(_) => {
                return None;
            }
        }
        let x = match mem::replace(e, Entry::Free(self.first_free)) {
            Entry::Used(x) => x,
            Entry::Free(_) => unreachable!(),
        };
        self.first_free = Some(i);
        Some(x)
    }
    pub fn get(&self, fp: PoolPtr) -> Option<&T> {
        self.storage[fp.0].as_ref()
    }
    pub fn get_mut(&mut self, fp: PoolPtr) -> Option<&mut T> {
        self.storage[fp.0].as_mut()
    }
}
