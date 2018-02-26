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

/// Implementation of `Fence` for Metal.
#[derive(Debug, Clone)]
pub struct Fence {
    data: Arc<OCPtr<MTLFence>>,
}

zangfx_impl_handle! { Fence, handles::Fence }

unsafe impl Send for Fence {}
unsafe impl Sync for Fence {}

impl Fence {
    pub(crate) unsafe fn new(metal_device: MTLDevice) -> Result<Self> {
        let metal_fence = metal_device.new_fence();
        if metal_fence.is_null() {
            return Err(nil_error("MTLDevice newFence"));
        }
        Ok(Self::from_raw(metal_fence).into())
    }

    pub unsafe fn from_raw(metal_fence: MTLFence) -> Self {
        Self {
            data: Arc::new(OCPtr::from_raw(metal_fence).unwrap()),
        }
    }
}
