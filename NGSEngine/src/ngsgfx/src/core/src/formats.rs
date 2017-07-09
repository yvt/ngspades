//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Defines various format types.
//!
//! Supported formats differ depending on the backend and the hardware.
//!
//! - **Supported** means the backend exposes the hardware's capability to use the format in some way.
//! - **Mandatory** means the format is always available if it's supported by the backend.
//! - **Undefined** means some operations cannot be defined on the format in a meaningful way.
//!     - Filtering, blending, and MSAA resolve operation are undefined on all unnormalized formats.
//!     - Depth/stencil operations are undefined on all non-depth/stencil formats.
//!     - Color attachment operations are undeifned on all depth/stencil formats.
//!

/// Image format.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ImageFormat {
    /// Represents a pixel format with a 8-bit red channel.
    ///
    /// Mandatory.
    R8(Signedness, Normalizedness),

    /// Represents a pixel format with a 8-bit red channel in the sRGB encoding.
    ///
    /// Not mandatory.
    SrgbR8,

    /// Represents a pixel format with a 8-bit red/green channels.
    ///
    /// Mandatory.
    Rg8(Signedness, Normalizedness),

    /// Represents a pixel format with a 8-bit red/green channels in the sRGB encoding.
    ///
    /// Not mandatory.
    SrgbRg8,

    /// Represents a pixel format with a 8-bit red/green/blue/alpha channels.
    ///
    /// Not mandatory.
    Rgba8(Signedness, Normalizedness),

    /// Represents a pixel format with a 8-bit red/green/blue/alpha channels in the sRGB encoding.
    ///
    /// Mandatory.
    SrgbRgba8,

    /// Represents a pixel format with a 10-bit red/green/blue and 2-bit alpha channels.
    ///
    /// Unsigned variations are mandatory.
    Rgb10A2(Signedness, Normalizedness),

    /// Represents a pixel format with a 16-bit red channel.
    ///
    /// Mandatory.
    R16(Signedness, Normalizedness),

    /// Represents a pixel format with a 16-bit floating point red channel.
    ///
    /// TODO: make this mandatory if required by Vulkan.
    RFloat16,

    /// Represents a pixel format with a 16-bit red/green channels.
    ///
    /// Not mandatory.
    Rg16(Signedness, Normalizedness),

    /// Represents a pixel format with a 16-bit floating point red/green channels.
    ///
    /// TODO: make this mandatory if required by Vulkan.
    RgFloat16,

    /// Represents a pixel format with a 16-bit red/green/blue/alpha channels.
    ///
    /// Unnormalized variations are mandatory.
    Rgba16(Signedness, Normalizedness),

    /// Represents a pixel format with a 16-bit floating point red/green/blue/alpha channels.
    ///
    /// Mandatory.
    RgbaFloat16,

    /// Represents a pixel format with a 32-bit red channel.
    ///
    /// Unnormalized variations are mandatory.
    R32(Signedness, Normalizedness),

    /// Represents a pixel format with a 32-bit floating point red channel.
    ///
    /// Mandatory.
    RFloat32,

    /// Represents a pixel format with a 32-bit red/green channels.
    ///
    /// Not mandatory.
    Rg32(Signedness, Normalizedness),

    /// Represents a pixel format with a 32-bit floating point red/green channels.
    ///
    /// Mandatory.
    RgFloat32,

    /// Represents a pixel format with a 32-bit red/green/blue/alpha channels.
    ///
    /// Unnormalized variations are mandatory.
    Rgba32(Signedness, Normalizedness),

    /// Represents a pixel format with a 32-bit floating point red/green/blue/alpha channels.
    ///
    /// Mandatory.
    RgbaFloat32,

    /// Represents a pixel format with a 32-bit red/green/blue/alpha channels in BGRA order.
    Bgra8(Signedness, Normalizedness),

    /// Represents a pixel format with a 8-bit red/green/blue/alpha channels in the sRGB encoding and
    /// in BGRA order.
    SrgbBgra8,

    /// Represents a pixel format with a 16-bit depth.
    ///
    /// Mandatory.
    ///
    /// FIXME: This format isn't supported on iOS and OS X 10.11. Should this be mandatory?
    Depth16,

    /// Represents a pixel format with a 24-bit depth.
    ///
    /// Either of this and `DepthFloat32` is mandatory.
    Depth24,

    /// Represents a pixel format with a 32-bit floating point depth.
    ///
    /// Either of this and `Depth24` is mandatory.
    DepthFloat32,

    /// Represents a pixel format with a 24-bit depth and 8-bit stencil.
    ///
    /// Either of this and `DepthFloat32Stencil8` is mandatory.
    Depth24Stencil8,

    /// Represents a pixel format with a 32-bit floating point depth and 8-bit stencil.
    ///
    /// Either of this and `Depth24Stencil8` is mandatory.
    DepthFloat32Stencil8,
}

