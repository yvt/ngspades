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
use metal::{MTLPixelFormat, MTLVertexFormat};

pub fn translate_image_format(format: ImageFormat) -> Option<MTLPixelFormat> {
    match format {
        ImageFormat::R8(Signed, Normalized) => Some(MTLPixelFormat::R8Snorm),
        ImageFormat::R8(Signed, Unnormalized) => Some(MTLPixelFormat::R8Sint),
        ImageFormat::R8(Unsigned, Normalized) => Some(MTLPixelFormat::R8Unorm),
        ImageFormat::R8(Unsigned, Unnormalized) => Some(MTLPixelFormat::R8Uint),
        ImageFormat::SrgbR8 => None,
        ImageFormat::Rg8(Signed, Normalized) => Some(MTLPixelFormat::RG8Snorm),
        ImageFormat::Rg8(Signed, Unnormalized) => Some(MTLPixelFormat::RG8Sint),
        ImageFormat::Rg8(Unsigned, Normalized) => Some(MTLPixelFormat::RG8Unorm),
        ImageFormat::Rg8(Unsigned, Unnormalized) => Some(MTLPixelFormat::RG8Uint),
        ImageFormat::SrgbRg8 => None,
        ImageFormat::Rgba8(Signed, Normalized) => Some(MTLPixelFormat::RGBA8Snorm),
        ImageFormat::Rgba8(Signed, Unnormalized) => Some(MTLPixelFormat::RGBA8Sint),
        ImageFormat::Rgba8(Unsigned, Normalized) => Some(MTLPixelFormat::RGBA8Unorm),
        ImageFormat::Rgba8(Unsigned, Unnormalized) => Some(MTLPixelFormat::RGBA8Uint),
        ImageFormat::SrgbRgba8 => Some(MTLPixelFormat::RGBA8Unorm_sRGB),

        ImageFormat::Bgra8(Signed, _) => None,
        ImageFormat::Bgra8(Unsigned, Normalized) => Some(MTLPixelFormat::BGRA8Unorm),
        ImageFormat::Bgra8(Unsigned, Unnormalized) => None,
        ImageFormat::SrgbBgra8 => Some(MTLPixelFormat::BGRA8Unorm_sRGB),

        ImageFormat::Rgb10A2(Signed, _) => None,
        ImageFormat::Rgb10A2(Unsigned, Normalized) => Some(MTLPixelFormat::RGB10A2Unorm),
        ImageFormat::Rgb10A2(Unsigned, Unnormalized) => Some(MTLPixelFormat::RGB10A2Uint),

        ImageFormat::R16(Signed, Normalized) => Some(MTLPixelFormat::R16Snorm),
        ImageFormat::R16(Signed, Unnormalized) => Some(MTLPixelFormat::R16Sint),
        ImageFormat::R16(Unsigned, Normalized) => Some(MTLPixelFormat::R16Unorm),
        ImageFormat::R16(Unsigned, Unnormalized) => Some(MTLPixelFormat::R16Uint),
        ImageFormat::RFloat16 => Some(MTLPixelFormat::R16Float),
        ImageFormat::Rg16(Signed, Normalized) => Some(MTLPixelFormat::RG16Snorm),
        ImageFormat::Rg16(Signed, Unnormalized) => Some(MTLPixelFormat::RG16Sint),
        ImageFormat::Rg16(Unsigned, Normalized) => Some(MTLPixelFormat::RG16Unorm),
        ImageFormat::Rg16(Unsigned, Unnormalized) => Some(MTLPixelFormat::RG16Uint),
        ImageFormat::RgFloat16 => Some(MTLPixelFormat::RG16Float),
        ImageFormat::Rgba16(Signed, Normalized) => Some(MTLPixelFormat::RGBA16Snorm),
        ImageFormat::Rgba16(Signed, Unnormalized) => Some(MTLPixelFormat::RGBA16Sint),
        ImageFormat::Rgba16(Unsigned, Normalized) => Some(MTLPixelFormat::RGBA16Unorm),
        ImageFormat::Rgba16(Unsigned, Unnormalized) => Some(MTLPixelFormat::RGBA16Uint),
        ImageFormat::RgbaFloat16 => Some(MTLPixelFormat::RGBA16Float),

        ImageFormat::R32(Signed, Normalized) => None,
        ImageFormat::R32(Signed, Unnormalized) => Some(MTLPixelFormat::R32Sint),
        ImageFormat::R32(Unsigned, Normalized) => None,
        ImageFormat::R32(Unsigned, Unnormalized) => Some(MTLPixelFormat::R32Uint),
        ImageFormat::RFloat32 => Some(MTLPixelFormat::R32Float),
        ImageFormat::Rg32(Signed, Normalized) => None,
        ImageFormat::Rg32(Signed, Unnormalized) => Some(MTLPixelFormat::RG32Sint),
        ImageFormat::Rg32(Unsigned, Normalized) => None,
        ImageFormat::Rg32(Unsigned, Unnormalized) => Some(MTLPixelFormat::RG32Uint),
        ImageFormat::RgFloat32 => Some(MTLPixelFormat::RG32Float),
        ImageFormat::Rgba32(Signed, Normalized) => None,
        ImageFormat::Rgba32(Signed, Unnormalized) => Some(MTLPixelFormat::RGBA32Sint),
        ImageFormat::Rgba32(Unsigned, Normalized) => None,
        ImageFormat::Rgba32(Unsigned, Unnormalized) => Some(MTLPixelFormat::RGBA32Uint),
        ImageFormat::RgbaFloat32 => Some(MTLPixelFormat::RGBA32Float),

        ImageFormat::Depth16 => Some(MTLPixelFormat::Depth16Unorm),
        ImageFormat::Depth24 => None,
        ImageFormat::Depth24Stencil8 => Some(MTLPixelFormat::Depth24Unorm_Stencil8),
        ImageFormat::DepthFloat32 => Some(MTLPixelFormat::Depth32Float),
        ImageFormat::DepthFloat32Stencil8 => Some(MTLPixelFormat::Depth32Float_Stencil8),
    }
}

