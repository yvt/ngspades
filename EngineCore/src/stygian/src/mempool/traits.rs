//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::{
    fmt::{self, Debug},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
};

/// A memory pool object.
pub trait MemPool {
    /// Create a new memory store.
    fn new_store<T: Send + Sync + Debug + 'static>(&self) -> Box<dyn MemStore<T>>;
}

/// The memory page identifier for [`MemStore`].
pub type MemPageId<T> = PageRefInner<fn() -> T>;

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

/// A memory store object created from [`MemPool`].
///
/// A memory store is a collection of memory pages. A page is a collection of
/// objects of a particular type (`T`) with a fixed capacity. A memory page may
/// or may not correspond to an operating system's memory pages.
pub trait MemStore<T>: Send + Sync + Debug {
    /// Allocate an empty page with a given capacity.
    fn new_page(&self, capacity: usize) -> MemPageId<T>;

    /// Asynchronously load specified pages into memory.
    fn prefetch_page(&self, _pages: &[MemPageId<T>]) {}

    /// Get a `MemPageRef` representing a memory page.
    fn get_page(&self, page: MemPageId<T>) -> &dyn MemPageRef<T>;

    /// Set the name of a memory store for debugging/profiling uses.
    fn set_name(&self, _name: &str) {}
}

/// Provides a low-level API for accessing the contents of [`MemStore`]'s memory
/// page.
///
/// See [`MemPageRefExt`] for the high-level API.
pub unsafe trait MemPageRef<T> {
    /// Get the pointer to the contents.
    ///
    /// The returned pointer is required to be valid only if the page is
    /// currently locked via `lock_read` or `lock_write`. Furthermore, the
    /// pointer is allowed to move when it's not locked.
    fn as_ptr(&self) -> *mut T;
    /// Get the size of the region pointed by `as_ptr`, measured in the number
    /// of `T`s.
    fn capacity(&self) -> usize;
    /// Get the pointer to a variable storing the number of the valid elements
    /// in `as_ptr()`.
    ///
    /// The pointed value is initialized to `0`. The high-level API manages
    /// the value.
    fn len(&self) -> NonNull<usize>;
    fn lock_read(&self);
    fn lock_write(&self);
    unsafe fn unlock_read(&self);
    unsafe fn unlock_write(&self);
}

/// An extension trait for [`MemPageRef`].
pub trait MemPageRefExt<T>: MemPageRef<T> {
    /// Acquire a reader lock. This may trigger synchronous load.
    fn read(&self) -> ReadGuard<Self, T>;

    /// Acquire a writer lock. This may trigger synchronous load.
    ///
    /// This method returns a lock guard that derefs to `[T]`.
    /// Use [`WriteGuard::as_vec`] to retrieve an accessor that can resize
    /// the storage (up to the predetermined capacity).
    fn write(&self) -> WriteGuard<Self, T>;

    /// Get an accessor through a mutable reference.
    ///
    /// This method is marked as `unsafe` because it provides a mean to
    /// dereference `self.as_ptr()` without acquiring a lock. The returned
    /// accessor assumes that the returned pointer is valid and stable.
    ///
    /// This method is useful during clean up of a memory page.
    unsafe fn get_mut(&mut self) -> MemPageVec<T>;
}

impl<T, P: MemPageRef<T> + ?Sized> MemPageRefExt<T> for P {
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

    unsafe fn get_mut(&mut self) -> MemPageVec<T> {
        MemPageVec {
            storage: self.as_ptr(),
            len: &mut *self.len().as_ptr(),
            capacity: self.capacity(),
            _phantom: PhantomData,
        }
    }
}

pub struct ReadGuard<'a, P: MemPageRef<T> + ?Sized, T> {
    page: &'a P,
    _phantom: PhantomData<*mut T>,
}

pub struct WriteGuard<'a, P: MemPageRef<T> + ?Sized, T> {
    page: &'a P,
    _phantom: PhantomData<*mut T>,
}

unsafe impl<P, T: Sync> Sync for ReadGuard<'_, P, T> where P: MemPageRef<T> + ?Sized {}
unsafe impl<P, T: Sync> Sync for WriteGuard<'_, P, T> where P: MemPageRef<T> + ?Sized {}

impl<P, T> Drop for ReadGuard<'_, P, T>
where
    P: MemPageRef<T> + ?Sized,
{
    fn drop(&mut self) {
        unsafe {
            self.page.unlock_read();
        }
    }
}

impl<P, T> Deref for ReadGuard<'_, P, T>
where
    P: MemPageRef<T> + ?Sized,
{
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.page.as_ptr(), *self.page.len().as_ref()) }
    }
}

impl<P, T: Debug> Debug for ReadGuard<'_, P, T>
where
    P: MemPageRef<T> + ?Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("ReadGuard").field(&&**self).finish()
    }
}

impl<P, T: Debug> Debug for WriteGuard<'_, P, T>
where
    P: MemPageRef<T> + ?Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("WriteGuard").field(&&**self).finish()
    }
}

impl<P, T> Drop for WriteGuard<'_, P, T>
where
    P: MemPageRef<T> + ?Sized,
{
    fn drop(&mut self) {
        unsafe {
            self.page.unlock_write();
        }
    }
}

impl<P, T> WriteGuard<'_, P, T>
where
    P: MemPageRef<T> + ?Sized,
{
    /// Get a `Vec`-like read-write accessor.
    pub fn as_vec(&mut self) -> MemPageVec<T> {
        MemPageVec {
            storage: self.page.as_ptr(),
            len: unsafe { &mut *self.page.len().as_ptr() },
            capacity: self.page.capacity(),
            _phantom: PhantomData,
        }
    }
}

impl<P, T> Deref for WriteGuard<'_, P, T>
where
    P: MemPageRef<T> + ?Sized,
{
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.page.as_ptr(), *self.page.len().as_ref()) }
    }
}

impl<P, T> DerefMut for WriteGuard<'_, P, T>
where
    P: MemPageRef<T> + ?Sized,
{
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.page.as_ptr(), *self.page.len().as_ref()) }
    }
}

/// `Vec`-like read-write accessor of a memory page. Created by [`WriteGuard::as_vec()`].
pub struct MemPageVec<'a, T> {
    storage: *mut T,
    len: &'a mut usize,
    capacity: usize,
    _phantom: PhantomData<T>,
}

impl<T> MemPageVec<'_, T> {
    pub fn push(&mut self, x: T) {
        self.reserve(1);
        unsafe {
            ptr::write(self.storage.offset((*self.len) as isize), x);
        }
        *self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if *self.len > 0 {
            *self.len -= 1;
            let x = unsafe { ptr::read(self.storage.offset((*self.len) as isize)) };
            Some(x)
        } else {
            None
        }
    }

    pub fn truncate(&mut self, new_len: usize) {
        if *self.len > new_len {
            let old_len = std::mem::replace(self.len, new_len);
            unsafe {
                ptr::drop_in_place(std::slice::from_raw_parts_mut(
                    self.storage,
                    old_len - new_len,
                ));
            }
        }
    }
    pub fn clear(&mut self) {
        self.truncate(0);
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn reserve(&mut self, count: usize) {
        assert!(self.len.checked_add(count).expect("count overflow") <= self.capacity);
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.storage, *self.len) }
    }
    pub fn as_slice_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.storage, *self.len) }
    }
}

impl<T> Deref for MemPageVec<'_, T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}
impl<T> DerefMut for MemPageVec<'_, T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_slice_mut()
    }
}

impl<T> std::iter::Extend<T> for MemPageVec<'_, T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for x in iter {
            self.push(x);
        }
    }
}
