//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! A mutex that adds another mode of explicit locking to satisfy a specific
//! performance and security need.
//!
//! This crate provides a mutex type [`StickyMutex`].
//! In addition to the normal operation of a reentrant mutex, it supports
//! transitioning into an intermediate state, which we call the *sticky* state.
//! In this state, a thread has a lock acquired on it, but doesn't hold a
//! reference to it, making it possible to sustain this state longer than
//! it's usually possible (so it *sticks* to a thread, hence the name).
//! This state is entered and left by calling the respective `stick` and
//! `unstick` method explicitly.
//!
//! To access its contained data, the program has to acquire a normal lock just
//! like a normal mutex. However, *if a thread already owns a sticky lock, the
//! lock operation can be performed extremely fast* because no actual locking has
//! to take place.
//!
//!     use stickylock::StickyMutex;
//!     struct BankAccounts {
//!         alice_balance: u64,
//!         bob_balance: u64,
//!     }
//!
//!     fn transfer(accounts: &StickyMutex<BankAccounts>) {
//!         let mut guard = accounts.lock();
//!         guard.alice_balance += 1;
//!         guard.bob_balance -= 1;
//!     }
//!
//!     let accounts = StickyMutex::new(BankAccounts{
//!         alice_balance: 10000,
//!         bob_balance: 10000,
//!     });
//!
//!     // The following loop runs slowly because a lock
//!     // is re-acquired on each iteration:
//!     for _ in 0..1000 {
//!         transfer(&accounts);
//!     }
//!
//!     // But not this one:
//!     accounts.stick();
//!     for _ in 0..1000 {
//!         transfer(&accounts);
//!     }
//!     accounts.unstick();
//!
//! Let us summarize each state a mutex can be in:
//!
//! - **Unlocked**. The mutex is not locked by any thread.
//! - **Sticky**. A thread owns a sticky lock because it called the mutex's
//!   `stick` method. This state lasts until `unstick` is called. At any point,
//!   only up to one thread can maintain a lock (normal/sticky) on a mutex.
//! - **Locked**. A thread owns a lock and holds a lock guard, and can access
//!   the contained value using the lock guard.
//!
//! Sticky lock is *recursive* - you can call `stick` as many times as
//! you want, as long as it's matched by the same number of calls to `unstick`.
//! Furthermore, you can call `stick`/`unstick` while at the same time holding a
//! normal lock.
//!
//! # Comparison with `ReentrantMutex`
//!
//! In some ways, the mutex provided by this crate bears some resemblance to
//! `ReentrantMutex` provided by `parking_lot`. In fact, I even considered
//! implementing this crate based on `ReentrantMutex`! However, there was a
//! fundamental difference, which was especially important for a certain use
//! case (which is discussed in the next section).
//!
//! You can acquire a lock from `ReentrantMutex` and keep it by `forget`ting
//! the returned lock guard. After that, you can release the lock by calling
//! its `raw_unlock` method. Acquiring another lock while holding one is also
//! fast, because no actual locking operation takes place. However, a problem
//! arises when you want to expose its locking/unlocking interface to an
//! untrusted code (or in other words, a code whose safety cannot be verified
//! statically) because `raw_unlock` is `unsafe` and exposing it would
//! jeopardize the memory safety.
//!
//! This crate deals with this problem by the addition of a separate mode of
//! locking which we call *sticky lock*. An untrusted code would acquire a sticky
//! lock to mark a code section within which it makes a massive number of calls
//! to a trusted code. The trusted code would have to acquire a normal lock
//! every time it is called, but it's very fast because the mutex is already
//! locked by a sticky lock. It's impossible for untrusted code to compromise
//! the memory safety because the trusted code uses a normal lock and
//! the untrusted code cannot interfere with it.
//!
//! (Although an untrusted code can cause a dead-lock by an incorrect use of
//! a sticky lock, it's beyond our scope as far as the memory-safety is
//! concerned.)
//!
//! # Motivation / use case
//!
//! The crate was created because of a necessity for an external interface that
//! fulfilles the following requirements:
//!
//!  - Performant enough that a massive number of operations can be performed
//!    even in real-time and/or interactive applications.
//!
//!  - Thread-safe. Since this interface would be called by a code loaded from
//!    an untrusted source, it was impossible to guarantee the mutual exclusion
//!    using any kind of static analysis. Therefore a runtime check was
//!    inevitable.
//!
//!  - No token passing. The aforementioned untrusted-code requirement also
//!    precludes [the use of a token-based access control], which in the end
//!    relies on Rust's unique mutable reference rule.
//!
//! [the use of a token-based access control]: https://crates.io/crates/tokenlock
//!
//! # Quirks
//!
//!  - This crate depends on `parking_lot` because the standard `std::sync::Mutex`
//!    lacks a raw locking interface.
//!  - If a thread exits without releasing a sticky lock, the lock ownership is
//!    not relinquished, but instead it's "lost" - it might be never recovered,
//!    or might be transfered to another thread that happens to have the same
//!    identifier (which in practice only happens on a 32-bit architecture).
//!  - Poisoning is not implemented.
//!
//! # Implementation notes
//!
//! `StickyMutex` can be alternatively implemented using `ReentrantMutex`.
//! However, doing so would require us to access the internal fields of
//! `ReentrantMutex` or maintain a shadow copy of them to ensure the safety of
//! `unstick` (note that `ReentrantMutex::raw_unlock` is `unsafe` while our
//! `unstick` is not).
extern crate parking_lot;

