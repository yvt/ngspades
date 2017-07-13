//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::clone::Clone;
use std::hash::Hash;
use std::fmt::Debug;
use std::cmp::{Eq, PartialEq};
use std::any::Any;

use enumflags::BitFlags;

use cgmath::prelude::*;
use cgmath::Vector3;

use {ImageFormat, Signedness, Normalizedness, Validate, DeviceCapabilities, Marker, StorageMode};

/// Handle for image objects.
pub trait Image
    : Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any + Marker {
    // TODO: get image subresource layout
}

/// Handle for image view objects.
pub trait ImageView
    : Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any + Marker {
}

/// Image description.
///
/// See [`ImageDescriptionValidationError`](enum.ImageDescriptionValidationError.html) for the valid usage.
#[derive(Debug, Clone, Copy)]
pub struct ImageDescription {
    pub flags: ImageFlags,
    pub usage: ImageUsageFlags,
    pub image_type: ImageType,
    pub format: ImageFormat,
    pub extent: Vector3<u32>,
    pub num_mip_levels: u32,
    pub num_array_layers: u32,
    pub initial_layout: ImageLayout,
    pub tiling: ImageTiling,
    pub storage_mode: StorageMode,
}

impl ::std::default::Default for ImageDescription {
    fn default() -> Self {
        Self {
            flags: BitFlags::empty(),
            usage: BitFlags::empty(),
            image_type: ImageType::TwoD,
            format: ImageFormat::Rgba8(Signedness::Unsigned, Normalizedness::Normalized),
            extent: Vector3::new(1, 1, 1),
            num_mip_levels: 1,
            num_array_layers: 1,
            initial_layout: ImageLayout::Undefined,
            tiling: ImageTiling::Optimal,
            storage_mode: StorageMode::Private,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ImageType {
    OneD,
    TwoD,
    TwoDArray,
    ThreeD,
    Cube,
    CubeArray,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ImageTiling {
    Optimal,
    Linear,
}

#[derive(Debug, Clone, Copy)]
pub struct ImageViewDescription<'a, TImage: Image> {
    /// Specifies the arrangement of the image view.
    ///
    /// If the image was not created with `ImageFlag::MutableType`, this must
    /// be equal to the one used to create the image.
    ///
    /// Only the following combinations of the original image's `ImageType` and
    /// the image view's one are supported:
    ///
    /// |  Original Image Type  |               View Image Type               |
    /// | --------------------- | ------------------------------------------- |
    /// | `OneD`                | `OneD`                                      |
    /// | `TwoD` or `TwoDArray` | `TwoD` or `TwoDArray`                       |
    /// | `Cube` or `CubeArray` | `TwoD`, `TwoDArray`, `Cube`, or `CubeArray` |
    /// | `ThreeD`              | `ThreeD`                                    |
    pub image_type: ImageType,

    /// Specifies the image to create a image view from.
    pub image: &'a TImage,

    /// Specifies the image format.
    ///
    /// If the image was not created with `ImageFlag::MutableFormat`, this must
    /// be equal to the one used to create the image.
    pub format: ImageFormat,

    /// Specifies the ranges of slices and mip levels that are visible via the
    /// image view.
    ///
    /// If the image was not created with `ImageFlag::SubrangeViewCompatible`,
    /// the following restriction applies if this does not specify entire the
    /// image:
    ///
    ///  - The created image view cannot be used as an element of a descriptor set.
    ///
    pub range: ImageSubresourceRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ImageSubresourceRange {
    /// The first mipmap level accessible to the view.
    pub base_mip_level: u32,

    /// The number of mipmap levels. Use `None` to specify all remaining levels.
    pub num_mip_levels: Option<u32>,

    /// The first array layer accessible to the view.
    pub base_array_layer: u32,

    /// The number of array layers. Use `None` to specify all remaining layers.
    pub num_array_layers: Option<u32>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ImageLayout {
    Undefined,
    General,
    ColorAttachment,
    DepthStencilAttachment,
    DepthStencilRead,
    ShaderRead,
    TransferSource,
    TransferDestination,
    Preinitialized,
    Present,
}


// prevent `InnerXXX` from being exported
mod flags {
    #[derive(EnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
    #[repr(u32)]
    pub enum ImageUsage {
        TransferSource = 0b00000001,
        TransferDestination = 0b00000010,
        Sampled = 0b00000100,
        Storage = 0b00001000,
        ColorAttachment = 0b00010000,
        DepthStencilAttachment = 0b00100000,
        InputAttachment = 0b01000000,
        TransientAttachment = 0b10000000,
    }

    #[derive(EnumFlags, Copy, Clone, Debug, Hash, PartialEq, Eq)]
    #[repr(u32)]
    pub enum ImageFlag {
        /// Indicates `ImageView` can created from the image with a `ImageFormat` different
        /// from the one used to create the image.
        MutableFormat = 0b001,

        /// Indicates `ImageView` created from the image with a `ImageSubresourceRange` that
        /// does not specify entire the image (e.g., `base_mip_level` is not zero) can be used
        /// as an element of a descriptor set.
        ///
        /// Even without this flag, such `ImageView`s can be used for other purposes.
        SubrangeViewCompatible = 0b010,

        /// Indicates `ImageView` created from the image with a `ImageType` different
        /// from the one used to create the image can be used as an element of
        /// a descriptor set.
        MutableType = 0b100,
    }
}

pub use self::flags::{ImageUsage, ImageFlag};

pub type ImageFlags = BitFlags<ImageFlag>;
pub type ImageUsageFlags = BitFlags<ImageUsage>;

/// Validation errors for [`ImageDescription`](struct.ImageDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ImageDescriptionValidationError {
    /// One of `extent`'s element is `0`. (Vulkan 1.0 "11.3. Images" valid usage)
    ZeroExtent,
    /// `num_mip_levels` is `0`. (Vulkan 1.0 "11.3. Images" valid usage)
    ZeroMipLevels,
    /// `num_array_layers` is `0`. (Vulkan 1.0 "11.3. Images" valid usage)
    ZeroArrayLayers,
    /// Either of `ImageType::Cube` and `ImageType::CubeArray` is specified and `extent.x` is not equal to `extent.y`.
    /// (Vulkan 1.0, "11.3. Images" valid usage)
    CubeButNotSquare,
    /// Either of `ImageType::Cube` and `ImageType::CubeArray` is specified and `num_array_layers` is not a multiple of 6.
    CubeWithInvalidNumberOfLayers,
    /// `ImageTiling::Linear` is specified and `image_type` is not `ImageType::TwoD`,
    /// (macOS 10.12 Metal, `MTLBuffer.makeTexture`)
    LinearTilingButNot2D,
    /// `ImageTiling::Linear` is specified and `num_mip_levels` is greater than `1`,
    /// (macOS 10.12 Metal, `MTLBuffer.makeTexture`)
    LinearTilingButUsingMipmap,
    /// `extent` is greater than appropriate one of `DeviceLimits.max_image_extent_1d`,
    /// `DeviceLimits.max_image_extent_2d`, and `DeviceLimits.max_image_extent_3d`.
    /// (Vulkan 1.0, "11.3. Images" valid usage)
    ExtentTooLarge,
    /// Some elements in `extent` irrevelant for the image type are not set to `1`.
    /// (Vulkan 1.0, "11.3. Images" valid usage)
    InvalidExtentForImageType,
    /// `num_mip_levels` is not less than or equal to `log2(extent.max()) + 1`.
    /// (Vulkan 1.0, "11.3. Images" valid usage)
    TooManyMipLevels,
    /// `num_array_layers` is greater than `DeviceLimits.max_image_num_array_layers`.
    /// (Vulkan 1.0, "11.3. Images" valid usage)
    TooManyArrayLayers,
    /// `ImageType::ThreeD` is specified and `num_array_layers` is greater than `1`.
    /// (Vulkan 1.0, "11.3. Images" valid usage)
    ArrayBut3D,
    /// `ImageUsage::TransientAttachment` is specified, and one or more usages except
    /// `ImageUsage::ColorAttachment`, `ImageUsage::DepthStencilAttachment`, and
    /// `ImageUsage::InputAttachment` are specified.
    /// (Vulkan 1.0, "11.3. Images" valid usage)
    TransientButHasNonAttachmentUsage,
    /// `extent` is greater than appropriate one of `DeviceLimits.max_framebuffer_extent`
    /// and `usage` contains at least one of `ImageUsage::ColorAttachment`,
    /// `ImageUsage::DepthStencilAttachment`, `ImageUsage::InputAttachment`, and
    /// `ImageUsage::TransientAttachment`.
    /// (Vulkan 1.0, "11.3. Images" valid usage)
    ExtentTooLargeForFramebuffer,
    /// `initial_layout` is not either of `Undefined` or `Preinitialized`.
    InvalidInitialLayout,
    /// `image_type` is not one of `TwoDArray` and `CubeArray` but `num_array_layers` is
    /// greater than `1` or `6` for (`Cube`).
    NonArrayButHasMultipleLayers,
    /// `image_type` is `CubeArray` and `DeviceLimits.supports_cube_array` is `false`.
    /// (Vulkan 1.0, "11.5. Image Views" valid usage)
    CubeArrayNotSupported,
}

impl Validate for ImageDescription {
    type Error = ImageDescriptionValidationError;

    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
    where
        T: FnMut(Self::Error) -> (),
    {
        if self.extent.x == 0 || self.extent.y == 0 || self.extent.z == 0 {
            callback(ImageDescriptionValidationError::ZeroExtent);
        }

        if self.num_mip_levels == 0 {
            callback(ImageDescriptionValidationError::ZeroMipLevels);
        }
        if self.num_array_layers == 0 {
            callback(ImageDescriptionValidationError::ZeroArrayLayers);
        }

        if self.image_type == ImageType::Cube || self.image_type == ImageType::CubeArray {
            if self.extent.x != self.extent.y {
                callback(ImageDescriptionValidationError::CubeButNotSquare);
            }
            if self.num_array_layers % 6 == 0 {
                callback(
                    ImageDescriptionValidationError::CubeWithInvalidNumberOfLayers,
                );
            }
        }

        let max_num_layers_by_type = match self.image_type {
            ImageType::OneD | ImageType::TwoD | ImageType::ThreeD => Some(1),
            ImageType::Cube => Some(6),
            _ => None,
        };
        if let Some(i) = max_num_layers_by_type {
            if self.num_array_layers > i {
                callback(
                    ImageDescriptionValidationError::NonArrayButHasMultipleLayers,
                );
            }
        }

        if self.tiling == ImageTiling::Linear && self.image_type != ImageType::TwoD {
            callback(ImageDescriptionValidationError::LinearTilingButNot2D);
        }
        if self.tiling == ImageTiling::Linear && self.num_mip_levels > 1 {
            callback(ImageDescriptionValidationError::LinearTilingButUsingMipmap);
        }

        if match self.image_type {
            ImageType::OneD => self.extent.y != 1 || self.extent.z != 1,
            ImageType::TwoD | ImageType::Cube | ImageType::TwoDArray | ImageType::CubeArray => {
                self.extent.z != 1
            }
            ImageType::ThreeD => false,
        }
        {
            callback(ImageDescriptionValidationError::InvalidExtentForImageType);
        }

        let log2floor = 31 - self.extent.max().leading_zeros();
        if self.num_mip_levels > log2floor + 1 {
            callback(ImageDescriptionValidationError::TooManyMipLevels);
        }

        if self.image_type == ImageType::ThreeD && self.num_array_layers > 1 {
            callback(ImageDescriptionValidationError::ArrayBut3D);
        }

        if !(self.usage & ImageUsage::TransientAttachment).is_empty() &&
            !(self.usage &
                  (ImageUsage::ColorAttachment | ImageUsage::DepthStencilAttachment |
                       ImageUsage::InputAttachment)
                      .not())
                .is_empty()
        {
            callback(
                ImageDescriptionValidationError::TransientButHasNonAttachmentUsage,
            );
        }

        match self.initial_layout {
            ImageLayout::Undefined |
            ImageLayout::Preinitialized => {}
            _ => {
                callback(ImageDescriptionValidationError::InvalidInitialLayout);
            }
        }

        match cap {
            Some(cap) => {
                let limits = cap.limits();
                if self.extent.max() >
                    match self.image_type {
                        ImageType::OneD => limits.max_image_extent_1d,
                        ImageType::TwoD | ImageType::Cube | ImageType::TwoDArray |
                        ImageType::CubeArray => limits.max_image_extent_2d,
                        ImageType::ThreeD => limits.max_image_extent_3d,
                    }
                {
                    callback(ImageDescriptionValidationError::ExtentTooLarge);
                }

                if !(self.usage &
                         (ImageUsage::ColorAttachment | ImageUsage::DepthStencilAttachment |
                              ImageUsage::InputAttachment |
                              ImageUsage::TransientAttachment))
                    .is_empty() &&
                    self.extent.max() > limits.max_framebuffer_extent
                {
                    callback(
                        ImageDescriptionValidationError::ExtentTooLargeForFramebuffer,
                    );
                }

                if self.num_array_layers > limits.max_image_num_array_layers {
                    callback(ImageDescriptionValidationError::TooManyArrayLayers);
                }

                if !limits.supports_cube_array && self.image_type == ImageType::CubeArray {
                    callback(ImageDescriptionValidationError::CubeArrayNotSupported);
                }
            }
            None => {}
        }
    }
}

/// Validation errors for [`ImageViewDescription`](struct.ImageViewDescription.html).
///
/// Compatibility with the speciied image is not checked by the core validator because
/// the image's original `ImageDescription` is not accessible to the core (backends are
/// not required to keep `ImageDescription` in image handles). If `ImageDescription` is
/// available, [`compatible_with_image`] can be used to check the compatibility.
///
/// [`compatible_with_image`]: struct.ImageViewDescription.html#method.compatible_with_image
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ImageViewDescriptionValidationError {
    /// `num_mip_levels` is `0`.
    /// (Vulkan 1.0, "11.5. Image Views" valid usage)
    ZeroMipLevels,
    /// `num_array_layers` is `0`.
    /// (Vulkan 1.0, "11.5. Image Views" valid usage)
    ZeroArrayLayers,
    /// `num_mip_levels.unwrap_or(1) + base_mip_level` is greater than `log2(max_extent) + 1`.
    /// where `max_extent` is the maximum value of `DeviceLimits.max_image_extent_1d`,
    /// `DeviceLimits.max_image_extent_2d`, and `DeviceLimits.max_image_extent_3d`.
    /// (Vulkan 1.0, "11.5. Image Views" valid usage)
    ///
    /// **To backend implementors**: Backends must also check whether the mipmap level
    /// is valid for the specified image. `compatible_with_image` does this check.
    TooManyMipLevels,
    /// `num_array_layers.unwrap_or(1) + base_array_layer` is greater than
    /// `DeviceLimits.max_image_num_array_layers`.
    /// (Vulkan 1.0, "11.5. Image Views" valid usage)
    ///
    /// **To backend implementors**: Backends must also check whether the array layer
    /// is valid for the specified image. `compatible_with_image` does this check.
    TooManyArrayLayers,
    /// `image_type` is `CubeArray` and `DeviceLimits.supports_cube_array` is `false`.
    /// (Vulkan 1.0, "11.5. Image Views" valid usage)
    CubeArrayNotSupported,
}

impl<'a, TImage: Image> Validate for ImageViewDescription<'a, TImage> {
    type Error = ImageViewDescriptionValidationError;

    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
    where
        T: FnMut(Self::Error) -> (),
    {
        if self.range.num_mip_levels == Some(0) {
            callback(ImageViewDescriptionValidationError::ZeroMipLevels);
        }
        if self.range.num_array_layers == Some(0) {
            callback(ImageViewDescriptionValidationError::ZeroArrayLayers);
        }
        if self.range.num_mip_levels.unwrap_or(1).checked_add(
            self.range
                .base_mip_level,
        ) == None
        {
            callback(ImageViewDescriptionValidationError::TooManyMipLevels);
        }
        if self.range.num_array_layers.unwrap_or(1).checked_add(
            self.range.base_array_layer,
        ) == None
        {
            callback(ImageViewDescriptionValidationError::TooManyArrayLayers);
        }

        if let Some(cap) = cap {
            let limits: &::DeviceLimits = cap.limits();

            let max_extent = *[
                limits.max_image_extent_1d,
                limits.max_image_extent_2d,
                limits.max_image_extent_3d,
            ].iter()
                .max()
                .unwrap();
            let log2floor = 31 - max_extent.leading_zeros();
            if self.range.num_mip_levels.unwrap_or(1).saturating_add(
                self.range
                    .base_mip_level,
            ) > log2floor + 1
            {
                callback(ImageViewDescriptionValidationError::TooManyMipLevels);
            }

            if self.range.num_array_layers.unwrap_or(1).saturating_add(
                self.range.base_array_layer,
            ) > limits.max_image_num_array_layers
            {
                callback(ImageViewDescriptionValidationError::TooManyArrayLayers);
            }

            if !limits.supports_cube_array && self.image_type == ImageType::CubeArray {
                callback(ImageViewDescriptionValidationError::CubeArrayNotSupported);
            }
        }
    }
}

/// Used with the `Validate` trait to check the compatibility of an image view and
/// the image it is based on.
#[derive(Debug, Clone, Copy)]
pub struct CombinedImageAndImageViewDescription<'a, TImage: Image>(
    pub &'a ImageDescription,
    pub &'a ImageViewDescription<'a, TImage>
);

/// Validation errors for [`CombinedImageAndImageViewDescription`](struct.CombinedImageAndImageViewDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum CombinedImageAndImageViewDescriptionValidationError {
    /// An unsupported combination of `ImageType` and `ImageType` was found.
    TypeIncompatible,
    /// `num_mip_levels.unwrap_or(1) + base_mip_level` is greater than `image_desc.num_mip_levels`.
    /// (Vulkan 1.0, "11.5. Image Views" valid usage)
    TooManyMipLevels,
    /// `num_array_layers.unwrap_or(1) + base_array_layer` is greater than `image_desc.num_array_layers`.
    /// (Vulkan 1.0, "11.5. Image Views" valid usage)
    TooManyArrayLayers,
    /// `num_array_layers` is not `1` (`6` for cube images) and `image_type` is not `TwoDArray` nor `CubeArray`.
    /// (Vulkan 1.0, "11.5. Image Views" Table 8)
    InvalidArrayLayersForNonLayeredView,
    /// `num_array_layers` is not a multiple of `6` and `image_type` is `CubeArray`.
    /// (Vulkan 1.0, "11.5. Image Views" Table 8)
    InvalidArrayLayersForCubeArray,
    /// `format` is different from the one that was used to create the image, and
    /// `ImageFlag::MutableFormat` was not specified.
    DifferentFormat,

    // TODO: image format compatibility
}

impl<'a, TImage: Image> Validate for CombinedImageAndImageViewDescription<'a, TImage> {
    type Error = CombinedImageAndImageViewDescriptionValidationError;

    fn validate<T>(&self, _: Option<&DeviceCapabilities>, mut callback: T)
    where
        T: FnMut(Self::Error) -> (),
    {
        let image_desc = self.0;
        let view_desc = self.1;

        if view_desc.range.num_mip_levels.unwrap_or(1).saturating_add(
            view_desc.range.base_mip_level,
        ) > image_desc.num_mip_levels
        {
            callback(
                CombinedImageAndImageViewDescriptionValidationError::TooManyMipLevels,
            );
        }

        if view_desc
            .range
            .num_array_layers
            .unwrap_or(1)
            .saturating_add(view_desc.range.base_array_layer) >
            image_desc.num_array_layers
        {
            callback(
                CombinedImageAndImageViewDescriptionValidationError::TooManyArrayLayers,
            );
        }

        if view_desc.format != image_desc.format &&
            (image_desc.flags & ImageFlag::MutableFormat).is_empty()
        {
            callback(
                CombinedImageAndImageViewDescriptionValidationError::DifferentFormat,
            );
        }

        let num_array_layers = view_desc.range.num_array_layers.unwrap_or(
            image_desc.num_array_layers.saturating_sub(
                view_desc.range.base_array_layer,
            ),
        );

        match (view_desc.image_type, image_desc.image_type) {
            (ImageType::OneD, ImageType::OneD) => {
                if num_array_layers != 1 {
                    callback(CombinedImageAndImageViewDescriptionValidationError::InvalidArrayLayersForNonLayeredView);
                }
            }
            (ImageType::TwoD, ImageType::TwoD) |
            (ImageType::TwoD, ImageType::TwoDArray) |
            (ImageType::TwoD, ImageType::Cube) |
            (ImageType::TwoD, ImageType::CubeArray) => {
                if num_array_layers != 1 {
                    callback(CombinedImageAndImageViewDescriptionValidationError::InvalidArrayLayersForNonLayeredView);
                }
            }
            (ImageType::TwoDArray, ImageType::TwoD) |
            (ImageType::TwoDArray, ImageType::TwoDArray) |
            (ImageType::TwoDArray, ImageType::Cube) |
            (ImageType::TwoDArray, ImageType::CubeArray) => {}
            (ImageType::Cube, ImageType::Cube) |
            (ImageType::Cube, ImageType::CubeArray) => {
                if num_array_layers != 6 {
                    callback(CombinedImageAndImageViewDescriptionValidationError::InvalidArrayLayersForNonLayeredView);
                }
            }
            (ImageType::CubeArray, ImageType::Cube) |
            (ImageType::CubeArray, ImageType::CubeArray) => {
                if num_array_layers % 6 == 0 {
                    callback(CombinedImageAndImageViewDescriptionValidationError::InvalidArrayLayersForCubeArray);
                }
            }
            (ImageType::ThreeD, ImageType::ThreeD) => {
                if num_array_layers != 1 {
                    callback(CombinedImageAndImageViewDescriptionValidationError::InvalidArrayLayersForNonLayeredView);
                }
            }
            // TODO: (ImageType::TwoD, ImageType::ThreeD)?
            // TODO: (ImageType::TwoDArray, ImageType::ThreeD)?
            _ => {
                callback(
                    CombinedImageAndImageViewDescriptionValidationError::TypeIncompatible,
                );
            }
        }
    }
}