pub fn translate_vertex_format(format: VertexFormat) -> Option<MTLVertexFormat> {
    match format {
        VertexFormat(_, I32(_, Normalized)) => None,
        VertexFormat(Scalar, I8(_, _)) => None,
        VertexFormat(Scalar, I16(_, _)) => None,

        VertexFormat(Scalar, I32(Signed, Unnormalized)) => Some(MTLVertexFormat::Int),
        VertexFormat(Scalar, I32(Unsigned, Unnormalized)) => Some(MTLVertexFormat::UInt),
        VertexFormat(Scalar, F32) => Some(MTLVertexFormat::Float),

        VertexFormat(Vector2, I8(Signed, Normalized)) => Some(MTLVertexFormat::Char2Normalized),
        VertexFormat(Vector2, I8(Unsigned, Normalized)) => Some(MTLVertexFormat::UChar2Normalized),
        VertexFormat(Vector2, I8(Signed, Unnormalized)) => Some(MTLVertexFormat::Char2),
        VertexFormat(Vector2, I8(Unsigned, Unnormalized)) => Some(MTLVertexFormat::UChar2),

        VertexFormat(Vector2, I16(Signed, Normalized)) => Some(MTLVertexFormat::Short2Normalized),
        VertexFormat(Vector2, I16(Unsigned, Normalized)) => Some(MTLVertexFormat::UShort2Normalized),
        VertexFormat(Vector2, I16(Signed, Unnormalized)) => Some(MTLVertexFormat::Short2),
        VertexFormat(Vector2, I16(Unsigned, Unnormalized)) => Some(MTLVertexFormat::UShort2),

        VertexFormat(Vector2, I32(Signed, Unnormalized)) => Some(MTLVertexFormat::Int2),
        VertexFormat(Vector2, I32(Unsigned, Unnormalized)) => Some(MTLVertexFormat::UInt2),

        VertexFormat(Vector2, F32) => Some(MTLVertexFormat::Float2),

        VertexFormat(Vector3, I8(Signed, Normalized)) => Some(MTLVertexFormat::Char3Normalized),
        VertexFormat(Vector3, I8(Unsigned, Normalized)) => Some(MTLVertexFormat::UChar3Normalized),
        VertexFormat(Vector3, I8(Signed, Unnormalized)) => Some(MTLVertexFormat::Char3),
        VertexFormat(Vector3, I8(Unsigned, Unnormalized)) => Some(MTLVertexFormat::UChar3),

        VertexFormat(Vector3, I16(Signed, Normalized)) => Some(MTLVertexFormat::Short3Normalized),
        VertexFormat(Vector3, I16(Unsigned, Normalized)) => Some(MTLVertexFormat::UShort3Normalized),
        VertexFormat(Vector3, I16(Signed, Unnormalized)) => Some(MTLVertexFormat::Short3),
        VertexFormat(Vector3, I16(Unsigned, Unnormalized)) => Some(MTLVertexFormat::UShort3),

        VertexFormat(Vector3, I32(Signed, Unnormalized)) => Some(MTLVertexFormat::Int3),
        VertexFormat(Vector3, I32(Unsigned, Unnormalized)) => Some(MTLVertexFormat::UInt3),

        VertexFormat(Vector3, F32) => Some(MTLVertexFormat::Float3),

        VertexFormat(Vector4, I8(Signed, Normalized)) => Some(MTLVertexFormat::Char4Normalized),
        VertexFormat(Vector4, I8(Unsigned, Normalized)) => Some(MTLVertexFormat::UChar4Normalized),
        VertexFormat(Vector4, I8(Signed, Unnormalized)) => Some(MTLVertexFormat::Char4),
        VertexFormat(Vector4, I8(Unsigned, Unnormalized)) => Some(MTLVertexFormat::UChar4),

        VertexFormat(Vector4, I16(Signed, Normalized)) => Some(MTLVertexFormat::Short4Normalized),
        VertexFormat(Vector4, I16(Unsigned, Normalized)) => Some(MTLVertexFormat::UShort4Normalized),
        VertexFormat(Vector4, I16(Signed, Unnormalized)) => Some(MTLVertexFormat::Short4),
        VertexFormat(Vector4, I16(Unsigned, Unnormalized)) => Some(MTLVertexFormat::UShort4),

        VertexFormat(Vector4, I32(Signed, Unnormalized)) => Some(MTLVertexFormat::Int4),
        VertexFormat(Vector4, I32(Unsigned, Unnormalized)) => Some(MTLVertexFormat::UInt4),

        VertexFormat(Vector4, F32) => Some(MTLVertexFormat::Float4),
    }
}