mod mutex_core;
use mutex_core::StickyMutexCore;
pub use mutex_core::UnstickError;

use std::cell::UnsafeCell;
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};

/// A mutex type that supports holding a lock without holding a lock guard.
///
/// See [the crate-level documentation] for details.
///
/// [the crate-level documentation]: index.html
pub struct StickyMutex<T: ?Sized> {
    core: StickyMutexCore,
    borrowed: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for StickyMutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for StickyMutex<T> {}

impl<T> StickyMutex<T> {
    /// Construct a `StickyMutex` containing the supplied value.
    pub fn new(x: T) -> Self {
        Self {
            core: StickyMutexCore::new(),
            borrowed: AtomicBool::new(false),
            data: UnsafeCell::new(x),
        }
    }

    /// Consume this mutex, returning the contained value.
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for StickyMutex<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        if let Some(guard) = self.try_lock() {
            fmt.debug_struct("StickyMutex")
                .field("data", &&*guard)
                .finish()
        } else {
            struct LockedPlaceholder;
            impl fmt::Debug for LockedPlaceholder {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    f.write_str("<locked>")
                }
            }
            fmt.debug_struct("StickyMutex")
                .field("data", &LockedPlaceholder)
                .finish()
        }
    }
}

impl<T: ?Sized> StickyMutex<T> {
    /// Acquire a sticky lock for the current thread. Increase the sticky
    /// lock count.
    ///
    ///  - If the calling thread does not hold a normal nor sticky lock, a real
    ///    lock operation takes place. It will block if another thread already
    ///    holds a lock on it.
    ///  - If the calling thread already holds a normal or sticky lock, the
    ///    sticky lock count is increased.
    ///
    /// # Panics
    ///
    /// Panics if the lock count overflows.
    pub fn stick(&self) {
        self.core.stick();
    }

    /// Decrease the sticky lock count. Release a sticky lock if the count
    /// reaches zero.
    pub fn unstick(&self) -> Result<(), UnstickError> {
        unsafe { self.core.unstick(|| self.borrowed.load(Ordering::Relaxed)) }
    }

    /// Acquire a lock, blocking the current thread until it is able to do so.
    ///
    ///  - If the calling thread does not hold a normal nor sticky lock, a real
    ///    lock operation takes place. It will block if another thread already
    ///    holds a lock on it.
    ///  - If the calling thread already holds a sticky lock but not a normal
    ///    lock, then the lock succeeds immediately (and fast).
    ///  - It will panic if the current thread already holds a normal lock.
    ///
    /// # Panics
    ///
    /// Panics if it is already locked by the current thread.
    pub fn lock(&self) -> StickyMutexGuard<T> {
        self.core.lock();

        // Check the uniqueness of mutable reference
        if self.borrowed.load(Ordering::Relaxed) {
            panic!("already locked by the current thread");
        }
        self.borrowed.store(true, Ordering::Relaxed);

        StickyMutexGuard(self, PhantomData)
    }

    /// Attempt to acquire a lock.
    ///
    /// Works similarly to `lock`, but returns `None` if the lock could not
    /// be acquired at this time.
    pub fn try_lock(&self) -> Option<StickyMutexGuard<T>> {
        if !self.core.try_lock() {
            return None;
        }

        // Check the uniqueness of mutable reference
        if self.borrowed.load(Ordering::Relaxed) {
            return None;
        }

        Some(StickyMutexGuard(self, PhantomData))
    }

    /// Get a mutable reference to the contained data.
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }
}

/// An RAII lock guard of `StickyMutex`. The mutex is unlocked when this
/// structure is dropped.
#[derive(Debug)]
pub struct StickyMutexGuard<'a, T: ?Sized + 'a>(&'a StickyMutex<T>, PhantomData<*mut T>);

impl<'a, T: ?Sized + 'a> Deref for StickyMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.0.data.get() }
    }
}

impl<'a, T: ?Sized + 'a> DerefMut for StickyMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0.data.get() }
    }
}

impl<'a, T: ?Sized + 'a> Drop for StickyMutexGuard<'a, T> {
    fn drop(&mut self) {
        self.0.borrowed.store(false, Ordering::Relaxed);
        unsafe {
            self.0.core.unlock();
        }
    }
}
