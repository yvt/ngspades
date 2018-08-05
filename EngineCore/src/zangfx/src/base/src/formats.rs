//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Defines various format types.
//!
//! Supported formats differ depending on the backend and the hardware.
//!
//! - **Supported** means backends can expose the hardware's capability to use the format in some way.
//! - **Mandatory** means backends are required to support the format in some way.
//! - **Undefined** means some operations cannot be defined on the format in a meaningful way.
//!     - Filtering, blending, and MSAA resolve operation are undefined on all unnormalized formats.
//!     - Depth/stencil operations are undefined on all non-depth/stencil formats.
//!     - Color attachment operations are undeifned on all depth/stencil formats.
//!
use lazy_static::lazy_static;
use std::ops;
use {itervalues::IterValues, itervalues_derive::IterValues};

/// Image format.
///
/// See [`IntAsImageFormat`] and [`FloatAsImageFormat`] for a convenient way
/// to construct certain values of `ImageFormat`.
///
/// [`IntAsImageFormat`]: IntAsImageFormat
/// [`FloatAsImageFormat`]: FloatAsImageFormat
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, IterValues)]
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

    /// Represents a pixel format with a 8-bit red/green/blue/alpha channels
    /// in the sRGB encoding.
    ///
    /// Mandatory.
    SrgbRgba8,

    /// Represents a pixel format with a 10-bit red/green/blue and 2-bit
    /// alpha channels.
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

    /// Represents a pixel format with a 32-bit red/green/blue/alpha channels
    /// in BGRA order.
    Bgra8(Signedness, Normalizedness),

    /// Represents a pixel format with a 8-bit red/green/blue/alpha channels
    /// in the sRGB encoding and in BGRA order.
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
    /// Retrieve all possible values of `ImageFormat`.
    pub fn values() -> &'static [ImageFormat] {
        lazy_static! {
            static ref VALUES: Vec<ImageFormat> = ImageFormat::iter_values().collect();
        }
        &VALUES
    }

    pub fn has_color(&self) -> bool {
        !self.has_depth()
    }

    pub fn is_color_float(&self) -> bool {
        match *self {
            ImageFormat::RFloat16
            | ImageFormat::RFloat32
            | ImageFormat::RgFloat16
            | ImageFormat::RgFloat32
            | ImageFormat::RgbaFloat16
            | ImageFormat::RgbaFloat32 => true,
            _ => false,
        }
    }

    pub fn is_color_srgb(&self) -> bool {
        match *self {
            ImageFormat::SrgbR8
            | ImageFormat::SrgbRg8
            | ImageFormat::SrgbRgba8
            | ImageFormat::SrgbBgra8 => true,
            _ => false,
        }
    }

    pub fn color_int_type(&self) -> Option<(Signedness, Normalizedness)> {
        match *self {
            ImageFormat::R8(signedness, normalizedness)
            | ImageFormat::Rg8(signedness, normalizedness)
            | ImageFormat::Rgba8(signedness, normalizedness)
            | ImageFormat::Rgb10A2(signedness, normalizedness)
            | ImageFormat::R16(signedness, normalizedness)
            | ImageFormat::Rg16(signedness, normalizedness)
            | ImageFormat::Rgba16(signedness, normalizedness)
            | ImageFormat::R32(signedness, normalizedness)
            | ImageFormat::Rg32(signedness, normalizedness)
            | ImageFormat::Rgba32(signedness, normalizedness)
            | ImageFormat::Bgra8(signedness, normalizedness) => Some((signedness, normalizedness)),
            _ => None,
        }
    }

    pub fn has_depth(&self) -> bool {
        match *self {
            ImageFormat::Depth16
            | ImageFormat::Depth24
            | ImageFormat::DepthFloat32
            | ImageFormat::Depth24Stencil8
            | ImageFormat::DepthFloat32Stencil8 => true,
            _ => false,
        }
    }

    pub fn has_stencil(&self) -> bool {
        match *self {
            ImageFormat::Depth24Stencil8 | ImageFormat::DepthFloat32Stencil8 => true,
            _ => false,
        }
    }

    pub fn size_class(&self) -> ImageFormatSizeClass {
        match *self {
            ImageFormat::R8(_, _) => ImageFormatSizeClass::Color8,
            ImageFormat::Rg8(_, _) => ImageFormatSizeClass::Color16,
            ImageFormat::Rgba8(_, _) => ImageFormatSizeClass::Color32,
            ImageFormat::Rgb10A2(_, _) => ImageFormatSizeClass::Color32,
            ImageFormat::R16(_, _) => ImageFormatSizeClass::Color16,
            ImageFormat::Rg16(_, _) => ImageFormatSizeClass::Color32,
            ImageFormat::Rgba16(_, _) => ImageFormatSizeClass::Color64,
            ImageFormat::R32(_, _) => ImageFormatSizeClass::Color32,
            ImageFormat::Rg32(_, _) => ImageFormatSizeClass::Color64,
            ImageFormat::Rgba32(_, _) => ImageFormatSizeClass::Color128,
            ImageFormat::Bgra8(_, _) => ImageFormatSizeClass::Color32,
            ImageFormat::RFloat16 => ImageFormatSizeClass::Color16,
            ImageFormat::RFloat32 => ImageFormatSizeClass::Color32,
            ImageFormat::RgFloat16 => ImageFormatSizeClass::Color32,
            ImageFormat::RgFloat32 => ImageFormatSizeClass::Color64,
            ImageFormat::RgbaFloat16 => ImageFormatSizeClass::Color64,
            ImageFormat::RgbaFloat32 => ImageFormatSizeClass::Color128,
            ImageFormat::SrgbR8 => ImageFormatSizeClass::Color8,
            ImageFormat::SrgbRg8 => ImageFormatSizeClass::Color16,
            ImageFormat::SrgbRgba8 => ImageFormatSizeClass::Color32,
            ImageFormat::SrgbBgra8 => ImageFormatSizeClass::Color32,
            ImageFormat::Depth16 => ImageFormatSizeClass::Depth16,
            ImageFormat::Depth24 => ImageFormatSizeClass::Depth24,
            ImageFormat::DepthFloat32 => ImageFormatSizeClass::Depth32,
            ImageFormat::Depth24Stencil8 => ImageFormatSizeClass::Depth24Stencil8,
            ImageFormat::DepthFloat32Stencil8 => ImageFormatSizeClass::Depth32Stencil8,
        }
    }
}

