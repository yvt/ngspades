//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use flags_macro::flags;
use std::u32;

use zangfx_base::{self as base, limits, zangfx_impl_object};
use zangfx_metal_rs as metal;

use crate::MEMORY_REGION_GLOBAL;

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
    d24_s8_supported: bool,
}

zangfx_impl_object! { DeviceCaps: dyn limits::DeviceCaps, dyn crate::Debug }

impl DeviceCaps {
    pub(crate) fn new(device: metal::MTLDevice) -> Self {
        assert!(!device.is_null());

        let mtptg: metal::MTLSize = device.max_threads_per_threadgroup();

        assert!(device.supports_feature_set(metal::MTLFeatureSet::OSX_GPUFamily1_v2,));

        // https://developer.apple.com/metal/limits/
        // OSX_GPUFamily1_v2
        let limits = limits::DeviceLimits {
            supports_heap_aliasing: true,
            supports_semaphore: false,
            supports_depth_bounds: false,
            supports_cube_array: true,
            supports_depth_clamp: true,
            supports_fill_mode_non_solid: true,
            supports_independent_blend: true,
            max_image_extent_1d: 16384,
            max_image_extent_2d: 16384,
            max_image_extent_3d: 2048,
            max_image_num_array_layers: 2048,
            max_render_target_extent: 16384,
            max_num_viewports: 1, // TODO: support multiple viewports?
            max_render_target_num_layers: 2048,
            max_compute_workgroup_size: [
                mtptg.width as u32,
                mtptg.height as u32,
                mtptg.depth as u32,
            ],
            max_num_compute_workgroup_invocations: 256,
            max_compute_workgroup_count: [u32::max_value(); 3],
            uniform_buffer_align: crate::UNIFORM_BUFFER_MIN_ALIGN,
            storage_buffer_align: crate::STORAGE_BUFFER_MIN_ALIGN,
        };

        let working_set_size = device.recommended_max_working_set_size();

        let memory_types = [
            limits::MemoryTypeInfo {
                caps: flags![limits::MemoryTypeCapsFlags::{DEVICE_LOCAL}],
                region: MEMORY_REGION_GLOBAL,
            },
            limits::MemoryTypeInfo {
                caps: flags![limits::MemoryTypeCapsFlags::{HOST_VISIBLE | HOST_COHERENT}],
                region: MEMORY_REGION_GLOBAL,
            },
        ];

        let memory_regions = [limits::MemoryRegionInfo {
            size: working_set_size,
        }];

        let queue_families = [limits::QueueFamilyInfo {
            caps: flags![limits::QueueFamilyCapsFlags::{RENDER | COMPUTE | COPY}],
            count: <usize>::max_value(),
        }];

        Self {
            limits,
            memory_types,
            memory_regions,
            queue_families,
            d24_s8_supported: device.d24_s8_supported(),
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
        use crate::formats::translate_image_format;
        use zangfx_base::formats::ImageFormat;
        use zangfx_base::formats::Normalizedness::*;
        use zangfx_base::formats::Signedness::*;
        use zangfx_base::limits::ImageFormatCapsFlags;

        let trans = flags![ImageFormatCapsFlags::{COPY_READ | COPY_WRITE}];
        let all = flags![ImageFormatCapsFlags::{SAMPLED | SAMPLED_FILTER_LINEAR | STORAGE | RENDER | RENDER_BLEND}]
            | trans; // + MSAA w/Resolve

        // "Unavailable"
        let empty = flags![ImageFormatCapsFlags::{}];

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
            ImageFormat::SrgbRgba8 => {
                flags![ImageFormatCapsFlags::{SAMPLED | SAMPLED_FILTER_LINEAR | RENDER | RENDER_BLEND}]
                    | trans
            } // + MSAA w/Resolve
            ImageFormat::SrgbBgra8 => {
                flags![ImageFormatCapsFlags::{SAMPLED | SAMPLED_FILTER_LINEAR | RENDER | RENDER_BLEND}]
                    | trans
            } // + MSAA w/Resolve

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
            | ImageFormat::Rgba32(_, Unnormalized) => {
                flags![ImageFormatCapsFlags::{SAMPLED | STORAGE | RENDER}] | trans
            } // + MSAA

            ImageFormat::R32(_, Normalized) => undefined,
            ImageFormat::Rg32(_, Normalized) => undefined,
            ImageFormat::Rgba32(_, Normalized) => undefined,

            // Since macOS_GPUFamily1_v2 (macOS 10.12)
            ImageFormat::Depth16 => {
                flags![ImageFormatCapsFlags::{SAMPLED | SAMPLED_FILTER_LINEAR | RENDER}] | trans
            } // + MSAA w/Resolve

            ImageFormat::Depth24 => undefined,
            ImageFormat::Depth24Stencil8 => {
                if self.d24_s8_supported {
                    flags![ImageFormatCapsFlags::{SAMPLED | SAMPLED_FILTER_LINEAR | RENDER}] | trans
                } else {
                    empty
                }
            }
            ImageFormat::DepthFloat32 | ImageFormat::DepthFloat32Stencil8 => {
                flags![ImageFormatCapsFlags::{SAMPLED | SAMPLED_FILTER_LINEAR | RENDER}] | trans
            } // + MSAA w/Resolve
        }
    }

    fn vertex_format_caps(
        &self,
        format: base::formats::VertexFormat,
    ) -> limits::VertexFormatCapsFlags {
        use crate::formats::translate_vertex_format;
        if translate_vertex_format(format).is_some() {
            limits::VertexFormatCapsFlags::VERTEX
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
