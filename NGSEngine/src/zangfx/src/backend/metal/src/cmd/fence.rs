//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Fence` for Metal.
use std::sync::Arc;
use base::handles;
use common::Result;
use tokenlock::{TokenLock, TokenRef};
use metal::{MTLDevice, MTLFence};

use utils::{nil_error, OCPtr};
use cmd::queue::Item;

// TODO: recycle fences after use

/// Implementation of `Fence` for Metal.
#[derive(Debug, Clone)]
pub struct Fence {
    data: Arc<FenceData>,
}

zangfx_impl_handle! { Fence, handles::Fence }

#[derive(Debug)]
struct FenceData {
    metal_fence: OCPtr<MTLFence>,
    schedule: TokenLock<FenceScheduleData>,
}

#[derive(Debug)]
pub(super) struct FenceScheduleData {
    pub signaled: bool,
    pub waiting: Option<Box<Item>>,
}

unsafe impl Send for Fence {}
unsafe impl Sync for Fence {}

impl Fence {
    pub(crate) unsafe fn new(metal_device: MTLDevice, token_ref: TokenRef) -> Result<Self> {
        let metal_fence =
            OCPtr::new(metal_device.new_fence()).ok_or_else(|| nil_error("MTLDevice newFence"))?;
        Ok(Self {
            data: Arc::new(FenceData {
                metal_fence,
                schedule: TokenLock::new(
                    token_ref,
                    FenceScheduleData {
                        signaled: false,
                        waiting: None,
                    },
                ),
            }),
        })
    }

    pub fn metal_fence(&self) -> MTLFence {
        *self.data.metal_fence
    }

    pub(super) fn schedule_data(&self) -> &TokenLock<FenceScheduleData> {
        &self.data.schedule
    }
}