/// Size classes for image formats.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ImageFormatSizeClass {
    /// Color format class with 8 bits per pixel.
    Color8,

    /// Color format class with 16 bits per pixel.
    Color16,

    /// Color format class with 24 bits per pixel.
    Color24,

    /// Color format class with 32 bits per pixel.
    Color32,

    /// Color format class with 64 bits per pixel.
    Color64,

    /// Color format class with 128 bits per pixel.
    Color128,

    /// Depth format class with 16 bits per pixel.
    Depth16,

    /// Depth format class with 24 bits per pixel.
    Depth24,

    /// Depth format class with 32 bits per pixel.
    Depth32,

    /// Depth and stencil combined format class with 24 and 8 bits per pixel
    /// for the depth and stencil component, respectively.
    Depth24Stencil8,

    /// Depth and stencil combined format class with 32 and 8 bits per pixel
    /// for the depth and stencil component, respectively
    Depth32Stencil8,
}

impl ImageFormatSizeClass {
    /// Compute the *minimum* number of bytes consumed by each pixel for a given format size class.
    pub fn num_bytes_per_pixel(&self) -> usize {
        match *self {
            ImageFormatSizeClass::Color8 => 1,
            ImageFormatSizeClass::Color16 => 2,
            ImageFormatSizeClass::Color24 => 3,
            ImageFormatSizeClass::Color32 => 4,
            ImageFormatSizeClass::Color64 => 8,
            ImageFormatSizeClass::Color128 => 16,
            ImageFormatSizeClass::Depth16 => 2,
            ImageFormatSizeClass::Depth24 => 3,
            ImageFormatSizeClass::Depth32 => 4,
            ImageFormatSizeClass::Depth24Stencil8 => 4,
            ImageFormatSizeClass::Depth32Stencil8 => 5,
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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, IterValues)]
pub enum Signedness {
    Unsigned,
    Signed,
}

impl Signedness {
    /// Retrieve all possible values of `Signedness`.
    pub fn values() -> &'static [Signedness] {
        lazy_static! {
            static ref VALUES: Vec<Signedness> = Signedness::iter_values().collect();
        }
        &VALUES
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, IterValues)]
pub enum Normalizedness {
    Unnormalized,
    Normalized,
}

