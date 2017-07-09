//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::vk;
use core;

use std::hash::Hasher;
use std::ops::Deref;
use std::sync::Arc;

// TODO: Merge this with `metal/src/utils.rs` into one file

/// Checks the referential equality on references.
///
/// This would break if Rust had a moving garbage collector.
#[allow(dead_code)]
pub fn ref_eq<T: ?Sized>(a: &T, b: &T) -> bool {
    a as *const T == b as *const T
}

/// Compute a hash value based on the referential equality on references.
///
/// This would break if Rust had a moving garbage collector.
pub fn ref_hash<T: ?Sized, H: Hasher>(value: &T, state: &mut H) {
    state.write_usize(unsafe { ::std::mem::transmute_copy(&(value as *const T)) });
}

/// `Box` wrapper that provides a referential equality.
#[derive(Debug)]
pub struct RefEqBox<T: ?Sized>(Box<T>);

impl<T: ?Sized> PartialEq for RefEqBox<T> {
    fn eq(&self, other: &Self) -> bool {
        ::std::ptr::eq(&*self.0, &*other.0)
    }
}
impl<T: ?Sized> Eq for RefEqBox<T> {}
impl<T: ?Sized> ::std::hash::Hash for RefEqBox<T> {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        ref_hash(&*self.0, state);
    }
}

impl<T> RefEqBox<T> {
    pub fn new(data: T) -> Self {
        RefEqBox(Box::new(data))
    }
}

impl<T: Clone> Clone for RefEqBox<T> {
    fn clone(&self) -> Self {
        RefEqBox(self.0.clone())
    }
}

impl<T: ?Sized> Deref for RefEqBox<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<T: ?Sized> ::std::ops::DerefMut for RefEqBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

/// `Arc` wrapper that provides a referential equality.
#[derive(Debug)]
pub struct RefEqArc<T: ?Sized>(Arc<T>);

impl<T: ?Sized> PartialEq for RefEqArc<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl<T: ?Sized> Eq for RefEqArc<T> {}
impl<T: ?Sized> ::std::hash::Hash for RefEqArc<T> {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        ref_hash(&*self.0, state);
    }
}

impl<T> RefEqArc<T> {
    pub fn new(data: T) -> Self {
        RefEqArc(Arc::new(data))
    }
}

impl<T: ?Sized> Clone for RefEqArc<T> {
    fn clone(&self) -> Self {
        RefEqArc(self.0.clone())
    }
}

impl<T: ?Sized> Deref for RefEqArc<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

/// Translates a subset of `vk::Result` values into `core::GenericError`.
///
/// The following input values are permitted:
///
///  - `ErrorOutOfDeviceMemory`
///  - `ErrorDeviceLost`
///
/// `ErrorOutOfHostMemory` is escalated to a panic. (Maybe we should call `alloc::oom::oom()`?)
///
/// Unsupported values are returned unmodified.
pub(crate) fn translate_generic_error(
    result: vk::Result,
) -> Result<core::GenericError, vk::Result> {
    match result {
        vk::Result::ErrorOutOfDeviceMemory => Ok(core::GenericError::OutOfDeviceMemory),
        vk::Result::ErrorDeviceLost => Ok(core::GenericError::DeviceLost),
        vk::Result::ErrorOutOfHostMemory => panic!("out of memory"),
        result => Err(result),
    }
}

/// Equivalent to `translate_generic_error(result).unwrap()`.
///
/// That is, following errors are handled with this function:
///
///  - `ErrorOutOfDeviceMemory`
///  - `ErrorDeviceLost`
///  - `ErrorOutOfHostMemory` (escalated to a panic)
///
pub(crate) fn translate_generic_error_unwrap(result: vk::Result) -> core::GenericError {
    translate_generic_error(result).unwrap()
}

pub(crate) fn translate_image_layout(value: core::ImageLayout) -> vk::ImageLayout {
    match value {
        core::ImageLayout::Undefined => vk::ImageLayout::Undefined,
        core::ImageLayout::General => vk::ImageLayout::General,
        core::ImageLayout::ColorAttachment => vk::ImageLayout::ColorAttachmentOptimal,
        core::ImageLayout::DepthStencilAttachment => vk::ImageLayout::DepthStencilAttachmentOptimal,
        core::ImageLayout::DepthStencilRead => vk::ImageLayout::DepthStencilReadOnlyOptimal,
        core::ImageLayout::ShaderRead => vk::ImageLayout::ShaderReadOnlyOptimal,
        core::ImageLayout::TransferSource => vk::ImageLayout::TransferSrcOptimal,
        core::ImageLayout::TransferDestination => vk::ImageLayout::TransferDstOptimal,
        core::ImageLayout::Preinitialized => vk::ImageLayout::Preinitialized,
        core::ImageLayout::Present => vk::ImageLayout::PresentSrcKhr,
    }
}

pub(crate) fn translate_image_subresource_range(
    value: &core::ImageSubresourceRange,
    aspect_mask: vk::ImageAspectFlags,
) -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange {
        aspect_mask,
        base_mip_level: value.base_mip_level,
        base_array_layer: value.base_array_layer,
        level_count: value.num_mip_levels.unwrap_or(vk::VK_REMAINING_MIP_LEVELS),
        layer_count: value.num_array_layers.unwrap_or(
            vk::VK_REMAINING_ARRAY_LAYERS,
        ),
    }
}

pub(crate) fn translate_compare_function(value: core::CompareFunction) -> vk::CompareOp {
    match value {
        core::CompareFunction::Never => vk::CompareOp::Never,
        core::CompareFunction::Less => vk::CompareOp::Less,
        core::CompareFunction::Equal => vk::CompareOp::Equal,
        core::CompareFunction::LessEqual => vk::CompareOp::LessOrEqual,
        core::CompareFunction::Greater => vk::CompareOp::Greater,
        core::CompareFunction::NotEqual => vk::CompareOp::NotEqual,
        core::CompareFunction::GreaterEqual => vk::CompareOp::GreaterOrEqual,
        core::CompareFunction::Always => vk::CompareOp::Always,
    }
}
