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

use {DescriptorBindingLocation, ShaderStageFlags, Sampler, ImageLayout, Backend, Result, Marker,
     DescriptorBindingElementIndex, DeviceSize, Validate, DeviceCapabilities};

pub trait PipelineLayout
    : Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any + Marker {
}
pub trait DescriptorSetLayout
    : Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any + Marker {
}

// TODO: make this like `Heap`
/// Represents a descriptor pool that descriptor sets are allocated from.
///
/// Sets allocated from a pool hold a reference to the underlying storage of the pool.
/// Deallocating the `Allocation` associated to a set puts the set into the Invalid state
/// where the set no longer holds a reference to a heap. Attempt to putting a set that
/// is potentially being used by the device into the Invalid state will result in a panic.
pub trait DescriptorPool<B: Backend>: Debug + Send + Any {
    /// Represents an allocated region. Can outlive the parent `MappableHeap`.
    /// Dropping this will leak memory (useful for permanent allocations).
    type Allocation: Hash + Debug + Eq + PartialEq + Send + Any;

    /// Deallocates a region. `allocation` must orignate from the same `Heap`.
    ///
    /// Does nothing if `allocation` is already deallocated.
    fn deallocate(&mut self, allocation: &mut Self::Allocation);

    fn make_descriptor_set(
        &mut self,
        description: &DescriptorSetDescription<B::DescriptorSetLayout>,
    ) -> Result<Option<(B::DescriptorSet, Self::Allocation)>>;

    fn reset(&mut self);
}

/// Handle for descriptor sets containing a set of descriptors.
///
/// Modification of a descriptor set must be synchronized or it might result in a panic.
/// A descriptor set should not be modified once it was bound to a graphics command encoder
/// until the command encoder finished the execution.
pub trait DescriptorSet<B: Backend>
    : Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any {
    /// Updates one or more descriptors in this descriptor set.
    fn update(&self, writes: &[WriteDescriptorSet<B>]);

    /// Copies one or more descriptors to this descriptor set.
    fn copy_from(&self, copies: &[CopyDescriptorSet<Self>]);
}

#[derive(Debug, Clone, Copy)]
pub struct DescriptorSetLayoutDescription<'a, TSampler: Sampler> {
    /// Descriptor bindings.
    ///
    /// - If there is at least one binding, then there must not be a free binding
    ///   location `i <= max_location` where
    ///   `max_location == bindings.iter().map(|x| x.location).max().unwrap()`.
    /// - The elements' `location`s must be unique.
    pub bindings: &'a [DescriptorSetLayoutBinding<'a, TSampler>],
}

#[derive(Debug, Clone, Copy)]
pub struct DescriptorSetLayoutBinding<'a, TSampler: Sampler> {
    pub location: DescriptorBindingLocation,
    pub descriptor_type: DescriptorType,
    pub num_elements: usize,
    pub stage_flags: ShaderStageFlags,
    pub immutable_samplers: Option<&'a [&'a TSampler]>,
}

#[derive(Debug, Clone, Copy)]
pub struct PipelineLayoutDescription<'a, TDescriptorSetLayout: DescriptorSetLayout> {
    pub descriptor_set_layouts: &'a [&'a TDescriptorSetLayout],
}

#[derive(Debug, Clone, Copy)]
pub struct DescriptorPoolDescription<'a> {
    pub max_num_sets: usize,
    pub pool_sizes: &'a [DescriptorPoolSize],

    /// Specifies whether deallocating descriptor sets is supported.
    /// If this is set to `false`, resetting a pool will be the only way to reclaim a free space.
    pub supports_deallocation: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct DescriptorPoolSize {
    pub descriptor_type: DescriptorType,
    pub num_descriptors: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct DescriptorSetDescription<'a, TDescriptorSetLayout: DescriptorSetLayout> {
    pub layout: &'a TDescriptorSetLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DescriptorType {
    StorageImage,
    SampledImage,
    Sampler,
    CombinedImageSampler,
    ConstantBuffer,
    StorageBuffer,
    DynamicConstantBuffer,
    DynamicStorageBuffer,
    InputAttachment,
}

impl DescriptorType {
    pub fn has_image_view(&self) -> bool {
        match *self {
            DescriptorType::StorageImage => true,
            DescriptorType::SampledImage => true,
            DescriptorType::Sampler => false,
            DescriptorType::CombinedImageSampler => true,
            DescriptorType::ConstantBuffer => false,
            DescriptorType::StorageBuffer => false,
            DescriptorType::DynamicConstantBuffer => false,
            DescriptorType::DynamicStorageBuffer => false,
            DescriptorType::InputAttachment => true,
        }
    }

