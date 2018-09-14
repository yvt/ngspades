//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Sampler` for Vulkan.
use ash::version::*;
use ash::vk;
use parking_lot::Mutex;
use std::ops::Range;

use crate::device::DeviceRef;
use crate::AshDevice;
use zangfx_base as base;
use zangfx_base::Result;
use zangfx_base::{interfaces, vtable_for, zangfx_impl_handle, zangfx_impl_object};

use crate::utils::{translate_compare_op, translate_generic_error_unwrap};

crate struct SamplerPool {
    samplers: Mutex<Vec<vk::Sampler>>,
}

impl SamplerPool {
    crate fn new() -> Self {
        Self {
            samplers: Mutex::new(Vec::new()),
        }
    }

    crate fn destroy(&mut self, vk_device: &AshDevice) {
        for vk_sampler in self.samplers.get_mut().drain(..) {
            unsafe {
                vk_device.destroy_sampler(vk_sampler, None);
            }
        }
    }
}

/// Implementation of `SamplerBuilder` for Vulkan.
#[derive(Debug)]
pub struct SamplerBuilder {
    device: DeviceRef,
    mag_filter: base::Filter,
    min_filter: base::Filter,
    address_mode: [base::AddressMode; 3],
    mipmap_mode: base::MipmapMode,
    lod_clamp: Range<f32>,
    max_anisotropy: u32,
    cmp_fn: Option<base::CmpFn>,
    border_color: base::BorderColor,
    unnorm_coords: bool,
    label: Option<String>,
}

zangfx_impl_object! { SamplerBuilder: dyn base::SamplerBuilder, dyn (crate::Debug) }

impl SamplerBuilder {
    crate fn new(device: DeviceRef) -> Self {
        Self {
            device,
            mag_filter: base::Filter::Linear,
            min_filter: base::Filter::Linear,
            address_mode: [base::AddressMode::Repeat; 3],
            mipmap_mode: base::MipmapMode::Linear,
            lod_clamp: 0.0..0.0,
            max_anisotropy: 1,
            cmp_fn: None,
            border_color: base::BorderColor::FloatTransparentBlack,
            unnorm_coords: false,
            label: None,
        }
    }
}

impl base::SamplerBuilder for SamplerBuilder {
    fn mag_filter(&mut self, v: base::Filter) -> &mut dyn base::SamplerBuilder {
        self.mag_filter = v;
        self
    }

    fn min_filter(&mut self, v: base::Filter) -> &mut dyn base::SamplerBuilder {
        self.min_filter = v;
        self
    }

    fn address_mode(&mut self, v: &[base::AddressMode]) -> &mut dyn base::SamplerBuilder {
        use zangfx_common::IntoWithPad;
        self.address_mode = v.into_with_pad(v.last().cloned().unwrap_or(base::AddressMode::Repeat));
        self
    }

    fn mipmap_mode(&mut self, v: base::MipmapMode) -> &mut dyn base::SamplerBuilder {
        self.mipmap_mode = v;
        self
    }

    fn lod_clamp(&mut self, v: Range<f32>) -> &mut dyn base::SamplerBuilder {
        self.lod_clamp = v;
        self
    }

    fn max_anisotropy(&mut self, v: u32) -> &mut dyn base::SamplerBuilder {
        self.max_anisotropy = v;
        self
    }

    fn cmp_fn(&mut self, v: Option<base::CmpFn>) -> &mut dyn base::SamplerBuilder {
        self.cmp_fn = v;
        self
    }

    fn border_color(&mut self, v: base::BorderColor) -> &mut dyn base::SamplerBuilder {
        self.border_color = v;
        self
    }

    fn unnorm_coords(&mut self, v: bool) -> &mut dyn base::SamplerBuilder {
        self.unnorm_coords = v;
        self
    }

