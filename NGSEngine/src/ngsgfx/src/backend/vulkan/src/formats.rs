//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core::{ImageFormat, Signedness, Normalizedness, VertexFormat, VectorWidth, ScalarFormat};
use self::Signedness::{Signed, Unsigned};
use self::Normalizedness::{Normalized, Unnormalized};
use self::VectorWidth::{Scalar, Vector2, Vector3, Vector4};
use self::ScalarFormat::{I8, I16, I32, F32};
use ash::vk::{self, Format};

pub fn translate_image_format(format: ImageFormat) -> Option<vk::Format> {
    match format {
        ImageFormat::R8(Signed, Normalized) => Some(Format::R8Snorm),
        ImageFormat::R8(Signed, Unnormalized) => Some(Format::R8Sint),
        ImageFormat::R8(Unsigned, Normalized) => Some(Format::R8Unorm),
        ImageFormat::R8(Unsigned, Unnormalized) => Some(Format::R8Uint),
        ImageFormat::SrgbR8 => Some(Format::R8Srgb),
        ImageFormat::Rg8(Signed, Normalized) => Some(Format::R8g8Snorm),
        ImageFormat::Rg8(Signed, Unnormalized) => Some(Format::R8g8Sint),
        ImageFormat::Rg8(Unsigned, Normalized) => Some(Format::R8g8Unorm),
        ImageFormat::Rg8(Unsigned, Unnormalized) => Some(Format::R8g8Uint),
        ImageFormat::SrgbRg8 => Some(Format::R8g8Srgb),
        ImageFormat::Rgba8(Signed, Normalized) => Some(Format::R8g8b8a8Snorm),
        ImageFormat::Rgba8(Signed, Unnormalized) => Some(Format::R8g8b8a8Sint),
        ImageFormat::Rgba8(Unsigned, Normalized) => Some(Format::R8g8b8a8Unorm),
        ImageFormat::Rgba8(Unsigned, Unnormalized) => Some(Format::R8g8b8a8Uint),
        ImageFormat::SrgbRgba8 => Some(Format::R8g8b8a8Srgb),

        ImageFormat::Bgra8(Signed, Normalized) => Some(Format::B8g8r8a8Snorm),
        ImageFormat::Bgra8(Signed, Unnormalized) => Some(Format::B8g8r8a8Sint),
        ImageFormat::Bgra8(Unsigned, Normalized) => Some(Format::B8g8r8a8Unorm),
        ImageFormat::Bgra8(Unsigned, Unnormalized) => Some(Format::B8g8r8a8Uint),
        ImageFormat::SrgbBgra8 => Some(Format::B8g8r8a8Srgb),

        ImageFormat::Rgb10A2(Signed, Normalized) => Some(Format::A2b10g10r10SnormPack32),
        ImageFormat::Rgb10A2(Signed, Unnormalized) => Some(Format::A2b10g10r10SintPack32),
        ImageFormat::Rgb10A2(Unsigned, Normalized) => Some(Format::A2b10g10r10UnormPack32),
        ImageFormat::Rgb10A2(Unsigned, Unnormalized) => Some(Format::A2b10g10r10UintPack32),

        ImageFormat::R16(Signed, Normalized) => Some(Format::R16Snorm),
        ImageFormat::R16(Signed, Unnormalized) => Some(Format::R16Sint),
        ImageFormat::R16(Unsigned, Normalized) => Some(Format::R16Unorm),
        ImageFormat::R16(Unsigned, Unnormalized) => Some(Format::R16Uint),
        ImageFormat::RFloat16 => Some(Format::R16Sfloat),
        ImageFormat::Rg16(Signed, Normalized) => Some(Format::R16g16Snorm),
        ImageFormat::Rg16(Signed, Unnormalized) => Some(Format::R16g16Sint),
        ImageFormat::Rg16(Unsigned, Normalized) => Some(Format::R16g16Unorm),
        ImageFormat::Rg16(Unsigned, Unnormalized) => Some(Format::R16g16Uint),
        ImageFormat::RgFloat16 => Some(Format::R16g16Sfloat),
        ImageFormat::Rgba16(Signed, Normalized) => Some(Format::R16g16b16a16Snorm),
        ImageFormat::Rgba16(Signed, Unnormalized) => Some(Format::R16g16b16a16Sint),
        ImageFormat::Rgba16(Unsigned, Normalized) => Some(Format::R16g16b16a16Unorm),
        ImageFormat::Rgba16(Unsigned, Unnormalized) => Some(Format::R16g16b16a16Uint),
        ImageFormat::RgbaFloat16 => Some(Format::R16g16b16a16Sfloat),

        ImageFormat::R32(_, Normalized) => None,
        ImageFormat::R32(Signed, Unnormalized) => Some(Format::R32Sint),
        ImageFormat::R32(Unsigned, Unnormalized) => Some(Format::R32Uint),
        ImageFormat::RFloat32 => Some(Format::R32Sfloat),
        ImageFormat::Rg32(_, Normalized) => None,
        ImageFormat::Rg32(Signed, Unnormalized) => Some(Format::R32g32Sint),
        ImageFormat::Rg32(Unsigned, Unnormalized) => Some(Format::R32g32Uint),
        ImageFormat::RgFloat32 => Some(Format::R32g32Sfloat),
        ImageFormat::Rgba32(_, Normalized) => None,
        ImageFormat::Rgba32(Signed, Unnormalized) => Some(Format::R32g32b32a32Sint),
        ImageFormat::Rgba32(Unsigned, Unnormalized) => Some(Format::R32g32b32a32Uint),
        ImageFormat::RgbaFloat32 => Some(Format::R32g32b32a32Sfloat),

        ImageFormat::Depth16 => Some(Format::D16Unorm),
        ImageFormat::Depth24 => Some(Format::X8D24UnormPack32),
        ImageFormat::Depth24Stencil8 => Some(Format::D24UnormS8Uint),
        ImageFormat::DepthFloat32 => Some(Format::D32Sfloat),
        ImageFormat::DepthFloat32Stencil8 => Some(Format::D32SfloatS8Uint),
    }
}

