//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use metal;
use cgmath::Vector3;

use std::u32;

/// Feature sets for Metal.
///
/// See https://developer.apple.com/metal/limits/ for the feature table.
///
/// TODO: There are some discrepancies on the table between the lastest version
/// and the older one when macOS 10.12 was the lastest version os macOS
///
/// TODO: Handle `OSX_ReadWriteTextureTier2` correctly
///
/// It currently assumes the support of the feature set `OSX_GPUFamily1_v2`.
#[derive(Debug)]
pub struct DeviceCapabilities {
    limits: core::DeviceLimits,
}

impl DeviceCapabilities {
    pub(crate) fn new(device: metal::MTLDevice) -> Self {
        assert!(!device.is_null());

        let mtptg: metal::MTLSize = device.max_threads_per_threadgroup();

        assert!(device.supports_feature_set(
            metal::MTLFeatureSet::OSX_GPUFamily1_v2,
        ));

        // https://developer.apple.com/metal/limits/
        // OSX_GPUFamily1_v2
        let limits = core::DeviceLimits {
            supports_specialized_heap: false,
            supports_heap_aliasing: false,
            supports_depth_bounds: false,
            supports_cube_array: true,
            supports_depth_clamp: true,
            supports_fill_mode_non_solid: true,
            max_image_extent_1d: 16384,
            max_image_extent_2d: 16384,
            max_image_extent_3d: 2048,
            max_image_num_array_layers: 2048,
            max_framebuffer_extent: 16384,
            max_compute_workgroup_size: Vector3::new(
                mtptg.width as u32,
                mtptg.height as u32,
                mtptg.depth as u32,
            ),
            max_num_compute_workgroup_invocations: 256,
            max_compute_workgroup_count: Vector3::new(
                u32::max_value(),
                u32::max_value(),
                u32::max_value(),
            ),
        };

        Self { limits }
    }
}

impl core::DeviceCapabilities for DeviceCapabilities {
    fn limits(&self) -> &core::DeviceLimits {
        &self.limits
    }

    fn image_format_features(
        &self,
        format: core::ImageFormat,
        tiling: core::ImageTiling,
    ) -> core::ImageFormatFeatureFlags {
        use imp::translate_image_format;
        use core::ImageFormat;
        use core::Signedness::*;
        use core::Normalizedness::*;
        use core::ImageFormatFeature::*;

        let trans = TransferSource | TransferDestination;
        let all = Sampled | SampledFilterLinear | Storage | ColorAttachment |
            ColorAttachmentBlend | DepthStencilAttachment | trans; // + MSAA w/Resolve

        // "Unavailable"
        let empty = core::ImageFormatFeatureFlags::empty();

        // Not supported by Metal at this point
        let undefined = empty;

        if translate_image_format(format).is_none() {
            // `translate_image_format` does not support some formats even if
            // they are actually supported by Metal and some feature sets
            return empty;
        }

        if tiling == core::ImageTiling::Linear {
            // Not supported on macOS 10.12 yet
            return empty;
        }

        match format {
            ImageFormat::SrgbR8 => empty,
            ImageFormat::SrgbRg8 => empty,
            ImageFormat::SrgbRgba8 => {
                Sampled | SampledFilterLinear | ColorAttachment | ColorAttachmentBlend | trans
            } // + MSAA w/Resolve
            ImageFormat::SrgbBgra8 => {
                Sampled | SampledFilterLinear | ColorAttachment | ColorAttachmentBlend | trans
            } // + MSAA w/Resolve

            ImageFormat::Rgb10A2(Signed, _) => undefined,

            ImageFormat::R8(_, Normalized) |
            ImageFormat::Rg8(_, Normalized) |
            ImageFormat::Rgba8(_, Normalized) |
            ImageFormat::Bgra8(_, Normalized) |
            ImageFormat::R16(_, Normalized) |
            ImageFormat::Rg16(_, Normalized) |
            ImageFormat::Rgba16(_, Normalized) |
            ImageFormat::RFloat16 |
            ImageFormat::RgFloat16 |
            ImageFormat::RgbaFloat16 |
            ImageFormat::Rgb10A2(Unsigned, Normalized) |
            ImageFormat::RFloat32 |
            ImageFormat::RgFloat32 |
            ImageFormat::RgbaFloat32 => all,

            ImageFormat::R8(_, Unnormalized) |
            ImageFormat::Rg8(_, Unnormalized) |
            ImageFormat::Rgba8(_, Unnormalized) |
            ImageFormat::Bgra8(_, Unnormalized) |
            ImageFormat::R16(_, Unnormalized) |
            ImageFormat::Rg16(_, Unnormalized) |
            ImageFormat::Rgba16(_, Unnormalized) |
            ImageFormat::Rgb10A2(Unsigned, Unnormalized) |
            ImageFormat::R32(_, Unnormalized) |
            ImageFormat::Rg32(_, Unnormalized) |
            ImageFormat::Rgba32(_, Unnormalized) => Sampled | Storage | ColorAttachment | trans, // + MSAA

            ImageFormat::R32(_, Normalized) => undefined,
            ImageFormat::Rg32(_, Normalized) => undefined,
            ImageFormat::Rgba32(_, Normalized) => undefined,

            // Since macOS_GPUFamily1_v2 (macOS 10.12)
            ImageFormat::Depth16 => Sampled | SampledFilterLinear | DepthStencilAttachment | trans, // + MSAA w/Resolve

            ImageFormat::Depth24 => undefined,
            ImageFormat::Depth24Stencil8 |
            ImageFormat::DepthFloat32 |
            ImageFormat::DepthFloat32Stencil8 => {
                Sampled | SampledFilterLinear | DepthStencilAttachment | trans
            } // + MSAA w/Resolve
        }
    }

    fn vertex_format_features(&self, format: core::VertexFormat) -> core::VertexFormatFeatureFlags {
        use imp::translate_vertex_format;
        if translate_vertex_format(format).is_some() {
            core::VertexFormatFeature::VertexBuffer.into()
        } else {
            core::VertexFormatFeatureFlags::empty()
        }
    }
}
