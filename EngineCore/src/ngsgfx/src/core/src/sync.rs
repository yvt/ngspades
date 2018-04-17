//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Defines synchronization primitives.
use std::hash::Hash;
use std::fmt::Debug;
use std::cmp::{Eq, PartialEq};
use std::any::Any;
use std::time::{Duration, Instant};
use std::marker::Sized;

use {Result, Validate, DeviceCapabilities, Marker, DeviceEngineFlags, DeviceEngine};

/// Inter-pass synchronization primitive.
pub trait Fence: Hash + Debug + Eq + PartialEq + Send + Sync + Any + Marker {}

#[derive(Debug, Clone, Copy)]
pub struct FenceDescription {
    /// Specifies the set of `DeviceEngine`s that are allowed to update the fence.
    ///
    /// The specified value must not include `Host` and must include at least one engine.
    pub update_engines: DeviceEngineFlags,

    /// Specifies the set of `DeviceEngine`s that are allowed to wait on the fence.
    ///
    /// The specified value must not include `Host` and must include at least one engine.
    pub wait_engines: DeviceEngineFlags,
}

/// Host-device synchronization primitive.
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

/// Validation errors for [`FenceDescription`](struct.FenceDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum FenceDescriptionValidationError {
    /// `DeviceEngine::Host` is specified in one of `update_engines` and `wait_engines`
    HostEngine,
    /// `update_engines` is empty.
    NoUpdateEngines,
    /// `wait_engines` is empty.
    NoWaitEngines,
}

impl Validate for FenceDescription {
    type Error = FenceDescriptionValidationError;

    fn validate<T>(&self, _: Option<&DeviceCapabilities>, mut callback: T)
    where
        T: FnMut(Self::Error) -> (),
    {
        if !((self.update_engines | self.update_engines) & DeviceEngine::Host).is_empty() {
            callback(FenceDescriptionValidationError::HostEngine);
        }
        if self.update_engines.is_empty() {
            callback(FenceDescriptionValidationError::NoUpdateEngines);
        }
        if self.wait_engines.is_empty() {
            callback(FenceDescriptionValidationError::NoWaitEngines);
        }
    }
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