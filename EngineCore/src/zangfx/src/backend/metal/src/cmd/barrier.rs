//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Barrier` for Metal.
//!
//! On Metal, `Barrier` is implemented by `[MTLRenderCommandEncoder textureBarrier]`.
//! Since the granularity of `textureBarrier` is limited to entire the system,
//! we simply ignore all parameters specified via `BarrierBuilder`.
use std::ops::Range;
use metal;
use base;
use base::{handles, resources, sync};
use common::Result;

use utils::translate_render_stage;

/// Implementation of `BarrierBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct BarrierBuilder {
    dst_access: base::AccessTypeFlags,
}

zangfx_impl_object! { BarrierBuilder: sync::BarrierBuilder, ::Debug }

impl BarrierBuilder {
    pub fn new() -> Self {
        BarrierBuilder {
            dst_access: flags![base::AccessType::{}],
        }
    }
}

impl sync::BarrierBuilder for BarrierBuilder {
    fn global(
        &mut self,
        _src_access: base::AccessTypeFlags,
        dst_access: base::AccessTypeFlags,
    ) -> &mut sync::BarrierBuilder {
        self.dst_access |= dst_access;
        self
    }

    fn buffer(
        &mut self,
        _src_access: base::AccessTypeFlags,
        dst_access: base::AccessTypeFlags,
        _buffer: &handles::Buffer,
        _range: Option<Range<base::DeviceSize>>,
    ) -> &mut sync::BarrierBuilder {
        self.dst_access |= dst_access;
        self
    }

    fn image(
        &mut self,
        _src_access: base::AccessTypeFlags,
        dst_access: base::AccessTypeFlags,
        _image: &handles::Image,
        _src_layout: resources::ImageLayout,
        _dst_layout: resources::ImageLayout,
        _range: &resources::ImageSubRange,
    ) -> &mut sync::BarrierBuilder {
        self.dst_access |= dst_access;
        self
    }

    fn build(&mut self) -> Result<handles::Barrier> {
        Ok(Barrier {
            dst_stage: translate_render_stage(base::AccessType::union_supported_stages(
                self.dst_access,
            )),
        }.into())
    }
}

/// Implementation of `Barrier` for Metal.
#[derive(Debug, Clone)]
pub struct Barrier {
    dst_stage: metal::MTLRenderStages,
}

zangfx_impl_handle! { Barrier, handles::Barrier }

impl Barrier {
    pub(crate) fn metal_dst_stage(&self) -> metal::MTLRenderStages {
        self.dst_stage
    }
}
