//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Sampler` for Metal.
use std::ops::Range;
use zangfx_base::Result;
use zangfx_base::{self as base, sampler, CmpFn};
use zangfx_base::{zangfx_impl_handle, zangfx_impl_object};
use zangfx_metal_rs as metal;
use zangfx_metal_rs::NSObjectProtocol;

use crate::utils::{nil_error, translate_cmp_fn, OCPtr};

/// Implementation of `SamplerBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct SamplerBuilder {
    metal_device: OCPtr<metal::MTLDevice>,
    mag_filter: sampler::Filter,
    min_filter: sampler::Filter,
    address_mode: [sampler::AddressMode; 3],
    mipmap_mode: sampler::MipmapMode,
    lod_clamp: Range<f32>,
    max_anisotropy: u32,
    cmp_fn: Option<CmpFn>,
    border_color: sampler::BorderColor,
    unnorm_coords: bool,
    label: Option<String>,
}

zangfx_impl_object! { SamplerBuilder: dyn sampler::SamplerBuilder, dyn crate::Debug, dyn base::SetLabel }

unsafe impl Send for SamplerBuilder {}
unsafe impl Sync for SamplerBuilder {}

impl SamplerBuilder {
    /// Construct a `SamplerBuilder`.
    ///
    /// It's up to the caller to make sure `metal_device` is valid.
    pub unsafe fn new(metal_device: metal::MTLDevice) -> Self {
        Self {
            metal_device: OCPtr::new(metal_device).expect("nil device"),
            mag_filter: sampler::Filter::Linear,
            min_filter: sampler::Filter::Linear,
            address_mode: [sampler::AddressMode::Repeat; 3],
            mipmap_mode: sampler::MipmapMode::Linear,
            lod_clamp: 0.0..0.0,
            max_anisotropy: 1,
            cmp_fn: None,
            border_color: sampler::BorderColor::FloatTransparentBlack,
            unnorm_coords: false,
            label: None,
        }
    }
}

impl base::SetLabel for SamplerBuilder {
    fn set_label(&mut self, label: &str) {
        self.label = Some(label.to_owned());
    }
}

impl sampler::SamplerBuilder for SamplerBuilder {
    fn mag_filter(&mut self, v: sampler::Filter) -> &mut dyn sampler::SamplerBuilder {
        self.mag_filter = v;
        self
    }

    fn min_filter(&mut self, v: sampler::Filter) -> &mut dyn sampler::SamplerBuilder {
        self.min_filter = v;
        self
    }

    fn address_mode(&mut self, v: &[sampler::AddressMode]) -> &mut dyn sampler::SamplerBuilder {
        use zangfx_common::IntoWithPad;
        self.address_mode =
            v.into_with_pad(v.last().cloned().unwrap_or(sampler::AddressMode::Repeat));
        self
    }

    fn mipmap_mode(&mut self, v: sampler::MipmapMode) -> &mut dyn sampler::SamplerBuilder {
        self.mipmap_mode = v;
        self
    }

    fn lod_clamp(&mut self, v: Range<f32>) -> &mut dyn sampler::SamplerBuilder {
        self.lod_clamp = v;
        self
    }

    fn max_anisotropy(&mut self, v: u32) -> &mut dyn sampler::SamplerBuilder {
        self.max_anisotropy = v;
        self
    }

    fn cmp_fn(&mut self, v: Option<CmpFn>) -> &mut dyn sampler::SamplerBuilder {
        self.cmp_fn = v;
        self
    }

    fn border_color(&mut self, v: sampler::BorderColor) -> &mut dyn sampler::SamplerBuilder {
        self.border_color = v;
        self
    }

    fn unnorm_coords(&mut self, v: bool) -> &mut dyn sampler::SamplerBuilder {
        self.unnorm_coords = v;
        self
    }

