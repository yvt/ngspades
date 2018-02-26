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
use base;
use base::{handles, resources, sync};
use common::Result;

/// Implementation of `BarrierBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct BarrierBuilder;

zangfx_impl_object! { BarrierBuilder }
interfaces! { BarrierBuilder: sync::BarrierBuilder, ::Debug, ::Any }

impl sync::BarrierBuilder for BarrierBuilder {
    fn global(
        &mut self,
        _src_access: base::AccessTypeFlags,
        _dst_access: base::AccessTypeFlags,
    ) -> &mut sync::BarrierBuilder {
        self
    }

    fn buffer(
        &mut self,
        _src_access: base::AccessTypeFlags,
        _dst_access: base::AccessTypeFlags,
        _buffer: &handles::Buffer,
        _range: Option<Range<base::DeviceSize>>,
    ) -> &mut sync::BarrierBuilder {
        self
    }

    fn image(
        &mut self,
        _src_access: base::AccessTypeFlags,
        _dst_access: base::AccessTypeFlags,
        _image: &handles::Image,
        _src_layout: resources::ImageLayout,
        _dst_layout: resources::ImageLayout,
        _range: &resources::ImageSubRange,
    ) -> &mut sync::BarrierBuilder {
        self
    }

    fn build(&mut self) -> Result<handles::Barrier> {
        Ok(Barrier.into())
    }
}

/// Implementation of `Barrier` for Metal.
#[derive(Debug, Clone)]
pub struct Barrier;

zangfx_impl_handle! { Barrier, handles::Barrier }
