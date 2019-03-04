//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides a trait useful for generalizing a method's receiver (`self`) type.
//!
//! Certain methods, especially those returning `impl Future + 'self`, need to
//! retain an object that allows mutably borrowing `Self` for an extended
//! period of time. Whether this permits the direct possession of `&mut Self` or
//! not depends on specific situations. For example, unit tests or simple
//! programs probably want to use `&mut Self` as the receiver, thus extending
//! the mutable borrow until the returned `Future` expires. On the other hand,
//! in a situation involving multiple threads, `Arc<Mutex<Self>>` would be
//! more appropriate as the receiver. The trait provided by this crate makes
//! it possible to make a generic method accepting various kinds of receiver
//! types.
//!
//! # Examples
//!
//! Consider the following motivational code (though in real world code
//! `impl FnOnce` is likely to be `impl Future`):
//!
//!     fn hoge<'a>(x: &'a mut u32) -> impl FnOnce() -> u32 + 'a {
//!         move || { *x += 1; *x }
//!     }
//!
//!     let mut counter = 1;
//!     let a = hoge(&mut counter);
//!     assert_eq!(a(), 2);
//!     assert_eq!(counter, 2);
//!
//! This function can be rewritten to accept other kinds of (smart) references:
//!
//!     use lockable::BorrowLock;
//!     fn hoge<'a>(
//!         mut x: impl BorrowLock<Inner = u32> + 'a
//!     ) -> impl FnOnce() -> u32 + 'a {
//!         move || { *x.borrow_lock() += 1; *x.borrow_lock() }
//!     }
//!
//!     // &mut u32
//!     let mut counter = 1;
//!     let a = hoge(&mut counter);
//!     assert_eq!(a(), 2);
//!
//!     // &lock_api::Mutex<u32>
//!     use parking_lot::Mutex;
//!     let counter = Mutex::new(1);
//!     let a = hoge(&counter);
//!     let b = hoge(&counter);
//!     assert_eq!(a(), 2);
//!     assert_eq!(b(), 3);
//!
//!     // Arc<lock_api::Mutex<u32>>
//!     use std::sync::Arc;
//!     let (a, b);
//!     {
//!         let counter = Arc::new(Mutex::new(1));
//!         a = hoge(counter.clone());
//!         b = hoge(counter.clone());
//!     }
//!     assert_eq!(a(), 2);
//!     assert_eq!(b(), 3);
//!
//! However, even the `arbitrary_self_types` feature does not allow
//! `impl BorrowLock` to be actually used in the `self` position. Because of
//! this, it's advisable to provide wrapper methods that take `&mut self`:
//!
//!     use lockable::BorrowLock;
//!     struct Counter(u32);
//!     impl Counter {
//!         fn incrementer<'a>(
//!             mut this: impl BorrowLock<Inner = Self> + 'a
//!         ) -> impl FnOnce() -> u32 + 'a {
//!             move || { this.borrow_lock().0 += 1; this.borrow_lock().0 }
//!         }
//!
//!         fn incrementer_mut<'a>(&'a mut self) -> impl FnOnce() -> u32 + 'a {
//!             Self::incrementer(self)
//!         }
//!     }
//!
//!     let mut counter = Counter(1);
//!     let a = counter.incrementer_mut();
//!     assert_eq!(a(), 2);

use std::{
    ops::{Deref, DerefMut},
    pin::Pin,
    rc::Rc,
    sync::Arc,
};

/// Extends the notion of `BorrowMut` to types with run-time borrow uniqueness
/// checking.
pub unsafe trait BorrowLock {
    /// The type of the inner object.
    type Inner;

    /// Acquire a lock and get a pointer to the inner object.
    ///
    /// If `self` is already locked, there are two possible consequences
    /// depending on the implementation: (a) the current thread is blocked until
    /// a lock can be acquired; or (b) a panic.
    fn raw_lock(&mut self) -> *mut Self::Inner;

    /// Release an acquired lock.
    ///
    /// # Safety
    ///
    ///  - The calling thread must have a lock acquired on `self`.
    ///
    unsafe fn raw_unlock(&mut self);

    /// Acquire a lock and return an RAII lock guard.
    fn borrow_lock(&mut self) -> BorrowLockGuard<Self>
    where
        Self: Sized,
    {
        let ptr = self.raw_lock();
        BorrowLockGuard { lock: self, ptr }
    }
}

unsafe impl<T> BorrowLock for &mut T {
    type Inner = T;
    fn raw_lock(&mut self) -> *mut T {
        *self
    }
    unsafe fn raw_unlock(&mut self) {}
}

// `for impl Deref<Target = lock_api::Mutex<_, _>> + !DerefMut`
macro_rules! impl_borrow_lock_lock_api_mutex {
    ($t:ty) => {
        unsafe impl<R: lock_api::RawMutex, T> BorrowLock for $t {
            type Inner = T;
            fn raw_lock(&mut self) -> *mut T {
                let mut guard = (**self).lock();
                let ptr = (&mut *guard) as *mut _;
                std::mem::forget(guard);
                ptr
            }
            unsafe fn raw_unlock(&mut self) {
                self.force_unlock();
            }
        }
    };
}
impl_borrow_lock_lock_api_mutex!(&lock_api::Mutex<R, T>);
impl_borrow_lock_lock_api_mutex!(Arc<lock_api::Mutex<R, T>>);
impl_borrow_lock_lock_api_mutex!(Rc<lock_api::Mutex<R, T>>);
impl_borrow_lock_lock_api_mutex!(Pin<Arc<lock_api::Mutex<R, T>>>);
impl_borrow_lock_lock_api_mutex!(Pin<Rc<lock_api::Mutex<R, T>>>);

// I wanted to add `impl BorrowLock` for `RefCell`, but `RefCell` doesn't have a
// `force_unlock` equivalent...

/// The lock guard of [`BorrowLock`].
#[derive(Debug)]
pub struct BorrowLockGuard<'a, L: BorrowLock> {
    lock: &'a mut L,
    ptr: *mut L::Inner,
}

unsafe impl<'a, L: BorrowLock> Sync for BorrowLockGuard<'a, L> where L::Inner: Sync {}

impl<'a, L: BorrowLock> Drop for BorrowLockGuard<'a, L> {
    fn drop(&mut self) {
        unsafe {
            self.lock.raw_unlock();
        }
    }
}

impl<'a, L: BorrowLock> Deref for BorrowLockGuard<'a, L> {
    type Target = L::Inner;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<'a, L: BorrowLock> DerefMut for BorrowLockGuard<'a, L> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ptr }
    }
}