impl Normalizedness {
    /// Retrieve all possible values of `Normalizedness`.
    pub fn values() -> &'static [Normalizedness] {
        lazy_static! {
            static ref VALUES: Vec<Normalizedness> = Normalizedness::iter_values().collect();
        }
        &VALUES
    }
}

/// Vertex format.
///
/// See [`IntAsScalarFormat`] and [`FloatAsScalarFormat`] for a convenient way
/// to construct a `VertexFormat`.
///
/// [`IntAsScalarFormat`]: IntAsScalarFormat
/// [`FloatAsScalarFormat`]: FloatAsScalarFormat
///
/// # Support status
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
pub struct VertexFormat(pub VecWidth, pub ScalarFormat);

impl VertexFormat {
    pub fn width(&self) -> usize {
        self.0.width()
    }

    pub fn size(&self) -> usize {
        self.width() * self.1.size()
    }

    /// Retrieve all possible values of `VertexFormat`.
    pub fn values() -> &'static [VertexFormat] {
        lazy_static! {
            static ref VALUES: Vec<VertexFormat> = <(VecWidth, ScalarFormat)>::iter_values()
                .map(|(x, y)| VertexFormat(x, y))
                .collect();
        }
        &VALUES
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, IterValues)]
pub enum VecWidth {
    Scalar,
    Vector2,
    Vector3,
    Vector4,
}

impl VecWidth {
    /// Retrieve all possible values of `VecWidth`.
    pub fn values() -> &'static [VecWidth] {
        lazy_static! {
            static ref VALUES: Vec<VecWidth> = VecWidth::iter_values().collect();
        }
        &VALUES
    }

    pub fn from_width(width: usize) -> Option<Self> {
        match width {
            1 => Some(VecWidth::Scalar),
            2 => Some(VecWidth::Vector2),
            3 => Some(VecWidth::Vector3),
            4 => Some(VecWidth::Vector4),
            _ => None,
        }
    }

    pub fn width(self) -> usize {
        match self {
            VecWidth::Scalar => 1,
            VecWidth::Vector2 => 2,
            VecWidth::Vector3 => 3,
            VecWidth::Vector4 => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, IterValues)]
pub enum ScalarFormat {
    I8(Signedness, Normalizedness),
    I16(Signedness, Normalizedness),
    I32(Signedness, Normalizedness),
    F32,
}

impl ScalarFormat {
    /// Retrieve all possible values of `ScalarFormat`.
    pub fn values() -> &'static [ScalarFormat] {
        lazy_static! {
            static ref VALUES: Vec<ScalarFormat> = ScalarFormat::iter_values().collect();
        }
        &VALUES
    }

    pub fn size(&self) -> usize {
        match *self {
            ScalarFormat::I8(_, _) => 1,
            ScalarFormat::I16(_, _) => 2,
            ScalarFormat::I32(_, _) => 4,
            ScalarFormat::F32 => 4,
        }
    }

    pub fn int_signedness(&self) -> Option<Signedness> {
        match *self {
            ScalarFormat::I8(s, _) | ScalarFormat::I16(s, _) | ScalarFormat::I32(s, _) => Some(s),
            ScalarFormat::F32 => None,
        }
    }

    pub fn int_normalizedness(&self) -> Option<Normalizedness> {
        match *self {
            ScalarFormat::I8(_, n) | ScalarFormat::I16(_, n) | ScalarFormat::I32(_, n) => Some(n),
            ScalarFormat::F32 => None,
        }
    }
}

impl ops::Mul<usize> for ScalarFormat {
    type Output = VertexFormat;

    /// Construct a `VertexFormat` from the element type and the vector width.
    ///
    /// Panics if `rhs` is not one of 1, 2, 3, and 4.
    fn mul(self, rhs: usize) -> VertexFormat {
        VertexFormat(VecWidth::from_width(rhs).unwrap(), self)
    }
}

/// Index buffer format.
///
/// See [`AsIndexFormat`] for a convenient way to construct a `IndexFormat`.
///
/// [`AsIndexFormat`]: AsIndexFormat
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum IndexFormat {
    U16,
    U32,
}

