//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core::{ImageFormat, Signedness, Normalizedness};
use self::Signedness::{Signed, Unsigned};
use self::Normalizedness::{Normalized, Unnormalized};
use metal::MTLPixelFormat;

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