    fn build(&mut self) -> Result<base::SamplerRef> {
        let metal_desc = unsafe { OCPtr::from_raw(metal::MTLSamplerDescriptor::new()) }
            .ok_or(nil_error("MTLSamplerDescriptor new"))?;
        metal_desc.set_min_filter(translate_filter(self.min_filter));
        metal_desc.set_mag_filter(translate_filter(self.mag_filter));
        metal_desc.set_mip_filter(if self.unnorm_coords {
            metal::MTLSamplerMipFilter::NotMipmapped
        } else {
            translate_mipmap_mode(self.mipmap_mode)
        });
        metal_desc.set_address_mode_s(translate_address_mode(self.address_mode[0]));
        metal_desc.set_address_mode_t(translate_address_mode(self.address_mode[1]));
        metal_desc.set_address_mode_r(translate_address_mode(self.address_mode[2]));
        metal_desc.set_max_anisotropy(self.max_anisotropy as u64);
        metal_desc.set_compare_function(translate_cmp_fn(self.cmp_fn.unwrap_or(CmpFn::Never)));
        metal_desc.set_lod_min_clamp(self.lod_clamp.start);
        metal_desc.set_lod_max_clamp(self.lod_clamp.end);
        metal_desc.set_border_color(translate_border_color(self.border_color));
        metal_desc.set_normalized_coordinates(!self.unnorm_coords);
        metal_desc.set_support_argument_buffers(true);

        if let Some(ref label) = self.label {
            metal_desc.set_label(label);
        }

        let metal_sampler = self.metal_device.new_sampler(*metal_desc);
        if metal_sampler.is_null() {
            return Err(nil_error("MTLDevice newSamplerStateWithDescriptor:"));
        }
        unsafe {
            metal_sampler.retain();
            Ok(base::SamplerRef::new(Sampler::from_raw(metal_sampler)))
        }
    }
}

fn translate_filter(value: sampler::Filter) -> metal::MTLSamplerMinMagFilter {
    use self::sampler::Filter::*;
    match value {
        Linear => metal::MTLSamplerMinMagFilter::Linear,
        Nearest => metal::MTLSamplerMinMagFilter::Nearest,
    }
}

fn translate_mipmap_mode(value: sampler::MipmapMode) -> metal::MTLSamplerMipFilter {
    use self::sampler::MipmapMode::*;
    // No NgsGFX mipmap mode corresponds to MTLSamplerMipFilterNotMipmapped.
    // This behavior can be emulated by using lod_min_clamp = 0 and lod_max_clamp = 0.25.
    match value {
        Nearest => metal::MTLSamplerMipFilter::Nearest,
        Linear => metal::MTLSamplerMipFilter::Linear,
    }
}

fn translate_address_mode(value: sampler::AddressMode) -> metal::MTLSamplerAddressMode {
    use self::sampler::AddressMode::*;
    match value {
        Repeat => metal::MTLSamplerAddressMode::Repeat,
        ClampToEdge => metal::MTLSamplerAddressMode::ClampToEdge,
        ClampToBorderColor => metal::MTLSamplerAddressMode::ClampToBorderColor,
        MirroredRepeat => metal::MTLSamplerAddressMode::MirrorRepeat,
        MirroredClampToEdge => metal::MTLSamplerAddressMode::MirrorClampToEdge,
    }
}

fn translate_border_color(value: sampler::BorderColor) -> metal::MTLSamplerBorderColor {
    use self::sampler::BorderColor::*;
    match value {
        FloatOpaqueWhite | IntOpaqueWhite => metal::MTLSamplerBorderColor::OpaqueWhite,
        FloatOpaqueBlack | IntOpaqueBlack => metal::MTLSamplerBorderColor::OpaqueBlack,
        FloatTransparentBlack | IntTransparentBlack => {
            metal::MTLSamplerBorderColor::TransparentBlack
        }
    }
}

/// Implementation of `Sampler` for Metal.
#[derive(Debug, Clone)]
pub struct Sampler {
    metal_sampler: OCPtr<metal::MTLSamplerState>,
}

zangfx_impl_handle! { Sampler, base::SamplerRef }

unsafe impl Send for Sampler {}
unsafe impl Sync for Sampler {}

impl Sampler {
    pub unsafe fn from_raw(metal_sampler: metal::MTLSamplerState) -> Self {
        Self {
            metal_sampler: OCPtr::new(metal_sampler).expect("nil sampler"),
        }
    }

    pub fn metal_sampler(&self) -> metal::MTLSamplerState {
        *self.metal_sampler
    }
}
