//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of argument table layout for Vulkan.
use std::sync::Arc;
use ash::vk;
use ash::version::*;

use base;
use common::Result;
use device::DeviceRef;

use utils::{translate_generic_error_unwrap, translate_shader_stage_flags};
use super::{translate_descriptor_type, DescriptorCount};

/// Implementation of `ArgTableSigBuilder` for Vulkan.
#[derive(Debug)]
pub struct ArgTableSigBuilder {
    device: DeviceRef,
    args: Vec<Option<ArgSig>>,
}

zangfx_impl_object! { ArgTableSigBuilder: base::ArgTableSigBuilder, ::Debug }

impl ArgTableSigBuilder {
    pub(crate) unsafe fn new(device: DeviceRef) -> Self {
        Self {
            device,
            args: Vec::new(),
        }
    }
}

impl base::ArgTableSigBuilder for ArgTableSigBuilder {
    fn arg(&mut self, index: base::ArgIndex, ty: base::ArgType) -> &mut base::ArgSig {
        if index >= self.args.len() {
            self.args.resize(index + 1, None);
        }
        let ref mut e = self.args[index];
        if e.is_none() {
            *e = Some(ArgSig {
                vk_binding: vk::DescriptorSetLayoutBinding {
                    binding: index as u32,
                    descriptor_type: translate_descriptor_type(ty),
                    descriptor_count: 1,
                    stage_flags: vk::SHADER_STAGE_ALL,
                    p_immutable_samplers: ::null(),
                },
            });
        }
        e.as_mut().unwrap()
    }

    fn build(&mut self) -> Result<base::ArgTableSig> {
        let bindings: Vec<_> = self.args
            .iter()
            .filter_map(|x| x.as_ref())
            .map(|arg| arg.vk_binding.clone())
            .collect();

        let info = vk::DescriptorSetLayoutCreateInfo {
            s_type: vk::StructureType::DescriptorSetLayoutCreateInfo,
            p_next: ::null(),
            flags: vk::DescriptorSetLayoutCreateFlags::empty(),
            binding_count: bindings.len() as u32,
            p_bindings: bindings.as_ptr(),
        };

        // Count the number of descriptors for descriptor pool allocation
        let desc_count = DescriptorCount::from_bindings(&bindings);

        let vk_device = self.device.vk_device();
        let vk_ds_layout = unsafe { vk_device.create_descriptor_set_layout(&info, None) }
            .map_err(translate_generic_error_unwrap)?;
        Ok(ArgTableSig::new(self.device, vk_ds_layout, desc_count).into())
    }
}

/// Implementation of `ArgSig` for Vulkan.
#[derive(Debug, Clone)]
pub struct ArgSig {
    vk_binding: vk::DescriptorSetLayoutBinding,
}

zangfx_impl_object! { ArgSig: base::ArgSig, ::Debug }

impl base::ArgSig for ArgSig {
    fn set_len(&mut self, x: base::ArgArrayIndex) -> &mut base::ArgSig {
        self.vk_binding.descriptor_count = x as u32;
        self
    }

    fn set_stages(&mut self, x: base::ShaderStageFlags) -> &mut base::ArgSig {
        self.vk_binding.stage_flags = translate_shader_stage_flags(x);
        self
    }
}

/// Implementation of `ArgTableSig` for Vulkan.
#[derive(Debug, Clone)]
pub struct ArgTableSig {
    data: Arc<ArgTableSigData>,
}

zangfx_impl_handle! { ArgTableSig, base::ArgTableSig }

unsafe impl Sync for ArgTableSigData {}
unsafe impl Send for ArgTableSigData {}

#[derive(Debug, Clone)]
struct ArgTableSigData {
    device: DeviceRef,
    vk_ds_layout: vk::DescriptorSetLayout,
    desc_count: DescriptorCount,
}

impl Drop for ArgTableSigData {
    fn drop(&mut self) {
        unsafe {
            self.device
                .vk_device()
                .destroy_descriptor_set_layout(self.vk_ds_layout, None);
        }
    }
}

impl ArgTableSig {
    fn new(
        device: DeviceRef,
        vk_ds_layout: vk::DescriptorSetLayout,
        desc_count: DescriptorCount,
    ) -> Self {
        Self {
            data: Arc::new(ArgTableSigData {
                device,
                vk_ds_layout,
                desc_count,
            }),
        }
    }

    pub fn vk_descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.data.vk_ds_layout
    }

    pub(super) fn desc_count(&self) -> &DescriptorCount {
        &self.data.desc_count
    }
}

/// Implementation of `RootSigBuilder` for Vulkan.
#[derive(Debug)]
pub struct RootSigBuilder {
    device: DeviceRef,
    tables: Vec<Option<ArgTableSig>>,
}

zangfx_impl_object! { RootSigBuilder: base::RootSigBuilder, ::Debug }

impl RootSigBuilder {
    pub(crate) unsafe fn new(device: DeviceRef) -> Self {
        Self {
            device,
            tables: Vec::new(),
        }
    }
}

impl base::RootSigBuilder for RootSigBuilder {
    fn arg_table(
        &mut self,
        index: base::ArgTableIndex,
        x: &base::ArgTableSig,
    ) -> &mut base::RootSigBuilder {
        let our_table: &ArgTableSig = x.downcast_ref().expect("bad argument table signature type");
        if self.tables.len() <= index {
            self.tables.resize(index + 1, None);
        }
        self.tables[index] = Some(our_table.clone());
        self
    }

    fn build(&mut self) -> Result<base::RootSig> {
        let set_layouts: Vec<_> = self.tables
            .iter()
            .map(|x| {
                x.as_ref()
                    .expect("found an empty binding slot")
                    .vk_descriptor_set_layout()
            })
            .collect();

        let info = vk::PipelineLayoutCreateInfo {
            s_type: vk::StructureType::PipelineLayoutCreateInfo,
            p_next: ::null(),
            flags: vk::PipelineLayoutCreateFlags::empty(), // reserved for future use
            set_layout_count: set_layouts.len() as u32,
            p_set_layouts: set_layouts.as_ptr(),
            push_constant_range_count: 0,
            p_push_constant_ranges: ::null(),
        };

        let vk_device = self.device.vk_device();
        let vk_p_layout = unsafe { vk_device.create_pipeline_layout(&info, None) }
            .map_err(translate_generic_error_unwrap)?;
        Ok(RootSig::new(self.device, vk_p_layout).into())
    }
}

/// Implementation of `RootSig` for Vulkan.
#[derive(Debug, Clone)]
pub struct RootSig {
    data: Arc<RootSigData>,
}

zangfx_impl_handle! { RootSig, base::RootSig }

unsafe impl Sync for RootSigData {}
unsafe impl Send for RootSigData {}

#[derive(Debug, Clone)]
struct RootSigData {
    device: DeviceRef,
    vk_p_layout: vk::PipelineLayout,
}

impl Drop for RootSigData {
    fn drop(&mut self) {
        unsafe {
            self.device
                .vk_device()
                .destroy_pipeline_layout(self.vk_p_layout, None);
        }
    }
}

impl RootSig {
    fn new(device: DeviceRef, vk_p_layout: vk::PipelineLayout) -> Self {
        Self {
            data: Arc::new(RootSigData {
                device,
                vk_p_layout,
            }),
        }
    }

    pub fn vk_pipeline_layout(&self) -> vk::PipelineLayout {
        self.data.vk_p_layout
    }
}