pub fn reverse_translate_image_format(format: vk::Format) -> Option<ImageFormat> {
    match format {
        Format::R8Snorm => Some(ImageFormat::R8(Signed, Normalized)),
        Format::R8Sint => Some(ImageFormat::R8(Signed, Unnormalized)),
        Format::R8Unorm => Some(ImageFormat::R8(Unsigned, Normalized)),
        Format::R8Uint => Some(ImageFormat::R8(Unsigned, Unnormalized)),
        Format::R8Srgb => Some(ImageFormat::SrgbR8),
        Format::R8g8Snorm => Some(ImageFormat::Rg8(Signed, Normalized)),
        Format::R8g8Sint => Some(ImageFormat::Rg8(Signed, Unnormalized)),
        Format::R8g8Unorm => Some(ImageFormat::Rg8(Unsigned, Normalized)),
        Format::R8g8Uint => Some(ImageFormat::Rg8(Unsigned, Unnormalized)),
        Format::R8g8Srgb => Some(ImageFormat::SrgbRg8),
        Format::R8g8b8a8Snorm => Some(ImageFormat::Rgba8(Signed, Normalized)),
        Format::R8g8b8a8Sint => Some(ImageFormat::Rgba8(Signed, Unnormalized)),
        Format::R8g8b8a8Unorm => Some(ImageFormat::Rgba8(Unsigned, Normalized)),
        Format::R8g8b8a8Uint => Some(ImageFormat::Rgba8(Unsigned, Unnormalized)),
        Format::R8g8b8a8Srgb => Some(ImageFormat::SrgbRgba8),

        Format::B8g8r8a8Snorm => Some(ImageFormat::Bgra8(Signed, Normalized)),
        Format::B8g8r8a8Sint => Some(ImageFormat::Bgra8(Signed, Unnormalized)),
        Format::B8g8r8a8Unorm => Some(ImageFormat::Bgra8(Unsigned, Normalized)),
        Format::B8g8r8a8Uint => Some(ImageFormat::Bgra8(Unsigned, Unnormalized)),
        Format::B8g8r8a8Srgb => Some(ImageFormat::SrgbBgra8),

        Format::A2b10g10r10SnormPack32 => Some(ImageFormat::Rgb10A2(Signed, Normalized)),
        Format::A2b10g10r10SintPack32 => Some(ImageFormat::Rgb10A2(Signed, Unnormalized)),
        Format::A2b10g10r10UnormPack32 => Some(ImageFormat::Rgb10A2(Unsigned, Normalized)),
        Format::A2b10g10r10UintPack32 => Some(ImageFormat::Rgb10A2(Unsigned, Unnormalized)),

        Format::R16Snorm => Some(ImageFormat::R16(Signed, Normalized)),
        Format::R16Sint => Some(ImageFormat::R16(Signed, Unnormalized)),
        Format::R16Unorm => Some(ImageFormat::R16(Unsigned, Normalized)),
        Format::R16Uint => Some(ImageFormat::R16(Unsigned, Unnormalized)),
        Format::R16Sfloat => Some(ImageFormat::RFloat16),
        Format::R16g16Snorm => Some(ImageFormat::Rg16(Signed, Normalized)),
        Format::R16g16Sint => Some(ImageFormat::Rg16(Signed, Unnormalized)),
        Format::R16g16Unorm => Some(ImageFormat::Rg16(Unsigned, Normalized)),
        Format::R16g16Uint => Some(ImageFormat::Rg16(Unsigned, Unnormalized)),
        Format::R16g16Sfloat => Some(ImageFormat::RgFloat16),
        Format::R16g16b16a16Snorm => Some(ImageFormat::Rgba16(Signed, Normalized)),
        Format::R16g16b16a16Sint => Some(ImageFormat::Rgba16(Signed, Unnormalized)),
        Format::R16g16b16a16Unorm => Some(ImageFormat::Rgba16(Unsigned, Normalized)),
        Format::R16g16b16a16Uint => Some(ImageFormat::Rgba16(Unsigned, Unnormalized)),
        Format::R16g16b16a16Sfloat => Some(ImageFormat::RgbaFloat16),

        Format::R32Sint => Some(ImageFormat::R32(Signed, Unnormalized)),
        Format::R32Uint => Some(ImageFormat::R32(Unsigned, Unnormalized)),
        Format::R32Sfloat => Some(ImageFormat::RFloat32),
        Format::R32g32Sint => Some(ImageFormat::Rg32(Signed, Unnormalized)),
        Format::R32g32Uint => Some(ImageFormat::Rg32(Unsigned, Unnormalized)),
        Format::R32g32Sfloat => Some(ImageFormat::RgFloat32),
        Format::R32g32b32a32Sint => Some(ImageFormat::Rgba32(Signed, Unnormalized)),
        Format::R32g32b32a32Uint => Some(ImageFormat::Rgba32(Unsigned, Unnormalized)),
        Format::R32g32b32a32Sfloat => Some(ImageFormat::RgbaFloat32),

        Format::D16Unorm => Some(ImageFormat::Depth16),
        Format::X8D24UnormPack32 => Some(ImageFormat::Depth24),
        Format::D24UnormS8Uint => Some(ImageFormat::Depth24Stencil8),
        Format::D32Sfloat => Some(ImageFormat::DepthFloat32),
        Format::D32SfloatS8Uint => Some(ImageFormat::DepthFloat32Stencil8),

        _ => None,
    }
}

