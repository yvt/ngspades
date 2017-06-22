//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::{Condvar, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use core;

use {RefEqArc};

/// `Semaphore` implementation for Metal.
///
/// TODO: implement `Semaphore`. Note that `MTLFence` cannot be used for inter-queue synchronization,
/// and is not available on macOS 10.12.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Semaphore {
    data: RefEqArc<SemaphoreData>,
}

#[derive(Debug)]
struct SemaphoreData {
    label: Mutex<Option<String>>,
}

impl Semaphore {
    pub(crate) fn new(_: &core::SemaphoreDescription) -> Self {
        Self { data: RefEqArc::new(SemaphoreData{
            label: Mutex::new(None),
        }) }
    }
}

impl core::Marker for Semaphore {
    fn set_label(&self, label: Option<&str>) {
        *self.data.label.lock().unwrap() = label.map(String::from);
    }
}

impl core::Semaphore for Semaphore {}

/// `Fence` implementation for Metal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fence {
    data: RefEqArc<FenceData>,
}

/// Internal implementation of `Fence`.
///
/// Takes one of the following states:
///  - **Inital** - not signaled nor associated with any command submission
///     - No-op when resetted
///     - Transitions to **Associated** once passed to a submission function
///  - **Associated** - not signaled and associated with any command submission
///     - No host actions (except waiting for it being signaled) are allowed
///       during this state. Such attempts result in a panic.
///     - `num_pending_buffers > 0d`
///  - **Signaled** - signaled
///     - Transitions to **Initial** when resetted
///     - Transitions to **Associated** once passed to a submission function
///     - `num_pending_buffers == 0`
#[derive(Debug)]
struct FenceData {
    cvar: Condvar,
    state: Mutex<FenceState>,

    /// `num_pending_buffers` is managed by atomic operations in hopes of
    /// saving cpu cycles by eliding mutex ops.
    num_pending_buffers: AtomicUsize,

    label: Mutex<Option<String>>,
}

#[derive(Debug, Hash, PartialEq, Eq)]
enum FenceState {
    Initial,
    Associated,
    Signaled,
}

impl Fence {
    pub(crate) fn new(descriptor: &core::FenceDescription) -> Self {
        let signaled = descriptor.signaled;
        let initial_state = if signaled {
            FenceState::Signaled
        } else {
            FenceState::Initial
        };
        let initial_num_pending_buffers = if signaled { 0 } else { 1 };
        Self {
            data: RefEqArc::new(FenceData {
                               cvar: Condvar::new(),
                               state: Mutex::new(initial_state),
                               num_pending_buffers: AtomicUsize::new(initial_num_pending_buffers),
                               label: Mutex::new(None),
                           }),
        }
    }

    pub(crate) fn associate_pending_buffers(&self, num_buffers: usize) -> bool {
        let ref data = self.data;
        let mut state = data.state.lock().unwrap();
        match *state {
            FenceState::Initial | FenceState::Signaled => {
                data.num_pending_buffers
                    .store(num_buffers, Ordering::Relaxed);
                *state = FenceState::Associated;
                true
            }
            FenceState::Associated => false,
        }
    }

    pub(crate) fn remove_pending_buffers(&self, num_buffers: usize) {
        let ref data = self.data;
        debug_assert_eq!(*data.state.lock().unwrap(), FenceState::Associated);

        let new_num_pending_buffers = data.num_pending_buffers
            .fetch_sub(num_buffers, Ordering::Relaxed) -
                                      num_buffers;
        if new_num_pending_buffers == 0 {
            // the current batch is done!
            let mut state = data.state.lock().unwrap();
            *state = FenceState::Signaled;
            data.cvar.notify_all();
        }
    }
}

impl core::Marker for Fence {
    fn set_label(&self, label: Option<&str>) {
        *self.data.label.lock().unwrap() = label.map(String::from);
    }
}

impl core::Fence for Fence {
    fn reset(&self) -> core::Result<()> {
        let ref data = self.data;
        let mut state = data.state.lock().unwrap();
        match *state {
            FenceState::Initial => {
                debug_assert_ne!(data.num_pending_buffers.load(Ordering::Relaxed), 0);
                Ok(())
            }
            FenceState::Signaled => {
                debug_assert_eq!(data.num_pending_buffers.load(Ordering::Relaxed), 0);

                data.num_pending_buffers.store(1, Ordering::Relaxed);
                *state = FenceState::Initial;
                Ok(())
            }
            FenceState::Associated => {
                ::std::mem::drop(state);
                panic!("resetting a fence in the Associated state");
            }
        }
    }
    fn wait(&self, timeout: Duration) -> core::Result<bool> {
        let ref data = self.data;
        let mut state = data.state.lock().unwrap();

        if *state == FenceState::Signaled {
            return Ok(true);
        }

        let deadline = Instant::now() + timeout;
        while *state != FenceState::Signaled {
            let now = Instant::now();
            if now >= deadline {
                return Ok(false);
            }

            let timeout = deadline.duration_since(now);
            state = data.cvar.wait_timeout(state, timeout).unwrap().0;
        }

        Ok(true)
    }
    fn is_signaled(&self) -> core::Result<bool> {
        let ref data = self.data;
        Ok(data.num_pending_buffers.load(Ordering::Relaxed) == 0)
    }
}
