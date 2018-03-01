//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Builder for render/compute pipeline objects.
use Object;

use common::Result;
use handles::{ComputePipeline, Library, RootSig};

/// Trait for building compute pipelines.
///
/// # Valid Usage
///
///  - No instance of `ComputePipelineBuilder` may outlive the originating `Device`.
///
/// # Examples
///
///     # use zangfx_base::device::Device;
///     # use zangfx_base::handles::Library;
///     # fn test(device: &Device, library: &Library) {
///     let pipeline = device.build_compute_pipeline()
///         .compute_shader(library, "main")
///         .build()
///         .expect("Failed to create a pipeline.");
///     # }
///
pub trait ComputePipelineBuilder: Object {
    /// Set the compute shader.
    ///
    /// Mandatory.
    fn compute_shader(
        &mut self,
        library: &Library,
        entry_point: &str,
    ) -> &mut ComputePipelineBuilder;

    /// Set the root signature.
    ///
    /// Mandatory.
    fn root_sig(&mut self, v: &RootSig) -> &mut ComputePipelineBuilder;

    /// Build an `ComputePipeline`.
    ///
    /// # Valid Usage
    ///
    /// All mandatory properties must have their values set before this method
    /// is called.
    fn build(&mut self) -> Result<ComputePipeline>;
}

// TODO: render pipeline
