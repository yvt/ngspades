//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// A memory pool object.
pub trait MemPool {
    /// Create a new memory store.
    fn new_store<'a, T: Send + Sync + 'a>(&'a self) -> Box<dyn MemStore<T> + 'a>;
}

/// The memory page identifier for [`MemStore`].
pub type PageRef<T> = PageRefInner<fn() -> T>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageRefInner<T>(u64, PhantomData<T>);

impl<T> PageRefInner<T> {
    pub fn new(i: u64) -> Self {
        Self(i, PhantomData)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl<T> Default for PageRefInner<T> {
    fn default() -> Self {
        Self(0, PhantomData)
    }
}

/// A memory store object.
///
/// A memory store is a collection of memory pages each storing an object of
/// a particular type (`T`). A memory page may or may not correspond to an
/// operating system's memory pages.
pub trait MemStore<T: Send + Sync>: Send + Sync {
    /// Allocate pages. The pages are initialized in-place using `initer`.
    fn new_pages(&self, out_page: &mut [PageRef<T>], initer: &mut dyn FnMut(*mut T));

    /// Asynchronously load specified pages into memory.
    fn prefetch_page(&self, _pages: &[PageRef<T>]) {}

    /// Get a `MemPage` representing a memory page.
    fn get_page(&self, page: PageRef<T>) -> &dyn MemPage<T>;
}

/// An extension trait for `MemStore`.
pub trait MemStoreExt<T: Send + Sync>: MemStore<T> {
    /// Allocate a page. The page is initialized in-place using `initer`.
    ///
    /// `initer` must not panic.
    fn new_page(&self, initer: &mut dyn FnMut(*mut T)) -> PageRef<T>;

    /// Allocate pages. The pages are initialized in-place using the
    /// `InPlaceDefault` implementation of `T`.
    fn new_pages_default(&self, out_page: &mut [PageRef<T>])
    where
        T: InPlaceDefault;

    /// Allocate a page. The page is initialized in-place using the
    /// `InPlaceDefault` implementation of `T`.
    fn new_page_default(&self) -> PageRef<T>
    where
        T: InPlaceDefault;
}

impl<T: Send + Sync, S: MemStore<T> + ?Sized> MemStoreExt<T> for S {
    fn new_page(&self, initer: &mut dyn FnMut(*mut T)) -> PageRef<T> {
        let mut out = [PageRef::default()];
        self.new_pages(&mut out, initer);
        out[0]
    }

    fn new_pages_default(&self, out_page: &mut [PageRef<T>])
    where
        T: InPlaceDefault,
    {
        self.new_pages(out_page, &mut T::default_in_place)
    }

    fn new_page_default(&self) -> PageRef<T>
    where
        T: InPlaceDefault,
    {
        self.new_page(&mut T::default_in_place)
    }
}

/// In-place default initialization.
pub unsafe trait InPlaceDefault {
    /// Default-initialize `Self` in-place in `this`. It can be assumed that
    /// `this` is already zero-initialized.
    ///
    /// Implementations must not panic.
    fn default_in_place(this: *mut Self);
}

/// Provides a low-level API for accessing the contents of [`MemStore`]'s memory
/// page.
pub unsafe trait MemPage<T> {
    fn as_ptr(&self) -> *mut T;
    fn lock_read(&self);
    fn lock_write(&self);
    unsafe fn unlock_read(&self);
    unsafe fn unlock_write(&self);
}

/// An extension trait for [`MemPage`].
pub trait MemPageExt<T>: MemPage<T> {
    /// Acquire a reader lock. This may trigger synchronous load.
    fn read(&self) -> ReadGuard<Self, T>;
    /// Acquire a writer lock. This may trigger synchronous load.
    fn write(&self) -> WriteGuard<Self, T>;
}

impl<T, P: MemPage<T> + ?Sized> MemPageExt<T> for P {
    fn read(&self) -> ReadGuard<Self, T> {
        self.lock_read();
        ReadGuard {
            page: self,
            _phantom: PhantomData,
        }
    }
    fn write(&self) -> WriteGuard<Self, T> {
        self.lock_write();
        WriteGuard {
            page: self,
            _phantom: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct ReadGuard<'a, P: MemPage<T> + ?Sized, T> {
    page: &'a P,
    _phantom: PhantomData<*mut T>,
}

#[derive(Debug)]
pub struct WriteGuard<'a, P: MemPage<T> + ?Sized, T> {
    page: &'a P,
    _phantom: PhantomData<*mut T>,
}

unsafe impl<P, T: Sync> Sync for ReadGuard<'_, P, T> where P: MemPage<T> + ?Sized {}
unsafe impl<P, T: Sync> Sync for WriteGuard<'_, P, T> where P: MemPage<T> + ?Sized {}

impl<P, T> Drop for ReadGuard<'_, P, T>
where
    P: MemPage<T> + ?Sized,
{
    fn drop(&mut self) {
        unsafe {
            self.page.unlock_read();
        }
    }
}

impl<P, T> Deref for ReadGuard<'_, P, T>
where
    P: MemPage<T> + ?Sized,
{
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.page.as_ptr() }
    }
}

impl<P, T> Drop for WriteGuard<'_, P, T>
where
    P: MemPage<T> + ?Sized,
{
    fn drop(&mut self) {
        unsafe {
            self.page.unlock_write();
        }
    }
}

impl<P, T> Deref for WriteGuard<'_, P, T>
where
    P: MemPage<T> + ?Sized,
{
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.page.as_ptr() }
    }
}

impl<P, T> DerefMut for WriteGuard<'_, P, T>
where
    P: MemPage<T> + ?Sized,
{
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.page.as_ptr() }
    }
}
