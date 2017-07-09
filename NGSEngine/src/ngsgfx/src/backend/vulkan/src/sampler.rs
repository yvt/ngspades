//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use ash::vk;
use ash::version::DeviceV1_0;
use std::ptr;

use {RefEqArc, DeviceRef, AshDevice, translate_generic_error_unwrap, translate_compare_function};

pub struct Sampler<T: DeviceRef> {
    data: RefEqArc<SamplerData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for Sampler<T> => data
}

#[derive(Debug)]
struct SamplerData<T: DeviceRef> {
    device_ref: T,
    handle: vk::Sampler,
}

impl<T: DeviceRef> Sampler<T> {
    pub(crate) fn new(device_ref: &T, desc: &core::SamplerDescription) -> core::Result<Self> {
        let info = vk::SamplerCreateInfo {
            s_type: vk::StructureType::SamplerCreateInfo,
            p_next: ptr::null(),
            flags: vk::SamplerCreateFlags::empty(), // reserved for future use
            mag_filter: translate_filter(desc.mag_filter),
            min_filter: translate_filter(desc.mag_filter),
            mipmap_mode: translate_mipmap_mode(desc.mipmap_mode),
            address_mode_u: translate_address_mode(desc.address_mode[0]),
            address_mode_v: translate_address_mode(desc.address_mode[1]),
            address_mode_w: translate_address_mode(desc.address_mode[2]),
            mip_lod_bias: 0f32,
            anisotropy_enable: if desc.max_anisotropy > 1 {
                vk::VK_TRUE
            } else {
                vk::VK_FALSE
            },
            max_anisotropy: desc.max_anisotropy as f32,
            compare_enable: if desc.compare_function.is_some() {
                vk::VK_TRUE
            } else {
                vk::VK_FALSE
            },
            compare_op: desc.compare_function
                .map(translate_compare_function)
                .unwrap_or(vk::CompareOp::Never),
            min_lod: desc.lod_min_clamp,
            max_lod: desc.lod_max_clamp,
            border_color: translate_sampler_border_color(desc.border_color),
            unnormalized_coordinates: if desc.unnormalized_coordinates {
                vk::VK_TRUE
            } else {
                vk::VK_FALSE
            },
        };

        let device_ref = device_ref.clone();
        let handle;
        {
            let device: &AshDevice = device_ref.device();
            handle = unsafe { device.create_sampler(&info, device_ref.allocation_callbacks()) }
                .map_err(translate_generic_error_unwrap)?;
        }

        Ok(Sampler {
            data: RefEqArc::new(SamplerData { device_ref, handle }),
        })
    }

    pub(crate) fn device_ref(&self) -> &T {
        &self.data.device_ref
    }

    pub fn handle(&self) -> vk::Sampler {
        self.data.handle
    }
}

fn translate_filter(value: core::Filter) -> vk::Filter {
    match value {
        core::Filter::Nearest => vk::Filter::Nearest,
        core::Filter::Linear => vk::Filter::Linear,
    }
}

fn translate_mipmap_mode(value: core::MipmapMode) -> vk::SamplerMipmapMode {
    match value {
        core::MipmapMode::Nearest => vk::SamplerMipmapMode::Nearest,
        core::MipmapMode::Linear => vk::SamplerMipmapMode::Linear,
    }
}

fn translate_sampler_border_color(value: core::SamplerBorderColor) -> vk::BorderColor {
    match value {
        core::SamplerBorderColor::FloatTransparentBlack => vk::BorderColor::FloatTransparentBlack,
        core::SamplerBorderColor::FloatOpaqueBlack => vk::BorderColor::FloatOpaqueBlack,
        core::SamplerBorderColor::FloatOpaqueWhite => vk::BorderColor::FloatOpaqueWhite,
        core::SamplerBorderColor::IntTransparentBlack => vk::BorderColor::IntTransparentBlack,
        core::SamplerBorderColor::IntOpaqueBlack => vk::BorderColor::IntOpaqueBlack,
        core::SamplerBorderColor::IntOpaqueWhite => vk::BorderColor::IntOpaqueWhite,
    }
}

fn translate_address_mode(value: core::SamplerAddressMode) -> vk::SamplerAddressMode {
    match value {
        core::SamplerAddressMode::Repeat => vk::SamplerAddressMode::Repeat,
        core::SamplerAddressMode::MirroredRepeat => vk::SamplerAddressMode::MirroredRepeat,
        core::SamplerAddressMode::ClampToEdge => vk::SamplerAddressMode::ClampToEdge,
        core::SamplerAddressMode::ClampToBorderColor => vk::SamplerAddressMode::ClampToBorder,
        // TODO: requires VK_KHR_sampler_mirror_clamp_to_edge!
        core::SamplerAddressMode::MirroredClampToEdge => unimplemented!(),
    }
}

impl<T: DeviceRef> Drop for SamplerData<T> {
    fn drop(&mut self) {
        let device: &AshDevice = self.device_ref.device();
        unsafe { device.destroy_sampler(self.handle, self.device_ref.allocation_callbacks()) };
    }
}

impl<T: DeviceRef> core::Sampler for Sampler<T> {}

impl<T: DeviceRef> core::Marker for Sampler<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}
