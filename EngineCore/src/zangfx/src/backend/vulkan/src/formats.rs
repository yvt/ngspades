//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::vk::{self, Format};
use zangfx_base::{
    ImageFormat,
    Normalizedness::{Normalized, Unnormalized},
    ScalarFormat::{F32, I16, I32, I8},
    Signedness::{Signed, Unsigned},
    VecWidth::{Scalar, Vector2, Vector3, Vector4},
    VertexFormat,
};

pub fn translate_image_format(format: ImageFormat) -> Option<vk::Format> {
    match format {
        ImageFormat::R8(Signed, Normalized) => Some(Format::R8_SNORM),
        ImageFormat::R8(Signed, Unnormalized) => Some(Format::R8_SINT),
        ImageFormat::R8(Unsigned, Normalized) => Some(Format::R8_UNORM),
        ImageFormat::R8(Unsigned, Unnormalized) => Some(Format::R8_UINT),
        ImageFormat::SrgbR8 => Some(Format::R8_SRGB),
        ImageFormat::Rg8(Signed, Normalized) => Some(Format::R8G8_SNORM),
        ImageFormat::Rg8(Signed, Unnormalized) => Some(Format::R8G8_SINT),
        ImageFormat::Rg8(Unsigned, Normalized) => Some(Format::R8G8_UNORM),
        ImageFormat::Rg8(Unsigned, Unnormalized) => Some(Format::R8G8_UINT),
        ImageFormat::SrgbRg8 => Some(Format::R8G8_SRGB),
        ImageFormat::Rgba8(Signed, Normalized) => Some(Format::R8G8B8A8_SNORM),
        ImageFormat::Rgba8(Signed, Unnormalized) => Some(Format::R8G8B8A8_SINT),
        ImageFormat::Rgba8(Unsigned, Normalized) => Some(Format::R8G8B8A8_UNORM),
        ImageFormat::Rgba8(Unsigned, Unnormalized) => Some(Format::R8G8B8A8_UINT),
        ImageFormat::SrgbRgba8 => Some(Format::R8G8B8A8_SRGB),

        ImageFormat::Bgra8(Signed, Normalized) => Some(Format::B8G8R8A8_SNORM),
        ImageFormat::Bgra8(Signed, Unnormalized) => Some(Format::B8G8R8A8_SINT),
        ImageFormat::Bgra8(Unsigned, Normalized) => Some(Format::B8G8R8A8_UNORM),
        ImageFormat::Bgra8(Unsigned, Unnormalized) => Some(Format::B8G8R8A8_UINT),
        ImageFormat::SrgbBgra8 => Some(Format::B8G8R8A8_SRGB),

        ImageFormat::Rgb10A2(Signed, Normalized) => Some(Format::A2B10G10R10_SNORM_PACK32),
        ImageFormat::Rgb10A2(Signed, Unnormalized) => Some(Format::A2B10G10R10_SINT_PACK32),
        ImageFormat::Rgb10A2(Unsigned, Normalized) => Some(Format::A2B10G10R10_UNORM_PACK32),
        ImageFormat::Rgb10A2(Unsigned, Unnormalized) => Some(Format::A2B10G10R10_UINT_PACK32),

        ImageFormat::R16(Signed, Normalized) => Some(Format::R16_SNORM),
        ImageFormat::R16(Signed, Unnormalized) => Some(Format::R16_SINT),
        ImageFormat::R16(Unsigned, Normalized) => Some(Format::R16_UNORM),
        ImageFormat::R16(Unsigned, Unnormalized) => Some(Format::R16_UINT),
        ImageFormat::RFloat16 => Some(Format::R16_SFLOAT),
        ImageFormat::Rg16(Signed, Normalized) => Some(Format::R16G16_SNORM),
        ImageFormat::Rg16(Signed, Unnormalized) => Some(Format::R16G16_SINT),
        ImageFormat::Rg16(Unsigned, Normalized) => Some(Format::R16G16_UNORM),
        ImageFormat::Rg16(Unsigned, Unnormalized) => Some(Format::R16G16_UINT),
        ImageFormat::RgFloat16 => Some(Format::R16G16_SFLOAT),
        ImageFormat::Rgba16(Signed, Normalized) => Some(Format::R16G16B16A16_SNORM),
        ImageFormat::Rgba16(Signed, Unnormalized) => Some(Format::R16G16B16A16_SINT),
        ImageFormat::Rgba16(Unsigned, Normalized) => Some(Format::R16G16B16A16_UNORM),
        ImageFormat::Rgba16(Unsigned, Unnormalized) => Some(Format::R16G16B16A16_UINT),
        ImageFormat::RgbaFloat16 => Some(Format::R16G16B16A16_SFLOAT),

        ImageFormat::R32(_, Normalized) => None,
        ImageFormat::R32(Signed, Unnormalized) => Some(Format::R32_SINT),
        ImageFormat::R32(Unsigned, Unnormalized) => Some(Format::R32_UINT),
        ImageFormat::RFloat32 => Some(Format::R32_SFLOAT),
        ImageFormat::Rg32(_, Normalized) => None,
        ImageFormat::Rg32(Signed, Unnormalized) => Some(Format::R32G32_SINT),
        ImageFormat::Rg32(Unsigned, Unnormalized) => Some(Format::R32G32_UINT),
        ImageFormat::RgFloat32 => Some(Format::R32G32_SFLOAT),
        ImageFormat::Rgba32(_, Normalized) => None,
        ImageFormat::Rgba32(Signed, Unnormalized) => Some(Format::R32G32B32A32_SINT),
        ImageFormat::Rgba32(Unsigned, Unnormalized) => Some(Format::R32G32B32A32_UINT),
        ImageFormat::RgbaFloat32 => Some(Format::R32G32B32A32_SFLOAT),

        ImageFormat::Depth16 => Some(Format::D16_UNORM),
        ImageFormat::Depth24 => Some(Format::X8_D24_UNORM_PACK32),
        ImageFormat::Depth24Stencil8 => Some(Format::D24_UNORM_S8_UINT),
        ImageFormat::DepthFloat32 => Some(Format::D32_SFLOAT),
        ImageFormat::DepthFloat32Stencil8 => Some(Format::D32_SFLOAT_S8_UINT),
    }
}