impl ImageFormat {
    pub fn has_color(&self) -> bool {
        !self.has_depth()
    }

    pub fn is_color_float(&self) -> bool {
        match *self {
            ImageFormat::RFloat16 |
            ImageFormat::RFloat32 |
            ImageFormat::RgFloat16 |
            ImageFormat::RgFloat32 |
            ImageFormat::RgbaFloat16 |
            ImageFormat::RgbaFloat32 => true,
            _ => false,
        }
    }

    pub fn is_color_srgb(&self) -> bool {
        match *self {
            ImageFormat::SrgbR8 |
            ImageFormat::SrgbRg8 |
            ImageFormat::SrgbRgba8 |
            ImageFormat::SrgbBgra8 => true,
            _ => false,
        }
    }

    pub fn color_int_type(&self) -> Option<(Signedness, Normalizedness)> {
        match *self {
            ImageFormat::R8(signedness, normalizedness) |
            ImageFormat::Rg8(signedness, normalizedness) |
            ImageFormat::Rgba8(signedness, normalizedness) |
            ImageFormat::Rgb10A2(signedness, normalizedness) |
            ImageFormat::R16(signedness, normalizedness) |
            ImageFormat::Rg16(signedness, normalizedness) |
            ImageFormat::Rgba16(signedness, normalizedness) |
            ImageFormat::R32(signedness, normalizedness) |
            ImageFormat::Rg32(signedness, normalizedness) |
            ImageFormat::Rgba32(signedness, normalizedness) |
            ImageFormat::Bgra8(signedness, normalizedness) => Some((signedness, normalizedness)),
            _ => None,
        }
    }

    pub fn has_depth(&self) -> bool {
        match *self {
            ImageFormat::Depth16 |
            ImageFormat::Depth24 |
            ImageFormat::DepthFloat32 |
            ImageFormat::Depth24Stencil8 |
            ImageFormat::DepthFloat32Stencil8 => true,
            _ => false,
        }
    }

    pub fn has_stencil(&self) -> bool {
        match *self {
            ImageFormat::Depth24Stencil8 |
            ImageFormat::DepthFloat32Stencil8 => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ChannelSet {
    Red,
    RedGreen,
    RedGreenBlueAlpha,
    Depth,
    DepthStencil,
    Stencil,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Signedness {
    Unsigned,
    Signed,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Normalizedness {
    Unnormalized,
    Normalized,
}

/// Vertex format.
///
/// Following variants are not supported by Metal:
///
///  - `(Scalar, I8(_, _))`
///  - `(Scalar, I16(_, _))`
///  - `(_, I32(_, Normalized))`
///
/// Following variants are not supported by Vulkan:
///
///  - `(_, I32(_, Normalized))`
///
/// Following variants are not mandatory:
///
///  - `(Vector3, I8(_, _))`
///  - `(Vector3, I16(_, _))`
///  - `(_, I32(_, Normalized))`
///
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct VertexFormat(pub VectorWidth, pub ScalarFormat);

impl VertexFormat {
    pub fn width(&self) -> usize {
        self.0.width()
    }

    pub fn size(&self) -> usize {
        self.width() * self.1.size()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum VectorWidth {
    Scalar,
    Vector2,
    Vector3,
    Vector4,
}

impl VectorWidth {
    pub fn width(self) -> usize {
        match self {
            VectorWidth::Scalar => 1,
            VectorWidth::Vector2 => 2,
            VectorWidth::Vector3 => 3,
            VectorWidth::Vector4 => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ScalarFormat {
    I8(Signedness, Normalizedness),
    I16(Signedness, Normalizedness),
    I32(Signedness, Normalizedness),
    F32,
}

impl ScalarFormat {
    pub fn size(&self) -> usize {
        match *self {
            ScalarFormat::I8(_, _) => 1,
            ScalarFormat::I16(_, _) => 2,
            ScalarFormat::I32(_, _) => 4,
            ScalarFormat::F32 => 4,
        }
    }

    pub fn integer_signedness(&self) -> Option<Signedness> {
        match *self {
            ScalarFormat::I8(s, _) |
            ScalarFormat::I16(s, _) |
            ScalarFormat::I32(s, _) => Some(s),
            ScalarFormat::F32 => None,
        }
    }

    pub fn integer_normalizedness(&self) -> Option<Normalizedness> {
        match *self {
            ScalarFormat::I8(_, n) |
            ScalarFormat::I16(_, n) |
            ScalarFormat::I32(_, n) => Some(n),
            ScalarFormat::F32 => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum IndexFormat {
    U16,
    U32,
}
