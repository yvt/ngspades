//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! High-performance non-thread safe object pool.
//!
//! It also provides a type akin to pointers so you can realize linked list
//! data structures on it within the "safe" Rust. Memory safety is guaranteed by
//! runtime checks.
//!
//! Performance
//! -----------
//!
//! `Pool` outperformed Rust's default allocator (jemalloc) by at least twice
//! if each thread was given an exclusive access to an individual `Pool`.
//! It is expected that it will exhibit slightly better performance characteristics
//! on the real world use due to an improved spatial locality.
//!
//! It also comes with a sacrifice. It is impossible to return a free space to
//! the global heap without destroying entire the pool.
use std::{ops, ptr, cmp};

/// High-performance non-thread safe object pool.
#[derive(Debug, Clone)]
pub struct Pool<T> {
    storage: Vec<Entry<T>>,
    first_free: Option<usize>,
}

/// A (potentially invalid) pointer to an object in `Pool`, but without
/// information about which specific `Pool` this is associated with.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct PoolFreePtr(usize);

/// A pointer to an object in `Pool`.
#[derive(Debug, Clone, Copy)]
pub struct PoolPtr<'a, T: 'a>(usize, &'a Pool<T>);

/// A mutable pointer to an object in `Pool`.
#[derive(Debug)]
pub struct PoolMutPtr<'a, T: 'a>(usize, &'a mut Pool<T>);

#[derive(Debug, Clone)]
enum Entry<T> {
    Used(T),

    /// This entry is free. Points the next free entry.
    Free(Option<usize>),
}

impl PoolFreePtr {
    /// Return an uninitialized pointer that has no guarantee regarding its
    /// usage with any `Pool`.
    ///
    /// This value can be used as a memory-efficient replacement for
    /// `Option<PoolFreePtr>` without a tag indicating whether it has a
    /// valid value or not.
    ///
    /// The returned pointer actually has a well-defined initialized value so
    /// using it will never result in an undefined behavior, hence this function
    /// is not marked with `unsafe`. It is just that it has no specific object
    /// or pool associated with it in a meaningful way.
    #[inline]
    pub fn uninitialized() -> Self {
        PoolFreePtr(0)
    }
}

impl<'a, T: 'a> cmp::PartialEq for PoolPtr<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && ptr::eq(self.1, other.1)
    }
}

impl<'a, 'b, T: 'a> From<&'b PoolPtr<'a, T>> for PoolFreePtr {
    fn from(x: &'b PoolPtr<'a, T>) -> Self {
        PoolFreePtr(x.0)
    }
}
impl<'a, 'b, T: 'a> From<&'b PoolMutPtr<'a, T>> for PoolFreePtr {
    fn from(x: &'b PoolMutPtr<'a, T>) -> Self {
        PoolFreePtr(x.0)
    }
}

impl<'a, 'b: 'a, T> From<&'b PoolMutPtr<'a, T>> for PoolPtr<'a, T> {
    fn from(x: &'b PoolMutPtr<'a, T>) -> Self {
        PoolPtr(x.0, x.1)
    }
}

impl<'a, T> ops::Deref for PoolPtr<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.1.storage[self.0].as_ref().expect("invalid pointer")
    }
}
impl<'a, T> ops::Deref for PoolMutPtr<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.1.storage[self.0].as_ref().expect("invalid pointer")
    }
}
impl<'a, T> ops::DerefMut for PoolMutPtr<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.1.storage[self.0].as_mut().expect("invalid pointer")
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
    pub fn allocate(&mut self, x: T) -> PoolFreePtr {
        match self.first_free {
            None => {
                self.storage.push(Entry::Used(x));
                PoolFreePtr(self.storage.len() - 1)
            }
            Some(i) => {
                let next_free = self.storage[i].next_free_index();
                self.first_free = next_free;
                self.storage[i] = Entry::Used(x);
                PoolFreePtr(i)
            }
        }
    }
    pub fn deallocate<S: Into<PoolFreePtr>>(&mut self, i: S) {
        let i = i.into().0;
        let ref mut e = self.storage[i];
        match e {
            &mut Entry::Used(_) => {}
            &mut Entry::Free(_) => panic!("double free"),
        }
        *e = Entry::Free(self.first_free);
        self.first_free = Some(i);
    }
    pub fn get(&self, fp: PoolFreePtr) -> PoolPtr<T> {
        PoolPtr(fp.0, self)
    }
    pub fn get_mut(&mut self, fp: PoolFreePtr) -> PoolMutPtr<T> {
        PoolMutPtr(fp.0, self)
    }
}

