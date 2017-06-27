//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::{Condvar, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use core;

use RefEqArc;

/// `Event` implementation for Metal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Event {
    data: RefEqArc<EventData>,
}

/// Internal implementation of `Event`.
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
struct EventData {
    cvar: Condvar,
    state: Mutex<EventState>,

    /// `num_pending_buffers` is managed by atomic operations in hopes of
    /// saving cpu cycles by eliding mutex ops.
    num_pending_buffers: AtomicUsize,

    label: Mutex<Option<String>>,
}

#[derive(Debug, Hash, PartialEq, Eq)]
enum EventState {
    Initial,
    Associated,
    Signaled,
}

impl Event {
    pub(crate) fn new(descriptor: &core::EventDescription) -> Self {
        let signaled = descriptor.signaled;
        let initial_state = if signaled {
            EventState::Signaled
        } else {
            EventState::Initial
        };
        let initial_num_pending_buffers = if signaled { 0 } else { 1 };
        Self {
            data: RefEqArc::new(EventData {
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
            EventState::Initial | EventState::Signaled => {
                data.num_pending_buffers.store(
                    num_buffers,
                    Ordering::Relaxed,
                );
                *state = EventState::Associated;
                true
            }
            EventState::Associated => false,
        }
    }

    pub(crate) fn remove_pending_buffers(&self, num_buffers: usize) {
        let ref data = self.data;
        debug_assert_eq!(*data.state.lock().unwrap(), EventState::Associated);

        let new_num_pending_buffers = data.num_pending_buffers.fetch_sub(
            num_buffers,
            Ordering::Relaxed,
        ) - num_buffers;
        if new_num_pending_buffers == 0 {
            // the current batch is done!
            let mut state = data.state.lock().unwrap();
            *state = EventState::Signaled;
            data.cvar.notify_all();
        }
    }
}

impl core::Marker for Event {
    fn set_label(&self, label: Option<&str>) {
        *self.data.label.lock().unwrap() = label.map(String::from);
    }
}

impl core::Event for Event {
    fn reset(&self) -> core::Result<()> {
        let ref data = self.data;
        let mut state = data.state.lock().unwrap();
        match *state {
            EventState::Initial => {
                debug_assert_ne!(data.num_pending_buffers.load(Ordering::Relaxed), 0);
                Ok(())
            }
            EventState::Signaled => {
                debug_assert_eq!(data.num_pending_buffers.load(Ordering::Relaxed), 0);

                data.num_pending_buffers.store(1, Ordering::Relaxed);
                *state = EventState::Initial;
                Ok(())
            }
            EventState::Associated => {
                ::std::mem::drop(state);
                panic!("resetting an event in the Associated state");
            }
        }
    }
    fn wait(&self, timeout: Duration) -> core::Result<bool> {
        let ref data = self.data;
        let mut state = data.state.lock().unwrap();

        if *state == EventState::Signaled {
            return Ok(true);
        }

        let deadline = Instant::now() + timeout;
        while *state != EventState::Signaled {
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
