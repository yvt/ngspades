//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use parking_lot::Mutex;
use std::fmt::Debug;

use stygian::mempool::{MemPageId, MemPageRef, MemPool, MemStore};

/// Profiled memory pool.
#[derive(Debug)]
pub struct ProfMemPool<T> {
    inner: T,
}

#[derive(Debug)]
struct ProfMemStore<T> {
    inner: Box<dyn MemStore<T>>,
    name: Mutex<String>,
    stats: Mutex<Stats>,
}

#[derive(Debug)]
struct Stats {
    num_pages: u128,
    bytes: u128,
}

impl<T> ProfMemPool<T>
where
    T: MemPool,
{
    /// Construct a `ProfMemPool`, wrapping another `MemPool`.
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<P> MemPool for ProfMemPool<P>
where
    P: MemPool,
{
    fn new_store<T: Send + Sync + Debug + 'static>(&self) -> Box<dyn MemStore<T>> {
        Box::new(ProfMemStore {
            inner: self.inner.new_store(),
            name: Mutex::new("(unnamed)".to_owned()),
            stats: Mutex::new(Stats {
                num_pages: 0,
                bytes: 0,
            }),
        })
    }
}

impl<T: Send + Sync + Debug> MemStore<T> for ProfMemStore<T> {
    fn new_page(&self, capacity: usize) -> MemPageId<T> {
        {
            let mut stats = self.stats.lock();
            stats.num_pages += 1;
            stats.bytes += std::mem::size_of::<T>() as u128 * capacity as u128;
        }
        self.inner.new_page(capacity)
    }
    fn prefetch_page(&self, pages: &[MemPageId<T>]) {
        self.inner.prefetch_page(pages)
    }
    fn get_page(&self, page: MemPageId<T>) -> &dyn MemPageRef<T> {
        self.inner.get_page(page)
    }

    fn set_name(&self, name: &str) {
        *self.name.lock() = name.to_owned();
    }
}

impl<T> Drop for ProfMemStore<T> {
    fn drop(&mut self) {
        let name = self.name.get_mut();
        let stats = self.stats.get_mut();
        println!("Releasing '{:?}': {:?}", name, stats);
    }
}
