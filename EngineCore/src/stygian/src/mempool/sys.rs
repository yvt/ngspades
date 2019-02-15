//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides a memory pool implementation based on a global memory allocator.
use parking_lot::RwLock;
use std::{
    alloc,
    cell::UnsafeCell,
    fmt,
    marker::PhantomData,
    mem::{forget, needs_drop},
    ptr::NonNull,
};

use super::{MemPageId, MemPageRef, MemPageRefExt, MemPool, MemStore};

#[derive(Debug, Copy, Clone)]
pub struct SysMemPool;

impl MemPool for SysMemPool {
    fn new_store<'a, T: Send + Sync + fmt::Debug + 'a>(&'a self) -> Box<dyn MemStore<T> + 'a> {
        Box::new(SysMemStore {
            pages: RwLock::new(Vec::new()),
        })
    }
}

pub struct SysMemStore<T> {
    pages: RwLock<Vec<Box<SysMemPage<T>>>>,
}

impl<T: Send + Sync + fmt::Debug> MemStore<T> for SysMemStore<T> {
    fn new_page(&self, capacity: usize) -> MemPageId<T> {
        let page = Box::new(SysMemPage::new(capacity));

        let mut pages = self.pages.write();
        let id = MemPageId::new(pages.len() as u64);
        pages.push(page);

        id
    }

    fn prefetch_page(&self, _pages: &[MemPageId<T>]) {
        // TODO - Maybe use `PrefetchVirtualMemory` or `madvise`
    }

    fn get_page(&self, page: MemPageId<T>) -> &dyn MemPageRef<T> {
        let pages = self.pages.read();

        // Detach the lifetime from `ReadGuard`.
        // Safety: We know `SysMemPage` is pinned
        unsafe { &*((&*pages[page.value() as usize]) as *const SysMemPage<T>) }
    }
}

impl<T: fmt::Debug> fmt::Debug for SysMemStore<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list()
            .entries(self.pages.read().iter())
            .finish()
    }
}

pub struct SysMemPage<T> {
    cell: *mut u8,
    capacity: usize,
    len: UnsafeCell<usize>,
    lock: RwLock<()>,
    _phantom: PhantomData<T>,
}

unsafe impl<T: Send> Send for SysMemPage<T> {}
unsafe impl<T: Send + Sync> Sync for SysMemPage<T> {}

unsafe impl<T> MemPageRef<T> for SysMemPage<T> {
    fn as_ptr(&self) -> *mut T {
        self.cell as *mut T
    }
    fn capacity(&self) -> usize {
        self.capacity
    }
    fn len(&self) -> NonNull<usize> {
        NonNull::new(self.len.get()).unwrap()
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

impl<T: fmt::Debug> fmt::Debug for SysMemPage<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("SysMemPage").field(&&*self.read()).finish()
    }
}

impl<T> SysMemPage<T> {
    fn new(capacity: usize) -> Self {
        Self {
            cell: unsafe { alloc::alloc(alloc::Layout::array::<T>(capacity).unwrap()) },
            capacity,
            len: UnsafeCell::new(0),
            lock: RwLock::new(()),
            _phantom: PhantomData,
        }
    }
}

impl<T> Drop for SysMemPage<T> {
    fn drop(&mut self) {
        if needs_drop::<T>() {
            // Safety: `self.get_ptr()` is valid even when unlocked
            unsafe { self.get_mut() }.clear();
        }

        // Safety: This deallocates the region previously allocated by
        //         `std::alloc::alloc` with a matching parameter.
        unsafe { alloc::dealloc(self.cell, alloc::Layout::array::<T>(self.capacity).unwrap()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;

    #[test]
    fn sysmempool_sanity() {
        let obj;
        {
            let store = SysMemPool.new_store::<Arc<usize>>();

            let page_id = store.new_page(32);

            {
                let mut lock = store.get_page(page_id).write();
                let mut vec = lock.as_vec();
                vec.push(Arc::new(1));
                vec.push(Arc::new(2));
                vec.push(Arc::new(3));
                obj = Arc::new(4);
                vec.push(Arc::clone(&obj));
            }

            {
                let lock = store.get_page(page_id).read();
                assert_eq!(*lock[0], 1);
                assert_eq!(*lock[1], 2);
                assert_eq!(*lock[2], 3);
                assert_eq!(*lock[3], 4);
            }

            dbg!(&store);
        }

        // Check for memory leak
        assert_eq!(Arc::strong_count(&obj), 1);
    }
}
