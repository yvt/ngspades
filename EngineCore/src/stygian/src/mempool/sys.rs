//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides a memory pool implementation based on a global memory allocator.
use parking_lot::RwLock;
use std::{
    cell::UnsafeCell,
    mem::{forget, zeroed},
};

use super::{MemPage, MemPool, MemStore, PageRef};

#[derive(Debug, Copy, Clone)]
pub struct SysMemPool;

impl MemPool for SysMemPool {
    fn new_store<'a, T: Send + Sync + 'a>(&'a self) -> Box<dyn MemStore<T> + 'a> {
        Box::new(SysMemStore {
            pages: RwLock::new(Vec::new()),
        })
    }
}

#[derive(Debug)]
pub struct SysMemStore<T> {
    pages: RwLock<Vec<Box<SysMemPage<T>>>>,
}

impl<T: Send + Sync> MemStore<T> for SysMemStore<T> {
    fn new_pages(&self, out_page: &mut [PageRef<T>], initer: &mut dyn FnMut(*mut T)) {
        let mut pages = self.pages.write();
        pages.reserve(out_page.len());

        for out_page in out_page.iter_mut() {
            *out_page = PageRef::new(pages.len() as u64);

            let page = Box::new(SysMemPage {
                cell: unsafe { zeroed() },
                lock: RwLock::new(()),
            });
            initer(page.cell.get());

            pages.push(page);
        }
    }

    fn prefetch_page(&self, _pages: &[PageRef<T>]) {
        // TODO - Maybe use `PrefetchVirtualMemory` or `madvise`
    }

    fn get_page(&self, page: PageRef<T>) -> &dyn MemPage<T> {
        let pages = self.pages.read();

        // Detach the lifetime from `ReadGuard` - we know `SysMemPage` is
        // pinned
        unsafe { &*((&*pages[page.value() as usize]) as *const SysMemPage<T>) }
    }
}

#[derive(Debug)]
pub struct SysMemPage<T> {
    cell: UnsafeCell<T>,
    lock: RwLock<()>,
}

unsafe impl<T: Send> Send for SysMemPage<T> {}
unsafe impl<T: Send + Sync> Sync for SysMemPage<T> {}

unsafe impl<T> MemPage<T> for SysMemPage<T> {
    fn as_ptr(&self) -> *mut T {
        self.cell.get()
    }
    fn lock_read(&self) {
        forget(self.lock.read());
    }
    fn lock_write(&self) {
        forget(self.lock.write());
    }
    unsafe fn unlock_read(&self) {
        self.lock.force_unlock_read();
    }
    unsafe fn unlock_write(&self) {
        self.lock.force_unlock_write();
    }
}
