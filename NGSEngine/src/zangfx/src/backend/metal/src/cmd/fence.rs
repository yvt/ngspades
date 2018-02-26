//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Fence` for Metal.
use std::sync::Arc;
use base::{handles, sync};
use common::Result;
use metal::{MTLDevice, MTLFence};
use utils::{nil_error, OCPtr};

// TODO: recycle fences after use

/// Implementation of `FenceBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct FenceBuilder {
    metal_device: MTLDevice,
}

zangfx_impl_object! { FenceBuilder }

unsafe impl Send for FenceBuilder {}
unsafe impl Sync for FenceBuilder {}

impl FenceBuilder {
    pub(crate) unsafe fn new(metal_device: MTLDevice) -> Self {
        Self { metal_device }
    }
}

impl sync::FenceBuilder for FenceBuilder {
    fn build(&mut self) -> Result<handles::Fence> {
        let metal_fence = self.metal_device.new_fence();
        if metal_fence.is_null() {
            return Err(nil_error("MTLDevice newFence"));
        }
        Ok(unsafe { Fence::from_raw(metal_fence) }.into())
    }
}

/// Implementation of `Fence` for Metal.
#[derive(Debug, Clone)]
pub struct Fence {
    data: Arc<OCPtr<MTLFence>>,
}

zangfx_impl_handle! { Fence, handles::Fence }

unsafe impl Send for Fence {}
unsafe impl Sync for Fence {}

impl Fence {
    pub unsafe fn from_raw(metal_fence: MTLFence) -> Self {
        Self {
            data: Arc::new(OCPtr::from_raw(metal_fence).unwrap()),
        }
    }
}
