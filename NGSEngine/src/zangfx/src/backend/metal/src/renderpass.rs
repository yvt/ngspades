//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use metal;
use base::handles;

/// Implementation of `RenderTargetTable` for Metal.
#[derive(Debug, Clone)]
pub struct RenderTargetTable {}

zangfx_impl_handle! { RenderTargetTable, handles::RenderTargetTable }

impl RenderTargetTable {
    pub fn metal_render_pass(&self) -> metal::MTLRenderPassDescriptor {
        unimplemented!()
    }
}
