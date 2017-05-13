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

use super::{ImageFormat, Signedness, Normalizedness, Validate, DeviceCapabilities};

/// Image handle.
pub trait Image: Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any {
    // TODO: get image subresource layout
}

/// Image view handle.
pub trait ImageView: Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any {}

/// Image description.
///
/// See [`ImageDescriptionValidationError`](enum.ImageDescriptionValidationError.html) for the valid usage.
#[derive(Debug, Clone, Copy)]
pub struct ImageDescription {
    pub flags: BitFlags<ImageFlags>,
    pub usage: BitFlags<ImageUsageFlags>,
    pub image_type: ImageType,
    pub format: ImageFormat,
    pub extent: Vector3<u32>,
    pub num_mip_levels: u32,
    pub num_array_layers: u32,
    pub initial_layout: ImageLayout,
    pub tiling: ImageTiling,
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
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ImageType {
    OneD,
    TwoD,
    ThreeD,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ImageTiling {
    Optimal,
    Linear,
}

#[derive(Debug, Clone, Copy)]
pub struct ImageViewDescription<'a, TImage: Image> {
    pub view_type: ImageViewType,
    pub image: &'a TImage,
    // TODO: mip layer, etc.
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ImageViewType {
    OneD,
    TwoD,
    TwoDArray,
    ThreeD,
    Cube,
    CubeArray,
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
    #[derive(EnumFlags, Copy, Clone, Debug, Hash)]
    #[repr(u32)]
    pub enum ImageUsageFlags {
        TransferSource = 0b00000001,
        TransferDestination = 0b00000010,
        Sampled = 0b00000100,
        Storage = 0b00001000,
        ColorAttachment = 0b00010000,
        DepthStencilAttachment = 0b00100000,
        InputAttachment = 0b01000000,
        TransientAttachment = 0b10000000,
    }

    #[derive(EnumFlags, Copy, Clone, Debug, Hash)]
    #[repr(u32)]
    pub enum ImageFlags {
        CubeCompatible = 0b1,
    }
}

pub use self::flags::{ImageUsageFlags, ImageFlags};

/// Validation errors for [`ImageDescription`](struct.ImageDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ImageDescriptionValidationError {
    /// One of `extent`'s element is `0`. (Vulkan 1.0 "11.3. Images" valid usage)
    ZeroExtent,
    /// `num_mip_levels` is `0`. (Vulkan 1.0 "11.3. Images" valid usage)
    ZeroMipLevels,
    /// `num_array_layers` is `0`. (Vulkan 1.0 "11.3. Images" valid usage)
    ZeroArrayLayers,
    /// `ImageFlags::CubeCompatible` is specified and `image_type` is not `ImageType::TwoD`.
    /// (Vulkan 1.0, "11.3. Images" valid usage)
    CubeCompatibleButNot2D,
    /// `ImageFlags::CubeCompatible` is specified and `extent.x` is not equal to `extent.y`.
    /// (Vulkan 1.0, "11.3. Images" valid usage)
    CubeCompatibleButNotSquare,
    /// `ImageFlags::CubeCompatible` is specified and `num_array_layers` is less than 6.
    /// (Vulkan 1.0, "11.3. Images" valid usage)
    CubeCompatibleButNotEnoughLayers,
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
    /// `num_mip_levels` is not  less than or equal to `log2(extent.max()) + 1`.
    /// (Vulkan 1.0, "11.3. Images" valid usage)
    TooManyMipLevels,
    /// `num_array_layers` is greater than `DeviceLimits.max_image_num_array_layers`.
    /// (Vulkan 1.0, "11.3. Images" valid usage)
    TooManyArrayLayers,
    /// `ImageType::ThreeD` is specified and `num_array_layers` is greater than `1`.
    /// (Vulkan 1.0, "11.3. Images" valid usage)
    ArrayBut3D,
    /// `ImageUsageFlags::TransientAttachment` is specified, and one or more usages except
    /// `ImageUsageFlags::ColorAttachment`, `ImageUsageFlags::DepthStencilAttachment`, and
    /// `ImageUsageFlags::InputAttachment` are specified.
    /// (Vulkan 1.0, "11.3. Images" valid usage)
    TransientButHasNonAttachmentUsage,
    /// `extent` is greater than appropriate one of `DeviceLimits.max_framebuffer_extent`
    /// and `usage` contains at least one of `ImageUsageFlags::ColorAttachment`,
    /// `ImageUsageFlags::DepthStencilAttachment`, `ImageUsageFlags::InputAttachment`, and
    /// `ImageUsageFlags::TransientAttachment`.
    /// (Vulkan 1.0, "11.3. Images" valid usage)
    ExtentTooLargeForFramebuffer,
    /// `initial_layout` is not either of `Undefined` or `Preinitialized`.
    InvalidInitialLayout,
}

impl Validate for ImageDescription {
    type Error = ImageDescriptionValidationError;

    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
        where T: FnMut(Self::Error) -> ()
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

        if !(self.flags & ImageFlags::CubeCompatible).is_empty() {
            if self.image_type != ImageType::TwoD {
                callback(ImageDescriptionValidationError::CubeCompatibleButNot2D);
            }
            if self.extent.x != self.extent.y {
                callback(ImageDescriptionValidationError::CubeCompatibleButNotSquare);
            }
            if self.num_array_layers < 6 {
                callback(ImageDescriptionValidationError::CubeCompatibleButNotEnoughLayers);
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
               ImageType::TwoD => self.extent.z != 1,
               ImageType::ThreeD => false,
           } {
            callback(ImageDescriptionValidationError::InvalidExtentForImageType);
        }

        let log2floor = 31 - self.extent.max().leading_zeros();
        if self.num_mip_levels > log2floor + 1 {
            callback(ImageDescriptionValidationError::TooManyMipLevels);
        }

        if self.image_type == ImageType::ThreeD && self.num_array_layers > 1 {
            callback(ImageDescriptionValidationError::ArrayBut3D);
        }

        if !(self.usage & ImageUsageFlags::TransientAttachment).is_empty() &&
           !(self.usage &
             (ImageUsageFlags::ColorAttachment | ImageUsageFlags::DepthStencilAttachment |
              ImageUsageFlags::InputAttachment)
                     .not())
                    .is_empty() {
            callback(ImageDescriptionValidationError::TransientButHasNonAttachmentUsage);
        }

        match self.initial_layout {
            ImageLayout::Undefined | ImageLayout::Preinitialized => {}
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
                       ImageType::TwoD => limits.max_image_extent_2d,
                       ImageType::ThreeD => limits.max_image_extent_3d,
                   } {
                    callback(ImageDescriptionValidationError::ExtentTooLarge);
                }

                if !(self.usage &
                     (ImageUsageFlags::ColorAttachment | ImageUsageFlags::DepthStencilAttachment |
                      ImageUsageFlags::InputAttachment |
                      ImageUsageFlags::TransientAttachment))
                            .is_empty() &&
                   self.extent.max() > limits.max_framebuffer_extent {
                    callback(ImageDescriptionValidationError::ExtentTooLargeForFramebuffer);
                }

                if self.num_array_layers > limits.max_image_num_array_layers {
                    callback(ImageDescriptionValidationError::TooManyArrayLayers);
                }
            }
            None => {}
        }
    }
}

/// Validation errors for [`ImageViewDescription`](struct.ImageViewDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ImageViewDescriptionValidationError {
    // TODO
}

impl<'a, TImage: Image> Validate for ImageViewDescription<'a, TImage> {
    type Error = ImageViewDescriptionValidationError;

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
        where T: FnMut(Self::Error) -> ()
    {
        // TODO
    }
}