    pub fn has_sampler(&self) -> bool {
        match *self {
            DescriptorType::StorageImage => false,
            DescriptorType::SampledImage => false,
            DescriptorType::Sampler => true,
            DescriptorType::CombinedImageSampler => true,
            DescriptorType::ConstantBuffer => false,
            DescriptorType::StorageBuffer => false,
            DescriptorType::DynamicConstantBuffer => false,
            DescriptorType::DynamicStorageBuffer => false,
            DescriptorType::InputAttachment => false,
        }
    }

    pub fn has_buffer(&self) -> bool {
        match *self {
            DescriptorType::StorageImage => false,
            DescriptorType::SampledImage => false,
            DescriptorType::Sampler => false,
            DescriptorType::CombinedImageSampler => false,
            DescriptorType::ConstantBuffer => true,
            DescriptorType::StorageBuffer => true,
            DescriptorType::DynamicConstantBuffer => true,
            DescriptorType::DynamicStorageBuffer => true,
            DescriptorType::InputAttachment => false,
        }
    }

    pub fn needs_dynamic_offsets(&self) -> bool {
        match *self {
            DescriptorType::StorageImage => false,
            DescriptorType::SampledImage => false,
            DescriptorType::Sampler => false,
            DescriptorType::CombinedImageSampler => false,
            DescriptorType::ConstantBuffer => false,
            DescriptorType::StorageBuffer => false,
            DescriptorType::DynamicConstantBuffer => true,
            DescriptorType::DynamicStorageBuffer => true,
            DescriptorType::InputAttachment => false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WriteDescriptorSet<'a, B: Backend> {
    pub start_binding: DescriptorBindingLocation,
    pub start_index: DescriptorBindingElementIndex,
    pub elements: WriteDescriptors<'a, B>,
}

#[derive(Debug)]
pub enum WriteDescriptors<'a, B: Backend> {
    StorageImage(&'a [DescriptorImage<'a, B>]),
    SampledImage(&'a [DescriptorImage<'a, B>]),
    Sampler(&'a [&'a B::Sampler]),
    CombinedImageSampler(&'a [(DescriptorImage<'a, B>, &'a B::Sampler)]),
    ConstantBuffer(&'a [DescriptorBuffer<'a, B>]),
    StorageBuffer(&'a [DescriptorBuffer<'a, B>]),
    DynamicConstantBuffer(&'a [DescriptorBuffer<'a, B>]),
    DynamicStorageBuffer(&'a [DescriptorBuffer<'a, B>]),
    InputAttachment(&'a [DescriptorImage<'a, B>]),
}

impl<'a, B: Backend> WriteDescriptors<'a, B> {
    pub fn len(&self) -> usize {
        match *self {
            WriteDescriptors::StorageImage(x) => x.len(),
            WriteDescriptors::SampledImage(x) => x.len(),
            WriteDescriptors::Sampler(x) => x.len(),
            WriteDescriptors::CombinedImageSampler(x) => x.len(),
            WriteDescriptors::ConstantBuffer(x) => x.len(),
            WriteDescriptors::StorageBuffer(x) => x.len(),
            WriteDescriptors::DynamicConstantBuffer(x) => x.len(),
            WriteDescriptors::DynamicStorageBuffer(x) => x.len(),
            WriteDescriptors::InputAttachment(x) => x.len(),
        }
    }

    pub fn descriptor_type(&self) -> DescriptorType {
        match *self {
            WriteDescriptors::StorageImage(_) => DescriptorType::StorageImage,
            WriteDescriptors::SampledImage(_) => DescriptorType::SampledImage,
            WriteDescriptors::Sampler(_) => DescriptorType::Sampler,
            WriteDescriptors::CombinedImageSampler(_) => DescriptorType::CombinedImageSampler,
            WriteDescriptors::ConstantBuffer(_) => DescriptorType::ConstantBuffer,
            WriteDescriptors::StorageBuffer(_) => DescriptorType::StorageBuffer,
            WriteDescriptors::DynamicConstantBuffer(_) => DescriptorType::DynamicConstantBuffer,
            WriteDescriptors::DynamicStorageBuffer(_) => DescriptorType::DynamicStorageBuffer,
            WriteDescriptors::InputAttachment(_) => DescriptorType::InputAttachment,
        }
    }
}