impl IndexFormat {
    /// Retrieve the number of bytes per vertex.
    pub fn size(&self) -> usize {
        match *self {
            IndexFormat::U16 => 2,
            IndexFormat::U32 => 4,
        }
    }
}

/// Constructs a [`IndexFormat`] from an integer type. Provided for convenience.
///
/// [`IndexFormat`]: IndexFormat
///
/// # Examples
///
///     use zangfx_base::formats::IndexFormat;
///     use zangfx_base::formats::AsIndexFormat;
///     assert_eq!(<u16>::as_index_format(), IndexFormat::U16);
///
pub trait AsIndexFormat: Sized {
    fn as_index_format() -> IndexFormat;
}

impl AsIndexFormat for u16 {
    fn as_index_format() -> IndexFormat {
        IndexFormat::U16
    }
}
impl AsIndexFormat for u32 {
    fn as_index_format() -> IndexFormat {
        IndexFormat::U32
    }
}

/// Constructs a [`ScalarFormat`] from an integer type.
/// Provided for convenience.
///
/// See also: [`FloatAsScalarFormat`].
///
/// [`ScalarFormat`]: ScalarFormat
/// [`FloatAsScalarFormat`]: FloatAsScalarFormat
///
/// # Examples
///
///     use zangfx_base::formats::{ScalarFormat, Signedness, Normalizedness};
///     use zangfx_base::formats::{VertexFormat, VecWidth};
///     use zangfx_base::formats::IntAsScalarFormat;
///     assert_eq!(
///         <u16>::as_format_norm(),
///         ScalarFormat::I16(Signedness::Unsigned, Normalizedness::Normalized),
///     );
///     assert_eq!(
///         <u16>::as_format_norm() * 3,
///         VertexFormat(
///             VecWidth::Vector3,
///             ScalarFormat::I16(Signedness::Unsigned, Normalizedness::Normalized)
///         ),
///     );
pub trait IntAsScalarFormat: Sized {
    fn as_format_norm() -> ScalarFormat;
    fn as_format_unnorm() -> ScalarFormat;
}

macro_rules! impl_int_as_scalar_format {
    ($t:ty, $signedness:expr, $idt:ident) => {
        impl IntAsScalarFormat for $t {
            fn as_format_norm() -> ScalarFormat {
                ScalarFormat::$idt($signedness, Normalizedness::Normalized)
            }
            fn as_format_unnorm() -> ScalarFormat {
                ScalarFormat::$idt($signedness, Normalizedness::Unnormalized)
            }
        }
    };
}

impl_int_as_scalar_format!(u8, Signedness::Unsigned, I8);
impl_int_as_scalar_format!(u16, Signedness::Unsigned, I16);
impl_int_as_scalar_format!(u32, Signedness::Unsigned, I32);
impl_int_as_scalar_format!(i8, Signedness::Signed, I8);
impl_int_as_scalar_format!(i16, Signedness::Signed, I16);
impl_int_as_scalar_format!(i32, Signedness::Signed, I32);

/// Constructs a [`ScalarFormat`] from a floating point value type.
/// Provided for convenience.
///
/// See also: [`IntAsScalarFormat`].
///
/// [`ScalarFormat`]: ScalarFormat
/// [`IntAsScalarFormat`]: IntAsScalarFormat
///
/// # Examples
///
///     use zangfx_base::formats::{ScalarFormat, Signedness, Normalizedness};
///     use zangfx_base::formats::{VertexFormat, VecWidth};
///     use zangfx_base::formats::FloatAsScalarFormat;
///     assert_eq!(
///         <f32>::as_format(),
///         ScalarFormat::F32
///     );
///     assert_eq!(
///         <f32>::as_format() * 3,
///         VertexFormat(VecWidth::Vector3, ScalarFormat::F32),
///     );
///
pub trait FloatAsScalarFormat: Sized {
    fn as_format() -> ScalarFormat;
}

impl FloatAsScalarFormat for f32 {
    fn as_format() -> ScalarFormat {
        ScalarFormat::F32
    }
}

