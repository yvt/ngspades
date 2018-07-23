//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::Arc;
use metal;

use zangfx_base::{self as base, pipeline, shader};
use zangfx_base::{Error, ErrorKind, Result};
use zangfx_base::{zangfx_impl_object, interfaces, vtable_for, zangfx_impl_handle};
use crate::arg::rootsig::RootSig;
use crate::shader::Library;

use crate::utils::{nil_error, OCPtr};

/// Implementation of `ComputePipelineBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct ComputePipelineBuilder {
    /// A reference to a `MTLDevice`. We are not required to maintain a strong
    /// reference. (See the base interface's documentation)
    metal_device: metal::MTLDevice,

    compute_shader: Option<(Library, String)>,
    root_sig: Option<RootSig>,

    label: Option<String>,
}

zangfx_impl_object! { ComputePipelineBuilder:
pipeline::ComputePipelineBuilder, crate::Debug, base::SetLabel }

unsafe impl Send for ComputePipelineBuilder {}
unsafe impl Sync for ComputePipelineBuilder {}

impl ComputePipelineBuilder {
    /// Construct a `ComputePipelineBuilder`.
    ///
    /// Ir's up to the caller to maintain the lifetime of `metal_device`.
    pub unsafe fn new(metal_device: metal::MTLDevice) -> Self {
        Self {
            metal_device,
            compute_shader: None,
            root_sig: None,
            label: None,
        }
    }
}

impl base::SetLabel for ComputePipelineBuilder {
    fn set_label(&mut self, label: &str) {
        self.label = Some(label.to_owned());
    }
}

impl pipeline::ComputePipelineBuilder for ComputePipelineBuilder {
    fn compute_shader(
        &mut self,
        library: &base::LibraryRef,
        entry_point: &str,
    ) -> &mut pipeline::ComputePipelineBuilder {
        let my_library: &Library = library.downcast_ref().expect("bad library type");
        self.compute_shader = Some((my_library.clone(), entry_point.to_owned()));
        self
    }

    fn root_sig(&mut self, v: &base::RootSigRef) -> &mut pipeline::ComputePipelineBuilder {
        let my_root_sig: &RootSig = v.downcast_ref().expect("bad root signature type");
        self.root_sig = Some(my_root_sig.clone());
        self
    }

    fn build(&mut self) -> Result<base::ComputePipelineRef> {
        let compute_shader = self.compute_shader
            .as_ref()
            .expect("compute_shader");
        let root_sig = self.root_sig
            .as_ref()
            .expect("root_sig");

        let metal_desc = unsafe {
            OCPtr::from_raw(metal::MTLComputePipelineDescriptor::alloc().init())
                .ok_or_else(|| nil_error("MTLComputePipelineDescriptor alloc"))?
        };

        let compute_fn = compute_shader.0.new_metal_function(
            &compute_shader.1,
            shader::ShaderStage::Compute,
            root_sig,
            ::std::iter::empty(),
            self.metal_device,
            &self.label,
        )?;
        metal_desc.set_compute_function(*compute_fn);

        let local_size = compute_shader.0.workgroup_size();
        let threads_per_threadgroup = metal::MTLSize {
            width: local_size[0] as u64,
            height: local_size[1] as u64,
            depth: local_size[2] as u64,
        };

        if let Some(ref label) = self.label {
            metal_desc.set_label(label);
        }

        let metal_pipeline = self.metal_device
            .new_compute_pipeline_state(*metal_desc)
            .map_err(|e| Error::with_detail(ErrorKind::Other, e))
            .and_then(|p| {
                OCPtr::new(p).ok_or_else(|| {
                    nil_error(
                        "MTLDevice newComputePipelineStateWithDescriptor:options:reflection:error:",
                    )
                })
            })?;

        // we cannot know this beforehand without actually creating a compute pipeline state
        // but at least it seems to be around 256 (tested on Iris Graphics 550).
        //
        // If the number of invocations specified by the shader exceeds the limitation
        // reported by the pipeline state, there is no way other than panicking to report
        // this state. I expect this will not happen in practice.
        let supported_max_total_invocations = metal_pipeline.max_total_threads_per_threadgroup();
        let total_invocations = threads_per_threadgroup
            .width
            .checked_mul(threads_per_threadgroup.height)
            .and_then(|x| x.checked_mul(threads_per_threadgroup.depth));
        if let Some(total_invocations) = total_invocations {
            if total_invocations > supported_max_total_invocations {
                panic!(
                    "too many compute shader invocations per work group ({} > {})",
                    total_invocations, supported_max_total_invocations
                );
            }
        } else {
            panic!(
                "too many compute shader invocations per work group ((overflow) > {})",
                supported_max_total_invocations
            );
        }

        let data = ComputePipelineData {
            metal_pipeline,
            threads_per_threadgroup,
        };

        Ok(ComputePipeline {
            data: Arc::new(data),
        }.into())
    }
}

/// Implementation of `ComputePipeline` for Metal.
#[derive(Debug, Clone)]
pub struct ComputePipeline {
    data: Arc<ComputePipelineData>,
}

zangfx_impl_handle! { ComputePipeline, base::ComputePipelineRef }

#[derive(Debug)]
struct ComputePipelineData {
    metal_pipeline: OCPtr<metal::MTLComputePipelineState>,
    threads_per_threadgroup: metal::MTLSize,
}

unsafe impl Send for ComputePipelineData {}
unsafe impl Sync for ComputePipelineData {}

impl ComputePipeline {
    pub fn metal_pipeline(&self) -> metal::MTLComputePipelineState {
        *self.data.metal_pipeline
    }

    pub fn threads_per_threadgroup(&self) -> metal::MTLSize {
        self.data.threads_per_threadgroup
    }
}