// #[derive(Clone, Copy)] does not work as intended on WriteDescriptors currently
// due to: https://github.com/rust-lang/rust/issues/26925
impl<'a, B: Backend> Clone for WriteDescriptors<'a, B> {
    fn clone(&self) -> Self {
        match *self {
            WriteDescriptors::StorageImage(e) => WriteDescriptors::StorageImage(e),
            WriteDescriptors::SampledImage(e) => WriteDescriptors::SampledImage(e),
            WriteDescriptors::Sampler(e) => WriteDescriptors::Sampler(e),
            WriteDescriptors::CombinedImageSampler(e) => WriteDescriptors::CombinedImageSampler(e),
            WriteDescriptors::ConstantBuffer(e) => WriteDescriptors::ConstantBuffer(e),
            WriteDescriptors::StorageBuffer(e) => WriteDescriptors::StorageBuffer(e),
            WriteDescriptors::DynamicConstantBuffer(e) => WriteDescriptors::DynamicConstantBuffer(
                e,
            ),
            WriteDescriptors::DynamicStorageBuffer(e) => WriteDescriptors::DynamicStorageBuffer(e),
            WriteDescriptors::InputAttachment(e) => WriteDescriptors::InputAttachment(e),
        }
    }
}

impl<'a, B: Backend> Copy for WriteDescriptors<'a, B> {}

#[derive(Debug, Clone, Copy)]
pub struct DescriptorImage<'a, B: Backend> {
    pub image_view: &'a B::ImageView,
    pub image_layout: ImageLayout,
}

#[derive(Debug, Clone, Copy)]
pub struct DescriptorBuffer<'a, B: Backend> {
    pub buffer: &'a B::Buffer,
    pub offset: DeviceSize,
    pub range: DeviceSize,
}

#[derive(Debug, Clone, Copy)]
pub struct CopyDescriptorSet<'a, T: 'a> {
    pub source: &'a T,
    pub source_binding: DescriptorBindingLocation,
    pub source_index: DescriptorBindingElementIndex,
    pub destination_binding: DescriptorBindingLocation,
    pub destination_index: DescriptorBindingElementIndex,
    pub num_elements: usize,
}

/// Validation errors for [`DescriptorPoolDescription`](struct.DescriptorPoolDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum DescriptorPoolDescriptionValidationError {
    // TODO
}

impl<'a> Validate for DescriptorPoolDescription<'a> {
    type Error = DescriptorPoolDescriptionValidationError;

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
    where
        T: FnMut(Self::Error) -> (),
    {
        // TODO
    }
}

/// Validation errors for [`DescriptorSetDescription`](struct.DescriptorSetDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum DescriptorSetDescriptionValidationError {
}

impl<'a, TDescriptorSetLayout: DescriptorSetLayout> Validate
    for DescriptorSetDescription<'a, TDescriptorSetLayout> {
    type Error = DescriptorPoolDescriptionValidationError;

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
    where
        T: FnMut(Self::Error) -> (),
    {
        // There is nothing we can check here
    }
}

/// Validation errors for [`PipelineLayoutDescription`](struct.PipelineLayoutDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum PipelineLayoutDescriptionValidationError {
}

impl<'a, TDescriptorSetLayout: DescriptorSetLayout> Validate
    for PipelineLayoutDescription<'a, TDescriptorSetLayout> {
    type Error = PipelineLayoutDescriptionValidationError;

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
    where
        T: FnMut(Self::Error) -> (),
    {
        // There is nothing we can check here
    }
}

/// Validation errors for [`DescriptorSetLayoutDescription`](struct.DescriptorSetLayoutDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum DescriptorSetLayoutDescriptionValidationError {
    /// There is at least one free binding location `i < max_location` where
    /// `self.bindings.iter().map(|x| x.location).max().unwrap()`.
    FreeBindingLocation,

    /// One of the binding location is associated with more than one element of
    /// `self.bindings`.
    NonUniqueLocation,
}

impl<'a, TSampler: Sampler> Validate for DescriptorSetLayoutDescription<'a, TSampler> {
    type Error = DescriptorSetLayoutDescriptionValidationError;

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
    where
        T: FnMut(Self::Error) -> (),
    {
        if let Some(max_loc) = self.bindings.iter().map(|x| x.location).max() {
            let mut table = vec![false; max_loc + 1];
            let mut nul_reported = false;
            for &DescriptorSetLayoutBinding { location, .. } in self.bindings.iter() {
                if table[location] && !nul_reported {
                    callback(
                        DescriptorSetLayoutDescriptionValidationError::NonUniqueLocation,
                    );
                    nul_reported = true;
                }
                table[location] = true;
            }
            for &e in table.iter() {
                if !e {
                    callback(
                        DescriptorSetLayoutDescriptionValidationError::FreeBindingLocation,
                    );
                }
            }
        }

        // TODO: more checks?
    }
}
