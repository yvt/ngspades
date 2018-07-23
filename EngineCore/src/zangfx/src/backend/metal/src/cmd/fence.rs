//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Fence` for Metal.
use refeq::RefEqArc;
use tokenlock::{TokenLock, TokenRef};
use zangfx_base::zangfx_impl_handle;
use zangfx_base::{self as base, Result};
use zangfx_metal_rs::{MTLDevice, MTLFence};

use crate::cmd::queue::Item;
use crate::utils::{nil_error, OCPtr};

// TODO: recycle fences after use

/// Implementation of `Fence` for Metal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fence {
    data: RefEqArc<FenceData>,
}

zangfx_impl_handle! { Fence, base::FenceRef }

#[derive(Debug)]
struct FenceData {
    metal_fence: OCPtr<MTLFence>,
    schedule: TokenLock<FenceScheduleData>,
}

#[derive(Debug)]
pub(super) struct FenceScheduleData {
    crate signaled: bool,
    crate waiting: Option<Box<Item>>,
}

unsafe impl Send for Fence {}
unsafe impl Sync for Fence {}

impl Fence {
    pub(crate) unsafe fn new(metal_device: MTLDevice, token_ref: TokenRef) -> Result<Self> {
        let metal_fence =
            OCPtr::new(metal_device.new_fence()).ok_or_else(|| nil_error("MTLDevice newFence"))?;
        Ok(Self {
            data: RefEqArc::new(FenceData {
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
