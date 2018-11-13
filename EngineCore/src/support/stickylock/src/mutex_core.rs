//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

use parking_lot::Mutex;
use std::mem::forget;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
pub struct StickyMutexCore {
    mutex: Mutex<()>,
    owner: AtomicUsize, // Atomic<ThreadId>
    stick_count: AtomicUsize,
}

/// An error value returned by the `unstick` method.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum UnstickError {
    /// The sticky lock count is already zero.
    NotLocked,
}

impl StickyMutexCore {
    pub fn new() -> Self {
        Self {
            mutex: Mutex::new(()),
            owner: AtomicUsize::new(NOBODY),
            stick_count: AtomicUsize::new(0),
        }
    }

    /// Acquire a "hard" lock. No-op if it already has a hard lock.
    pub fn lock(&self) {
        let current_thread_id = current_thread_id();
        if self.owner.load(Ordering::Relaxed) != current_thread_id {
            forget(self.mutex.lock());

            debug_assert_eq!(self.stick_count.load(Ordering::Relaxed), 0);
            self.owner.store(current_thread_id, Ordering::Relaxed);
        }
    }

    /// Try to acquire a "hard" lock. No-op if it already has a hard lock.
    pub fn try_lock(&self) -> bool {
        let current_thread_id = current_thread_id();
        if self.owner.load(Ordering::Relaxed) != current_thread_id {
            let lock = self.mutex.try_lock();
            if lock.is_none() {
                return false;
            }
            forget(lock);

            debug_assert_eq!(self.stick_count.load(Ordering::Relaxed), 0);
            self.owner.store(current_thread_id, Ordering::Relaxed);
        }
        true
    }

    /// Release a "hard" lock. The caller must ensure that it already has a hard
    /// lock. Note that the hard lock modeled by this type is not recursive -
    /// you must call `unlock` exactly once no matter how many times you called
    /// `lock` before.
    pub unsafe fn unlock(&self) {
        debug_assert_eq!(self.owner.load(Ordering::Relaxed), current_thread_id());

        let stick_count = self.stick_count.load(Ordering::Relaxed);

        if stick_count == 0 {
            self.owner.store(NOBODY, Ordering::Relaxed);
            self.mutex.force_unlock();
        }
    }

    /// Increase the sticky lock count.
    pub fn stick(&self) {
        let current_thread_id = current_thread_id();
        if self.owner.load(Ordering::Relaxed) == current_thread_id {
            let new_stick_count = self.stick_count
                .load(Ordering::Relaxed)
                .checked_add(1)
                .expect("sticky lock count overflow");

            self.stick_count.store(new_stick_count, Ordering::Relaxed);
        } else {
            forget(self.mutex.lock());

            debug_assert_eq!(self.stick_count.load(Ordering::Relaxed), 0);
            self.stick_count.store(1, Ordering::Relaxed);
            self.owner.store(current_thread_id, Ordering::Relaxed);
        }
    }

    /// Decrease the sticky lock count. `has_normal_lock`, which is called only
    /// if the current thread owns the mutex, must return whether the mutex
    /// is currently locked using a "hard" lock (i.e. there have been calls to
    /// `lock` without a matching call to `unlock`).
    pub unsafe fn unstick<F>(&self, has_normal_lock: F) -> Result<(), UnstickError>
    where
        F: FnOnce() -> bool,
    {
        let current_thread_id = current_thread_id();
        if self.owner.load(Ordering::Relaxed) == current_thread_id {
            let new_stick_count = self.stick_count
                .load(Ordering::Relaxed)
                .checked_sub(1)
                .ok_or(UnstickError::NotLocked)?;

            self.stick_count.store(new_stick_count, Ordering::Relaxed);

            if new_stick_count == 0 && !has_normal_lock() {
                self.owner.store(NOBODY, Ordering::Relaxed);
                self.mutex.force_unlock();
            }
            Ok(())
        } else {
            Err(UnstickError::NotLocked)
        }
    }
}

/// An identifier to indicate which thread owns the mutex. The zero value is
/// reserved for `NOBODY`.
type ThreadId = usize;

const NOBODY: ThreadId = 0;

fn current_thread_id() -> ThreadId {
    use std::mem::size_of;
    if size_of::<usize>() < 8 {
        thread_local! {
            static KEY: u8 = 0;
        }
        KEY.with(|x| x as *const _ as usize)
    } else {
        static NEXT_THREAD_ID: AtomicUsize = AtomicUsize::new(1);
        thread_local! {
            static THREAD_ID: usize = {
                let ret = NEXT_THREAD_ID.fetch_add(1, Ordering::Relaxed);
                if ret == 0xffff_0000_0000_0000usize {
                    ::std::process::abort();
                }
                ret
            };
        }
        THREAD_ID.with(|x| *x)
    }
}