    fn build(&mut self) -> Result<base::SamplerRef> {
        let info = vk::SamplerCreateInfo {
            s_type: vk::StructureType::SamplerCreateInfo,
            p_next: crate::null(),
            flags: vk::SamplerCreateFlags::empty(), // reserved for future use
            mag_filter: translate_filter(self.mag_filter),
            min_filter: translate_filter(self.mag_filter),
            mipmap_mode: translate_mipmap_mode(self.mipmap_mode),
            address_mode_u: translate_address_mode(self.address_mode[0]),
            address_mode_v: translate_address_mode(self.address_mode[1]),
            address_mode_w: translate_address_mode(self.address_mode[2]),
            mip_lod_bias: 0f32,
            anisotropy_enable: if self.max_anisotropy > 1 {
                vk::VK_TRUE
            } else {
                vk::VK_FALSE
            },
            max_anisotropy: self.max_anisotropy as f32,
            compare_enable: if self.cmp_fn.is_some() {
                vk::VK_TRUE
            } else {
                vk::VK_FALSE
            },
            compare_op: self
                .cmp_fn
                .map(translate_compare_op)
                .unwrap_or(vk::CompareOp::Never),
            min_lod: self.lod_clamp.start,
            max_lod: self.lod_clamp.end,
            border_color: translate_sampler_border_color(self.border_color),
            unnormalized_coordinates: if self.unnorm_coords {
                vk::VK_TRUE
            } else {
                vk::VK_FALSE
            },
        };

        let ref pool = self.device.sampler_pool();
        let mut samplers = pool.samplers.lock();

        // TODO: De-duplicate samplers?

        samplers.reserve(1);

        let vk_device = self.device.vk_device();
        let vk_sampler = unsafe { vk_device.create_sampler(&info, None) }
            .map_err(translate_generic_error_unwrap)?;

        // Insert the created sampler into the global pool so that it is
        // automatically destroyed with the device
        samplers.push(vk_sampler);

        Ok(Sampler { vk_sampler }.into())
    }
}

/// Implementation of `Sampler` for Vulkan.
#[derive(Debug, Clone)]
pub struct Sampler {
    vk_sampler: vk::Sampler,
}

zangfx_impl_handle! { Sampler, base::SamplerRef }

unsafe impl Sync for Sampler {}
unsafe impl Send for Sampler {}

impl Sampler {
    pub unsafe fn from_raw(vk_sampler: vk::Sampler) -> Self {
        Self { vk_sampler }
    }

    pub fn vk_sampler(&self) -> vk::Sampler {
        self.vk_sampler
    }
}

fn translate_filter(value: base::Filter) -> vk::Filter {
    match value {
        base::Filter::Nearest => vk::Filter::Nearest,
        base::Filter::Linear => vk::Filter::Linear,
    }
}

fn translate_mipmap_mode(value: base::MipmapMode) -> vk::SamplerMipmapMode {
    match value {
        base::MipmapMode::Nearest => vk::SamplerMipmapMode::Nearest,
        base::MipmapMode::Linear => vk::SamplerMipmapMode::Linear,
    }
}

fn translate_sampler_border_color(value: base::BorderColor) -> vk::BorderColor {
    match value {
        base::BorderColor::FloatTransparentBlack => vk::BorderColor::FloatTransparentBlack,
        base::BorderColor::FloatOpaqueBlack => vk::BorderColor::FloatOpaqueBlack,
        base::BorderColor::FloatOpaqueWhite => vk::BorderColor::FloatOpaqueWhite,
        base::BorderColor::IntTransparentBlack => vk::BorderColor::IntTransparentBlack,
        base::BorderColor::IntOpaqueBlack => vk::BorderColor::IntOpaqueBlack,
        base::BorderColor::IntOpaqueWhite => vk::BorderColor::IntOpaqueWhite,
    }
}

fn translate_address_mode(value: base::AddressMode) -> vk::SamplerAddressMode {
    match value {
        base::AddressMode::Repeat => vk::SamplerAddressMode::Repeat,
        base::AddressMode::MirroredRepeat => vk::SamplerAddressMode::MirroredRepeat,
        base::AddressMode::ClampToEdge => vk::SamplerAddressMode::ClampToEdge,
        base::AddressMode::ClampToBorderColor => vk::SamplerAddressMode::ClampToBorder,
        // TODO: requires VK_KHR_sampler_mirror_clamp_to_edge!
        base::AddressMode::MirroredClampToEdge => unimplemented!(),
    }
}
