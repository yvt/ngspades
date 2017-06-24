//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use metal;

use {RefEqArc, OCPtr, translate_compare_function};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Sampler {
    data: RefEqArc<SamplerData>,
}

#[derive(Debug)]
struct SamplerData {
    metal_sampler: OCPtr<metal::MTLSamplerState>,
}

unsafe impl Send for SamplerData {}
unsafe impl Sync for SamplerData {} // no interior mutability

impl core::Marker for Sampler {
    fn set_label(&self, label: Option<&str>) {
        self.data.metal_sampler.set_label(label.unwrap_or(""));
    }
}

impl core::Sampler for Sampler {}

impl Sampler {
    pub(crate) fn new(
        metal_device: metal::MTLDevice,
        desc: &core::SamplerDescription,
    ) -> core::Result<Sampler> {
        let metal_desc =
            unsafe { OCPtr::from_raw(metal::MTLSamplerDescriptor::alloc().init()).unwrap() };
        metal_desc.set_min_filter(translate_filter(desc.min_filter));
        metal_desc.set_mag_filter(translate_filter(desc.mag_filter));
        metal_desc.set_mip_filter(if desc.unnormalized_coordinates {
            metal::MTLSamplerMipFilter::NotMipmapped
        } else {
            translate_mipmap_mode(desc.mipmap_mode)
        });
        metal_desc.set_address_mode_s(translate_address_mode(desc.address_mode[0]));
        metal_desc.set_address_mode_t(translate_address_mode(desc.address_mode[1]));
        metal_desc.set_address_mode_r(translate_address_mode(desc.address_mode[2]));
        metal_desc.set_max_anisotropy(desc.max_anisotropy as u64);
        metal_desc.set_compare_function(
            translate_compare_function(desc.compare_function.unwrap_or(
                core::CompareFunction::Never,
            )),
        );
        metal_desc.set_lod_min_clamp(desc.lod_min_clamp);
        metal_desc.set_lod_max_clamp(desc.lod_max_clamp);
        // TODO: set_border_color requires macOS 10.12+. Add OS version check?
        metal_desc.set_border_color(translate_border_color(desc.border_color));
        metal_desc.set_normalized_coordinates(!desc.unnormalized_coordinates);

        let metal_sampler = unsafe { OCPtr::from_raw(metal_device.new_sampler(*metal_desc)) }
            .ok_or(core::GenericError::OutOfDeviceMemory)?;
        let data = SamplerData { metal_sampler };

        Ok(Self { data: RefEqArc::new(data) })
    }

    pub fn metal_sampler_state(&self) -> metal::MTLSamplerState {
        *self.data.metal_sampler
    }
}

fn translate_filter(value: core::Filter) -> metal::MTLSamplerMinMagFilter {
    match value {
        core::Filter::Linear => metal::MTLSamplerMinMagFilter::Linear,
        core::Filter::Nearest => metal::MTLSamplerMinMagFilter::Nearest,
    }
}

fn translate_mipmap_mode(value: core::MipmapMode) -> metal::MTLSamplerMipFilter {
    // No NgsGFX mipmap mode corresponds to MTLSamplerMipFilterNotMipmapped.
    // This behavior can be emulated by using lod_min_clamp = 0 and lod_max_clamp = 0.25.
    match value {
        core::MipmapMode::Nearest => metal::MTLSamplerMipFilter::Nearest,
        core::MipmapMode::Linear => metal::MTLSamplerMipFilter::Linear,
    }
}

fn translate_address_mode(value: core::SamplerAddressMode) -> metal::MTLSamplerAddressMode {
    match value {
        core::SamplerAddressMode::Repeat => metal::MTLSamplerAddressMode::Repeat,
        core::SamplerAddressMode::ClampToEdge => metal::MTLSamplerAddressMode::ClampToEdge,
        // TODO: ClampToBorderColor requires macOS 10.12+. Add OS version check?
        core::SamplerAddressMode::ClampToBorderColor => {
            metal::MTLSamplerAddressMode::ClampToBorderColor
        }
        core::SamplerAddressMode::MirroredRepeat => metal::MTLSamplerAddressMode::MirrorRepeat,
        core::SamplerAddressMode::MirroredClampToEdge => {
            metal::MTLSamplerAddressMode::MirrorClampToEdge
        }
    }
}

fn translate_border_color(value: core::SamplerBorderColor) -> metal::MTLSamplerBorderColor {
    match value {
        core::SamplerBorderColor::FloatOpaqueWhite |
        core::SamplerBorderColor::IntOpaqueWhite => metal::MTLSamplerBorderColor::OpaqueWhite,
        core::SamplerBorderColor::FloatOpaqueBlack |
        core::SamplerBorderColor::IntOpaqueBlack => metal::MTLSamplerBorderColor::OpaqueBlack,
        core::SamplerBorderColor::FloatTransparentBlack |
        core::SamplerBorderColor::IntTransparentBlack => {
            metal::MTLSamplerBorderColor::TransparentBlack
        }
    }
}