pub fn reverse_translate_image_format(format: vk::Format) -> Option<ImageFormat> {
    match format {
        Format::R8_SNORM => Some(ImageFormat::R8(Signed, Normalized)),
        Format::R8_SINT => Some(ImageFormat::R8(Signed, Unnormalized)),
        Format::R8_UNORM => Some(ImageFormat::R8(Unsigned, Normalized)),
        Format::R8_UINT => Some(ImageFormat::R8(Unsigned, Unnormalized)),
        Format::R8_SRGB => Some(ImageFormat::SrgbR8),
        Format::R8G8_SNORM => Some(ImageFormat::Rg8(Signed, Normalized)),
        Format::R8G8_SINT => Some(ImageFormat::Rg8(Signed, Unnormalized)),
        Format::R8G8_UNORM => Some(ImageFormat::Rg8(Unsigned, Normalized)),
        Format::R8G8_UINT => Some(ImageFormat::Rg8(Unsigned, Unnormalized)),
        Format::R8G8_SRGB => Some(ImageFormat::SrgbRg8),
        Format::R8G8B8A8_SNORM => Some(ImageFormat::Rgba8(Signed, Normalized)),
        Format::R8G8B8A8_SINT => Some(ImageFormat::Rgba8(Signed, Unnormalized)),
        Format::R8G8B8A8_UNORM => Some(ImageFormat::Rgba8(Unsigned, Normalized)),
        Format::R8G8B8A8_UINT => Some(ImageFormat::Rgba8(Unsigned, Unnormalized)),
        Format::R8G8B8A8_SRGB => Some(ImageFormat::SrgbRgba8),

        Format::B8G8R8A8_SNORM => Some(ImageFormat::Bgra8(Signed, Normalized)),
        Format::B8G8R8A8_SINT => Some(ImageFormat::Bgra8(Signed, Unnormalized)),
        Format::B8G8R8A8_UNORM => Some(ImageFormat::Bgra8(Unsigned, Normalized)),
        Format::B8G8R8A8_UINT => Some(ImageFormat::Bgra8(Unsigned, Unnormalized)),
        Format::B8G8R8A8_SRGB => Some(ImageFormat::SrgbBgra8),

        Format::A2B10G10R10_SNORM_PACK32 => Some(ImageFormat::Rgb10A2(Signed, Normalized)),
        Format::A2B10G10R10_SINT_PACK32 => Some(ImageFormat::Rgb10A2(Signed, Unnormalized)),
        Format::A2B10G10R10_UNORM_PACK32 => Some(ImageFormat::Rgb10A2(Unsigned, Normalized)),
        Format::A2B10G10R10_UINT_PACK32 => Some(ImageFormat::Rgb10A2(Unsigned, Unnormalized)),

        Format::R16_SNORM => Some(ImageFormat::R16(Signed, Normalized)),
        Format::R16_SINT => Some(ImageFormat::R16(Signed, Unnormalized)),
        Format::R16_UNORM => Some(ImageFormat::R16(Unsigned, Normalized)),
        Format::R16_UINT => Some(ImageFormat::R16(Unsigned, Unnormalized)),
        Format::R16_SFLOAT => Some(ImageFormat::RFloat16),
        Format::R16G16_SNORM => Some(ImageFormat::Rg16(Signed, Normalized)),
        Format::R16G16_SINT => Some(ImageFormat::Rg16(Signed, Unnormalized)),
        Format::R16G16_UNORM => Some(ImageFormat::Rg16(Unsigned, Normalized)),
        Format::R16G16_UINT => Some(ImageFormat::Rg16(Unsigned, Unnormalized)),
        Format::R16G16_SFLOAT => Some(ImageFormat::RgFloat16),
        Format::R16G16B16A16_SNORM => Some(ImageFormat::Rgba16(Signed, Normalized)),
        Format::R16G16B16A16_SINT => Some(ImageFormat::Rgba16(Signed, Unnormalized)),
        Format::R16G16B16A16_UNORM => Some(ImageFormat::Rgba16(Unsigned, Normalized)),
        Format::R16G16B16A16_UINT => Some(ImageFormat::Rgba16(Unsigned, Unnormalized)),
        Format::R16G16B16A16_SFLOAT => Some(ImageFormat::RgbaFloat16),

        Format::R32_SINT => Some(ImageFormat::R32(Signed, Unnormalized)),
        Format::R32_UINT => Some(ImageFormat::R32(Unsigned, Unnormalized)),
        Format::R32_SFLOAT => Some(ImageFormat::RFloat32),
        Format::R32G32_SINT => Some(ImageFormat::Rg32(Signed, Unnormalized)),
        Format::R32G32_UINT => Some(ImageFormat::Rg32(Unsigned, Unnormalized)),
        Format::R32G32_SFLOAT => Some(ImageFormat::RgFloat32),
        Format::R32G32B32A32_SINT => Some(ImageFormat::Rgba32(Signed, Unnormalized)),
        Format::R32G32B32A32_UINT => Some(ImageFormat::Rgba32(Unsigned, Unnormalized)),
        Format::R32G32B32A32_SFLOAT => Some(ImageFormat::RgbaFloat32),

        Format::D16_UNORM => Some(ImageFormat::Depth16),
        Format::X8_D24_UNORM_PACK32 => Some(ImageFormat::Depth24),
        Format::D24_UNORM_S8_UINT => Some(ImageFormat::Depth24Stencil8),
        Format::D32_SFLOAT => Some(ImageFormat::DepthFloat32),
        Format::D32_SFLOAT_S8_UINT => Some(ImageFormat::DepthFloat32Stencil8),

        _ => None,
    }
}

