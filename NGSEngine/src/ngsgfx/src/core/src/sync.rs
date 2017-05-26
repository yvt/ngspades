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

use {Result, Validate, DeviceCapabilities};

/// The handle to a synchronization primitive used to synchronize between the host and device.
pub trait Fence: Hash + Debug + Eq + PartialEq + Send + Sync + Any {
    fn reset(&self) -> Result<()>;
    fn wait(&self, timeout: Duration) -> Result<bool>;
    fn is_signaled(&self) -> Result<bool> {
        self.wait(Duration::new(0, 0))
    }

    /// Resets all specified fences.
    ///
    /// The specified fences must originate from the same device.
    fn reset_all(fences: &[Self]) -> Result<()>
        where Self: Sized
    {
        for fence in fences {
            try!(fence.reset());
        }
        Ok(())
    }

    /// Waits for all specified fences to be signalled.
    ///
    /// The specified fences must originate from the same device.
    fn wait_all(fences: &[Self], timeout: Duration) -> Result<bool>
        where Self: Sized
    {
        if timeout == Duration::new(0, 0) {
            for fence in fences {
                if !try!(fence.wait(timeout)) {
                    return Ok(false);
                }
            }
        } else {
            let deadline = Instant::now() + timeout;
            for fence in fences {
                let now = Instant::now();
                let sub_timeout = if now >= deadline {
                    Duration::new(0, 0)
                } else {
                    deadline.duration_since(now)
                };
                if !try!(fence.wait(sub_timeout)) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}

/// The handle to an inter-queue synchronization primitive.
pub trait Semaphore: Hash + Debug + Eq + PartialEq + Send + Sync + Any {}

#[derive(Debug, Clone, Copy)]
pub struct FenceDescription {
    pub signaled: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct SemaphoreDescription {}

/// Validation errors for [`FenceDescription`](struct.FenceDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum FenceDescriptionValidationError {
    // None so far
}

impl Validate for FenceDescription {
    type Error = FenceDescriptionValidationError;

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
        where T: FnMut(Self::Error) -> ()
    {
        // None so far
    }
}

/// Validation errors for [`SemaphoreDescription`](struct.SemaphoreDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum SemaphoreDescriptionValidationError {
    // None so far
}

impl Validate for SemaphoreDescription {
    type Error = SemaphoreDescriptionValidationError;

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
        where T: FnMut(Self::Error) -> ()
    {
        // None so far
    }
}