impl<T> ops::Index<PoolFreePtr> for Pool<T> {
    type Output = T;
    fn index(&self, index: PoolFreePtr) -> &Self::Output {
        self.storage[index.0].as_ref().expect("dangling pointer")
    }
}
impl<T> ops::IndexMut<PoolFreePtr> for Pool<T> {
    fn index_mut(&mut self, index: PoolFreePtr) -> &mut Self::Output {
        self.storage[index.0].as_mut().expect("dangling pointer")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    #[test]
    fn test() {
        let mut pool = Pool::new();
        let ptr1 = pool.allocate(1);
        let ptr2 = pool.allocate(2);
        assert_eq!(pool[ptr1], 1);
        assert_eq!(pool[ptr2], 2);
    }

    #[test]
    #[should_panic]
    fn dangling_ptr() {
        let mut pool = Pool::new();
        let ptr = pool.allocate(1);
        pool.deallocate(ptr);
        *pool.get(ptr);
    }

    struct Xorshift32(u32);

    impl Xorshift32 {
        fn next(&mut self) -> u32 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 17;
            self.0 ^= self.0 << 5;
            self.0
        }
    }

    #[bench]
    fn allocation_random_std(b: &mut Bencher) {
        let mut v = vec![None; 512];
        b.iter(|| {
            let mut r = Xorshift32(0x11451419);
            for _ in 0..65536 {
                let i = ((r.next() >> 8) & 511) as usize;
                if v[i].is_some() {
                    v[i] = None;
                } else {
                    v[i] = Some(Box::new(i));
                }
            }
            let mut sum = 0;
            for x in v.iter_mut() {
                if let Some(x) = x.take() {
                    sum += *x;
                }
            }
            sum
        });
    }

    #[bench]
    fn allocation_random_pool(b: &mut Bencher) {
        let mut v = vec![None; 512];
        let mut pool = Pool::with_capacity(512);
        b.iter(|| {
            let mut r = Xorshift32(0x11451419);
            for _ in 0..65536 {
                let i = ((r.next() >> 8) & 511) as usize;
                if v[i].is_some() {
                    pool.deallocate(v[i].take().unwrap());
                } else {
                    v[i] = Some(pool.allocate(i));
                }
            }
            let mut sum = 0;
            for x in v.iter_mut() {
                if let Some(x) = x.take() {
                    sum += pool[x];
                    pool.deallocate(x);
                }
            }
            sum
        });
    }

    #[bench]
    fn allocation_random_mt_std(b: &mut Bencher) {
        use std::thread::Builder;
        let mut states = vec![Some(vec![None; 512]); 8];
        b.iter(|| {
            let mut threads: Vec<_> = states
                .iter_mut()
                .map(|s| {
                    let mut v = s.take().unwrap();
                    Builder::new()
                        .spawn(move || {
                            let mut r = Xorshift32(0x11451419);
                            for _ in 0..655360 {
                                let i = ((r.next() >> 8) & 511) as usize;
                                if v[i].is_some() {
                                    v[i] = None;
                                } else {
                                    v[i] = Some(Box::new(i));
                                }
                            }
                            let mut sum = 0;
                            for x in v.iter_mut() {
                                if let Some(x) = x.take() {
                                    sum += *x;
                                }
                            }
                            (v, sum)
                        })
                        .expect("failed to create thread")
                })
                .collect();
            let mut sum = 0;
            for (i, handle) in threads.drain(..).enumerate() {
                let (st, sub_sum) = handle.join().unwrap();
                states[i] = Some(st);
                sum += sub_sum;
            }
            sum
        });
    }

    #[bench]
    fn allocation_random_mt_pool(b: &mut Bencher) {
        use std::thread::Builder;
        let mut states = vec![Some((vec![None; 512], Pool::with_capacity(512))); 8];
        b.iter(|| {
            let mut threads: Vec<_> = states
                .iter_mut()
                .map(|s| {
                    let (mut v, mut pool) = s.take().unwrap();
                    Builder::new()
                        .spawn(move || {
                            let mut r = Xorshift32(0x11451419);
                            for _ in 0..655360 {
                                let i = ((r.next() >> 8) & 511) as usize;
                                if v[i].is_some() {
                                    pool.deallocate(v[i].take().unwrap());
                                } else {
                                    v[i] = Some(pool.allocate(i));
                                }
                            }
                            let mut sum = 0;
                            for x in v.iter_mut() {
                                if let Some(x) = x.take() {
                                    sum += pool[x];
                                    pool.deallocate(x);
                                }
                            }
                            ((v, pool), sum)
                        })
                        .expect("failed to create thread")
                })
                .collect();
            let mut sum = 0;
            for (i, handle) in threads.drain(..).enumerate() {
                let (st, sub_sum) = handle.join().unwrap();
                states[i] = Some(st);
                sum += sub_sum;
            }
            sum
        });
    }
}