pub fn translate_vertex_format(format: VertexFormat) -> Option<vk::Format> {
    match format {
        VertexFormat(Scalar, I8(Signed, Normalized)) => Some(Format::R8Snorm),
        VertexFormat(Scalar, I8(Unsigned, Normalized)) => Some(Format::R8Unorm),
        VertexFormat(Scalar, I8(Signed, Unnormalized)) => Some(Format::R8Sint),
        VertexFormat(Scalar, I8(Unsigned, Unnormalized)) => Some(Format::R8Uint),

        VertexFormat(Scalar, I16(Signed, Normalized)) => Some(Format::R16Snorm),
        VertexFormat(Scalar, I16(Unsigned, Normalized)) => Some(Format::R16Unorm),
        VertexFormat(Scalar, I16(Signed, Unnormalized)) => Some(Format::R16Sint),
        VertexFormat(Scalar, I16(Unsigned, Unnormalized)) => Some(Format::R16Uint),

        VertexFormat(Scalar, I32(_, Normalized)) => None,
        VertexFormat(Scalar, I32(Signed, Unnormalized)) => Some(Format::R32Sint),
        VertexFormat(Scalar, I32(Unsigned, Unnormalized)) => Some(Format::R32Uint),
        VertexFormat(Scalar, F32) => Some(Format::R32Sfloat),

        VertexFormat(Vector2, I8(Signed, Normalized)) => Some(Format::R8g8Snorm),
        VertexFormat(Vector2, I8(Unsigned, Normalized)) => Some(Format::R8g8Unorm),
        VertexFormat(Vector2, I8(Signed, Unnormalized)) => Some(Format::R8g8Sint),
        VertexFormat(Vector2, I8(Unsigned, Unnormalized)) => Some(Format::R8g8Uint),

        VertexFormat(Vector2, I16(Signed, Normalized)) => Some(Format::R16g16Snorm),
        VertexFormat(Vector2, I16(Unsigned, Normalized)) => Some(Format::R16g16Unorm),
        VertexFormat(Vector2, I16(Signed, Unnormalized)) => Some(Format::R16g16Sint),
        VertexFormat(Vector2, I16(Unsigned, Unnormalized)) => Some(Format::R16g16Uint),

        VertexFormat(Vector2, I32(_, Normalized)) => None,
        VertexFormat(Vector2, I32(Signed, Unnormalized)) => Some(Format::R32g32Sint),
        VertexFormat(Vector2, I32(Unsigned, Unnormalized)) => Some(Format::R32g32Uint),
        VertexFormat(Vector2, F32) => Some(Format::R32g32Sfloat),

        VertexFormat(Vector3, I8(Signed, Normalized)) => Some(Format::R8g8b8Snorm),
        VertexFormat(Vector3, I8(Unsigned, Normalized)) => Some(Format::R8g8b8Unorm),
        VertexFormat(Vector3, I8(Signed, Unnormalized)) => Some(Format::R8g8b8Sint),
        VertexFormat(Vector3, I8(Unsigned, Unnormalized)) => Some(Format::R8g8b8Uint),

        VertexFormat(Vector3, I16(Signed, Normalized)) => Some(Format::R16g16b16Snorm),
        VertexFormat(Vector3, I16(Unsigned, Normalized)) => Some(Format::R16g16b16Unorm),
        VertexFormat(Vector3, I16(Signed, Unnormalized)) => Some(Format::R16g16b16Sint),
        VertexFormat(Vector3, I16(Unsigned, Unnormalized)) => Some(Format::R16g16b16Uint),

        VertexFormat(Vector3, I32(_, Normalized)) => None,
        VertexFormat(Vector3, I32(Signed, Unnormalized)) => Some(Format::R32g32b32Sint),
        VertexFormat(Vector3, I32(Unsigned, Unnormalized)) => Some(Format::R32g32b32Uint),
        VertexFormat(Vector3, F32) => Some(Format::R32g32b32Sfloat),

        VertexFormat(Vector4, I8(Signed, Normalized)) => Some(Format::R8g8b8a8Snorm),
        VertexFormat(Vector4, I8(Unsigned, Normalized)) => Some(Format::R8g8b8a8Unorm),
        VertexFormat(Vector4, I8(Signed, Unnormalized)) => Some(Format::R8g8b8a8Sint),
        VertexFormat(Vector4, I8(Unsigned, Unnormalized)) => Some(Format::R8g8b8a8Uint),

        VertexFormat(Vector4, I16(Signed, Normalized)) => Some(Format::R16g16b16a16Snorm),
        VertexFormat(Vector4, I16(Unsigned, Normalized)) => Some(Format::R16g16b16a16Unorm),
        VertexFormat(Vector4, I16(Signed, Unnormalized)) => Some(Format::R16g16b16a16Sint),
        VertexFormat(Vector4, I16(Unsigned, Unnormalized)) => Some(Format::R16g16b16a16Uint),

        VertexFormat(Vector4, I32(_, Normalized)) => None,
        VertexFormat(Vector4, I32(Signed, Unnormalized)) => Some(Format::R32g32b32a32Sint),
        VertexFormat(Vector4, I32(Unsigned, Unnormalized)) => Some(Format::R32g32b32a32Uint),
        VertexFormat(Vector4, F32) => Some(Format::R32g32b32a32Sfloat),
    }
}