pub fn translate_vertex_format(format: VertexFormat) -> Option<vk::Format> {
    match format {
        VertexFormat(Scalar, I8(Signed, Normalized)) => Some(Format::R8_SNORM),
        VertexFormat(Scalar, I8(Unsigned, Normalized)) => Some(Format::R8_UNORM),
        VertexFormat(Scalar, I8(Signed, Unnormalized)) => Some(Format::R8_SINT),
        VertexFormat(Scalar, I8(Unsigned, Unnormalized)) => Some(Format::R8_UINT),

        VertexFormat(Scalar, I16(Signed, Normalized)) => Some(Format::R16_SNORM),
        VertexFormat(Scalar, I16(Unsigned, Normalized)) => Some(Format::R16_UNORM),
        VertexFormat(Scalar, I16(Signed, Unnormalized)) => Some(Format::R16_SINT),
        VertexFormat(Scalar, I16(Unsigned, Unnormalized)) => Some(Format::R16_UINT),

        VertexFormat(Scalar, I32(_, Normalized)) => None,
        VertexFormat(Scalar, I32(Signed, Unnormalized)) => Some(Format::R32_SINT),
        VertexFormat(Scalar, I32(Unsigned, Unnormalized)) => Some(Format::R32_UINT),
        VertexFormat(Scalar, F32) => Some(Format::R32_SFLOAT),

        VertexFormat(Vector2, I8(Signed, Normalized)) => Some(Format::R8G8_SNORM),
        VertexFormat(Vector2, I8(Unsigned, Normalized)) => Some(Format::R8G8_UNORM),
        VertexFormat(Vector2, I8(Signed, Unnormalized)) => Some(Format::R8G8_SINT),
        VertexFormat(Vector2, I8(Unsigned, Unnormalized)) => Some(Format::R8G8_UINT),

        VertexFormat(Vector2, I16(Signed, Normalized)) => Some(Format::R16G16_SNORM),
        VertexFormat(Vector2, I16(Unsigned, Normalized)) => Some(Format::R16G16_UNORM),
        VertexFormat(Vector2, I16(Signed, Unnormalized)) => Some(Format::R16G16_SINT),
        VertexFormat(Vector2, I16(Unsigned, Unnormalized)) => Some(Format::R16G16_UINT),

        VertexFormat(Vector2, I32(_, Normalized)) => None,
        VertexFormat(Vector2, I32(Signed, Unnormalized)) => Some(Format::R32G32_SINT),
        VertexFormat(Vector2, I32(Unsigned, Unnormalized)) => Some(Format::R32G32_UINT),
        VertexFormat(Vector2, F32) => Some(Format::R32G32_SFLOAT),

        VertexFormat(Vector3, I8(Signed, Normalized)) => Some(Format::R8G8B8_SNORM),
        VertexFormat(Vector3, I8(Unsigned, Normalized)) => Some(Format::R8G8B8_UNORM),
        VertexFormat(Vector3, I8(Signed, Unnormalized)) => Some(Format::R8G8B8_SINT),
        VertexFormat(Vector3, I8(Unsigned, Unnormalized)) => Some(Format::R8G8B8_UINT),

        VertexFormat(Vector3, I16(Signed, Normalized)) => Some(Format::R16G16B16_SNORM),
        VertexFormat(Vector3, I16(Unsigned, Normalized)) => Some(Format::R16G16B16_UNORM),
        VertexFormat(Vector3, I16(Signed, Unnormalized)) => Some(Format::R16G16B16_SINT),
        VertexFormat(Vector3, I16(Unsigned, Unnormalized)) => Some(Format::R16G16B16_UINT),

        VertexFormat(Vector3, I32(_, Normalized)) => None,
        VertexFormat(Vector3, I32(Signed, Unnormalized)) => Some(Format::R32G32B32_SINT),
        VertexFormat(Vector3, I32(Unsigned, Unnormalized)) => Some(Format::R32G32B32_UINT),
        VertexFormat(Vector3, F32) => Some(Format::R32G32B32_SFLOAT),

        VertexFormat(Vector4, I8(Signed, Normalized)) => Some(Format::R8G8B8A8_SNORM),
        VertexFormat(Vector4, I8(Unsigned, Normalized)) => Some(Format::R8G8B8A8_UNORM),
        VertexFormat(Vector4, I8(Signed, Unnormalized)) => Some(Format::R8G8B8A8_SINT),
        VertexFormat(Vector4, I8(Unsigned, Unnormalized)) => Some(Format::R8G8B8A8_UINT),

        VertexFormat(Vector4, I16(Signed, Normalized)) => Some(Format::R16G16B16A16_SNORM),
        VertexFormat(Vector4, I16(Unsigned, Normalized)) => Some(Format::R16G16B16A16_UNORM),
        VertexFormat(Vector4, I16(Signed, Unnormalized)) => Some(Format::R16G16B16A16_SINT),
        VertexFormat(Vector4, I16(Unsigned, Unnormalized)) => Some(Format::R16G16B16A16_UINT),

        VertexFormat(Vector4, I32(_, Normalized)) => None,
        VertexFormat(Vector4, I32(Signed, Unnormalized)) => Some(Format::R32G32B32A32_SINT),
        VertexFormat(Vector4, I32(Unsigned, Unnormalized)) => Some(Format::R32G32B32A32_UINT),
        VertexFormat(Vector4, F32) => Some(Format::R32G32B32A32_SFLOAT),
    }
}