/// Constructs a [`ImageFormat`] from an integer type.
/// Provided for convenience.
///
/// See also: [`FloatAsImageFormat`].
///
/// [`ImageFormat`]: ImageFormat
/// [`FloatAsImageFormat`]: FloatAsImageFormat
///
/// # Examples
///
///     use zangfx_base::formats::{ImageFormat, Signedness, Normalizedness};
///     use zangfx_base::formats::IntAsImageFormat;
///     assert_eq!(
///         <u8>::as_rgba_norm(),
///         ImageFormat::Rgba8(Signedness::Unsigned, Normalizedness::Normalized),
///     );
pub trait IntAsImageFormat: Sized {
    fn as_red_norm() -> ImageFormat;
    fn as_red_unnorm() -> ImageFormat;
    fn as_rg_norm() -> ImageFormat;
    fn as_rg_unnorm() -> ImageFormat;
    fn as_rgba_norm() -> ImageFormat;
    fn as_rgba_unnorm() -> ImageFormat;
}

fn scalar_to_red(value: ScalarFormat) -> ImageFormat {
    use crate::ImageFormat::*;
    use crate::ScalarFormat::*;
    match value {
        I8(sgn, norm) => R8(sgn, norm),
        I16(sgn, norm) => R16(sgn, norm),
        I32(sgn, norm) => R32(sgn, norm),
        F32 => RFloat32,
    }
}

fn scalar_to_rg(value: ScalarFormat) -> ImageFormat {
    use crate::ImageFormat::*;
    use crate::ScalarFormat::*;
    match value {
        I8(sgn, norm) => Rg8(sgn, norm),
        I16(sgn, norm) => Rg16(sgn, norm),
        I32(sgn, norm) => Rg32(sgn, norm),
        F32 => RgFloat32,
    }
}

fn scalar_to_rgba(value: ScalarFormat) -> ImageFormat {
    use crate::ImageFormat::*;
    use crate::ScalarFormat::*;
    match value {
        I8(sgn, norm) => Rgba8(sgn, norm),
        I16(sgn, norm) => Rgba16(sgn, norm),
        I32(sgn, norm) => Rgba32(sgn, norm),
        F32 => RgbaFloat32,
    }
}

impl<T: IntAsScalarFormat> IntAsImageFormat for T {
    fn as_red_norm() -> ImageFormat {
        scalar_to_red(Self::as_format_norm())
    }
    fn as_red_unnorm() -> ImageFormat {
        scalar_to_red(Self::as_format_unnorm())
    }
    fn as_rg_norm() -> ImageFormat {
        scalar_to_rg(Self::as_format_norm())
    }
    fn as_rg_unnorm() -> ImageFormat {
        scalar_to_rg(Self::as_format_unnorm())
    }
    fn as_rgba_norm() -> ImageFormat {
        scalar_to_rgba(Self::as_format_norm())
    }
    fn as_rgba_unnorm() -> ImageFormat {
        scalar_to_rgba(Self::as_format_unnorm())
    }
}

/// Constructs a [`ImageFormat`] from a floating point value type.
/// Provided for convenience.
///
/// See also: [`IntAsImageFormat`].
///
/// [`ImageFormat`]: ImageFormat
/// [`IntAsImageFormat`]: IntAsImageFormat
///
/// # Examples
///
///     use zangfx_base::formats::ImageFormat;
///     use zangfx_base::formats::FloatAsImageFormat;
///     assert_eq!(<f32>::as_rgba(), ImageFormat::RgbaFloat32);
///     assert_eq!(<f32>::as_half_red(), ImageFormat::RFloat16);
pub trait FloatAsImageFormat: Sized {
    fn as_red() -> ImageFormat;
    fn as_rg() -> ImageFormat;
    fn as_rgba() -> ImageFormat;
    fn as_half_red() -> ImageFormat;
    fn as_half_rg() -> ImageFormat;
    fn as_half_rgba() -> ImageFormat;
}

impl FloatAsImageFormat for f32 {
    fn as_red() -> ImageFormat {
        ImageFormat::RFloat32
    }
    fn as_rg() -> ImageFormat {
        ImageFormat::RgFloat32
    }
    fn as_rgba() -> ImageFormat {
        ImageFormat::RgbaFloat32
    }
    fn as_half_red() -> ImageFormat {
        ImageFormat::RFloat16
    }
    fn as_half_rg() -> ImageFormat {
        ImageFormat::RgFloat16
    }
    fn as_half_rgba() -> ImageFormat {
        ImageFormat::RgbaFloat16
    }
}
