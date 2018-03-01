//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::u32;

use {base, metal};
use base::limits;

use MEMORY_REGION_GLOBAL;

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
pub struct DeviceCaps {
    limits: limits::DeviceLimits,
    memory_types: [limits::MemoryTypeInfo; 2],
    memory_regions: [limits::MemoryRegionInfo; 1],
    queue_families: [limits::QueueFamilyInfo; 1],
}

zangfx_impl_object! { DeviceCaps: limits::DeviceCaps, ::Debug }

impl DeviceCaps {
    pub(crate) fn new(device: metal::MTLDevice) -> Self {
        assert!(!device.is_null());

        let mtptg: metal::MTLSize = device.max_threads_per_threadgroup();

        assert!(device.supports_feature_set(metal::MTLFeatureSet::OSX_GPUFamily1_v2,));

        // https://developer.apple.com/metal/limits/
        // OSX_GPUFamily1_v2
        let limits = limits::DeviceLimits {
            supports_heap_aliasing: true,
            supports_depth_bounds: false,
            supports_cube_array: true,
            supports_depth_clamp: true,
            supports_fill_mode_non_solid: true,
            max_image_extent_1d: 16384,
            max_image_extent_2d: 16384,
            max_image_extent_3d: 2048,
            max_image_num_array_layers: 2048,
            max_framebuffer_extent: 16384,
            max_compute_workgroup_size: [
                mtptg.width as u32,
                mtptg.height as u32,
                mtptg.depth as u32,
            ],
            max_num_compute_workgroup_invocations: 256,
            max_compute_workgroup_count: [u32::max_value(); 3],
        };

        let working_set_size = device.recommended_max_working_set_size();

        let memory_types = [
            limits::MemoryTypeInfo {
                caps: flags![limits::MemoryTypeCaps::{DeviceLocal}],
                region: MEMORY_REGION_GLOBAL,
            },
            limits::MemoryTypeInfo {
                caps: flags![limits::MemoryTypeCaps::{HostVisible | HostCoherent}],
                region: MEMORY_REGION_GLOBAL,
            },
        ];

        let memory_regions = [
            limits::MemoryRegionInfo {
                size: working_set_size,
            },
        ];

        let queue_families = [
            limits::QueueFamilyInfo {
                caps: flags![limits::QueueFamilyCaps::{Render | Compute | Copy}],
                count: <usize>::max_value(),
            },
        ];

        Self {
            limits,
            memory_types,
            memory_regions,
            queue_families,
        }
    }
}

impl limits::DeviceCaps for DeviceCaps {
    fn limits(&self) -> &limits::DeviceLimits {
        &self.limits
    }

    fn image_format_caps(
        &self,
        format: base::formats::ImageFormat,
    ) -> limits::ImageFormatCapsFlags {
        use formats::translate_image_format;
        use base::formats::ImageFormat;
        use base::formats::Signedness::*;
        use base::formats::Normalizedness::*;
        use base::limits::ImageFormatCaps::*;

        let trans = CopyRead | CopyWrite;
        let all = Sampled | SampledFilterLinear | Storage | Render | RenderBlend | trans; // + MSAA w/Resolve

        // "Unavailable"
        let empty = limits::ImageFormatCapsFlags::empty();

        // Not supported by Metal at this point
        let undefined = empty;

        if translate_image_format(format).is_none() {
            // `translate_image_format` does not support some formats even if
            // they are actually supported by Metal and some feature sets
            return empty;
        }

        match format {
            ImageFormat::SrgbR8 => empty,
            ImageFormat::SrgbRg8 => empty,
            ImageFormat::SrgbRgba8 => Sampled | SampledFilterLinear | Render | RenderBlend | trans, // + MSAA w/Resolve
            ImageFormat::SrgbBgra8 => Sampled | SampledFilterLinear | Render | RenderBlend | trans, // + MSAA w/Resolve

            ImageFormat::Rgb10A2(Signed, _) => undefined,

            ImageFormat::R8(_, Normalized)
            | ImageFormat::Rg8(_, Normalized)
            | ImageFormat::Rgba8(_, Normalized)
            | ImageFormat::Bgra8(_, Normalized)
            | ImageFormat::R16(_, Normalized)
            | ImageFormat::Rg16(_, Normalized)
            | ImageFormat::Rgba16(_, Normalized)
            | ImageFormat::RFloat16
            | ImageFormat::RgFloat16
            | ImageFormat::RgbaFloat16
            | ImageFormat::Rgb10A2(Unsigned, Normalized)
            | ImageFormat::RFloat32
            | ImageFormat::RgFloat32
            | ImageFormat::RgbaFloat32 => all,

            ImageFormat::R8(_, Unnormalized)
            | ImageFormat::Rg8(_, Unnormalized)
            | ImageFormat::Rgba8(_, Unnormalized)
            | ImageFormat::Bgra8(_, Unnormalized)
            | ImageFormat::R16(_, Unnormalized)
            | ImageFormat::Rg16(_, Unnormalized)
            | ImageFormat::Rgba16(_, Unnormalized)
            | ImageFormat::Rgb10A2(Unsigned, Unnormalized)
            | ImageFormat::R32(_, Unnormalized)
            | ImageFormat::Rg32(_, Unnormalized)
            | ImageFormat::Rgba32(_, Unnormalized) => Sampled | Storage | Render | trans, // + MSAA

            ImageFormat::R32(_, Normalized) => undefined,
            ImageFormat::Rg32(_, Normalized) => undefined,
            ImageFormat::Rgba32(_, Normalized) => undefined,

            // Since macOS_GPUFamily1_v2 (macOS 10.12)
            ImageFormat::Depth16 => Sampled | SampledFilterLinear | Render | trans, // + MSAA w/Resolve

            ImageFormat::Depth24 => undefined,
            ImageFormat::Depth24Stencil8
            | ImageFormat::DepthFloat32
            | ImageFormat::DepthFloat32Stencil8 => Sampled | SampledFilterLinear | Render | trans, // + MSAA w/Resolve
        }
    }

    fn vertex_format_caps(
        &self,
        format: base::formats::VertexFormat,
    ) -> limits::VertexFormatCapsFlags {
        use formats::translate_vertex_format;
        if translate_vertex_format(format).is_some() {
            limits::VertexFormatCaps::Vertex.into()
        } else {
            limits::VertexFormatCapsFlags::empty()
        }
    }

    fn memory_types(&self) -> &[limits::MemoryTypeInfo] {
        &self.memory_types
    }

    fn memory_regions(&self) -> &[limits::MemoryRegionInfo] {
        &self.memory_regions
    }

    fn queue_families(&self) -> &[limits::QueueFamilyInfo] {
        &self.queue_families
    }
}
