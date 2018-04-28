//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides a container type `ArcLock` similar to `Mutex` but whose lock guard
//! type is `'static`.
#[cfg(feature = "owning_ref")]
extern crate owning_ref;

use std::fmt;
use std::mem::transmute;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard, LockResult, TryLockResult, TryLockError, PoisonError};
use std::cell::UnsafeCell;

/// A container type similar to `Mutex` but whose lock guard type is `'static`.
pub struct ArcLock<T: ?Sized> {
    inner: Arc<Inner<T>>,
}

#[derive(Debug)]
pub struct ArcLockGuard<T: ?Sized> {
    inner: Arc<Inner<T>>,

    /// Lock guard obtained from `Inner::mutex` and transmuted to extend its
    /// lifetime. We must ensure it is dropped before the originating mutex
    /// hence `inner` is dropped.
    guard: Option<MutexGuard<'static, ()>>,
}

#[derive(Debug)]
struct Inner<T: ?Sized> {
    mutex: Mutex<()>,
    cell: UnsafeCell<T>,
}

unsafe impl<T> Sync for ArcLock<T> {}
unsafe impl<T: Send> Send for ArcLock<T> {}

unsafe impl<T: Sync> Sync for ArcLockGuard<T> {}
unsafe impl<T: Send> Send for ArcLockGuard<T> {}

impl<T> ArcLock<T> {
    pub fn new(x: T) -> Self {
        Self {
            inner: Arc::new(Inner {
                mutex: Mutex::new(()),
                cell: UnsafeCell::new(x),
            }),
        }
    }
}

impl<T: ?Sized> ArcLock<T> {
    /// Retrieve a mutable reference to the inner value, if the `ArcLock` is
    /// not locked currently.
    pub fn try_get_mut(&mut self) -> Option<&mut T> {
        Arc::get_mut(&mut self.inner).map(|inner| unsafe { &mut *inner.cell.get() })
    }

    pub fn lock(&self) -> LockResult<ArcLockGuard<T>> {
        let mk_guard = |guard: MutexGuard<'static, ()>| {
            ArcLockGuard {
                inner: self.inner.clone(),
                guard: Some(guard),
            }
        };
        match self.inner.mutex.lock() {
            Ok(guard) => Ok(mk_guard(unsafe { transmute(guard) })),
            Err(error) => Err(PoisonError::new(
                mk_guard(unsafe { transmute(error.into_inner()) }),
            )),
        }
    }

    pub fn try_lock(&self) -> TryLockResult<ArcLockGuard<T>> {
        let mk_guard = |guard: MutexGuard<'static, ()>| {
            ArcLockGuard {
                inner: self.inner.clone(),
                guard: Some(guard),
            }
        };
        match self.inner.mutex.try_lock() {
            Ok(guard) => Ok(mk_guard(unsafe { transmute(guard) })),
            Err(TryLockError::Poisoned(err)) => Err(TryLockError::Poisoned(PoisonError::new(
                mk_guard(unsafe { transmute(err.into_inner()) }),
            ))),
            Err(TryLockError::WouldBlock) => Err(TryLockError::WouldBlock),
        }
    }
}

impl<T: ?Sized + Default> Default for ArcLock<T> {
    fn default() -> Self {
        ArcLock::new(T::default())
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for ArcLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_lock() {
            Ok(guard) => write!(f, "ArcLock {{ data: {:?} }}", &*guard),
            Err(TryLockError::Poisoned(err)) => {
                write!(f, "ArcLock {{ data: Poisoned({:?}) }}", &*err.into_inner())
            }
            Err(TryLockError::WouldBlock) => write!(f, "ArcLock {{ <locked> }}"),
        }
    }
}

impl<T: ?Sized> Drop for ArcLockGuard<T> {
    fn drop(&mut self) {
        // Drop `guard` before `inner` so it won't outlive `inner.mutex`
        self.guard.take();
    }
}

impl<T: ?Sized> Deref for ArcLockGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.inner.cell.get() }
    }
}

impl<T: ?Sized> DerefMut for ArcLockGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.inner.cell.get() }
    }
}

#[cfg(feature = "owning_ref")]
unsafe impl<T: ?Sized> owning_ref::StableAddress for ArcLockGuard<T> {}

#[test]
fn drop_arclock() {
    use std::mem::drop;
    let x = Arc::new(());
    assert_eq!(Arc::strong_count(&x), 1);

    let guard = {
        let t = ArcLock::new(x.clone());
        assert_eq!(Arc::strong_count(&x), 2);

        let guard = t.lock().unwrap();
        assert_eq!(Arc::strong_count(&x), 2);

        // `ArcLockGuard` can outlive `ArcLock`
        guard
    };

    assert_eq!(Arc::strong_count(&x), 2);

    drop(guard);
    assert_eq!(Arc::strong_count(&x), 1);
}

#[test]
#[should_panic]
fn lock_fail1() {
    let t = ArcLock::new(());
    let _guard = t.lock().unwrap();
    t.try_lock().unwrap();
}

#[test]
fn lock_fail2() {
    let mut t = ArcLock::new(());
    let _guard = t.lock().unwrap();
    assert!(t.try_get_mut().is_none());
}

#[test]
fn try_get_mut() {
    let mut t = ArcLock::new(());
    assert!(t.try_get_mut().is_some());
}
