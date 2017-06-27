//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::hash::Hash;
use std::fmt::Debug;
use std::cmp::{Eq, PartialEq};
use std::any::Any;
use std::time::{Duration, Instant};
use std::marker::Sized;

use {Result, Validate, DeviceCapabilities, Marker};

/// Handle for the synchronization primitive used to synchronize between the host and device.
pub trait Event: Hash + Debug + Eq + PartialEq + Send + Sync + Any + Marker {
    fn reset(&self) -> Result<()>;
    fn wait(&self, timeout: Duration) -> Result<bool>;
    fn is_signaled(&self) -> Result<bool> {
        self.wait(Duration::new(0, 0))
    }

    /// Resets all specified events.
    ///
    /// The specified events must originate from the same device.
    fn reset_all(events: &[Self]) -> Result<()>
    where
        Self: Sized,
    {
        for event in events {
            try!(event.reset());
        }
        Ok(())
    }

    /// Waits for all specified events to be signalled.
    ///
    /// The specified events must originate from the same device.
    fn wait_all(events: &[Self], timeout: Duration) -> Result<bool>
    where
        Self: Sized,
    {
        if timeout == Duration::new(0, 0) {
            for event in events {
                if !try!(event.wait(timeout)) {
                    return Ok(false);
                }
            }
        } else {
            let deadline = Instant::now() + timeout;
            for event in events {
                let now = Instant::now();
                let sub_timeout = if now >= deadline {
                    Duration::new(0, 0)
                } else {
                    deadline.duration_since(now)
                };
                if !try!(event.wait(sub_timeout)) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EventDescription {
    pub signaled: bool,
}

/// Validation errors for [`EventDescription`](struct.EventDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum EventDescriptionValidationError {
    // None so far
}

impl Validate for EventDescription {
    type Error = EventDescriptionValidationError;

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
    where
        T: FnMut(Self::Error) -> (),
    {
        // None so far
    }
}
