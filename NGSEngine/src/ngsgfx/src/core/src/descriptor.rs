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

use {DescriptorBindingLocation, ShaderStageFlags, Sampler, ImageLayout, Backend, Result, Marker};

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
    pub bindings: &'a [DescriptorSetLayoutBinding<'a, TSampler>],
}

#[derive(Debug, Clone, Copy)]
pub struct DescriptorSetLayoutBinding<'a, TSampler: Sampler> {
    pub location: DescriptorBindingLocation,
    pub descriptor_type: DescriptorType,
    pub num_elements: usize,
    pub stage_flags: BitFlags<ShaderStageFlags>,
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
}

#[derive(Debug, Clone, Copy)]
pub struct WriteDescriptorSet<'a, B: Backend> {
    pub start_binding: DescriptorBindingLocation,
    pub start_index: usize,
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
    pub fn descriptor_type(&self) -> DescriptorType {
        match *self {
            WriteDescriptors::StorageImage(e) => DescriptorType::StorageImage,
            WriteDescriptors::SampledImage(e) => DescriptorType::SampledImage,
            WriteDescriptors::Sampler(e) => DescriptorType::Sampler,
            WriteDescriptors::CombinedImageSampler(e) => DescriptorType::CombinedImageSampler,
            WriteDescriptors::ConstantBuffer(e) => DescriptorType::ConstantBuffer,
            WriteDescriptors::StorageBuffer(e) => DescriptorType::StorageBuffer,
            WriteDescriptors::DynamicConstantBuffer(e) => DescriptorType::DynamicConstantBuffer,
            WriteDescriptors::DynamicStorageBuffer(e) => DescriptorType::DynamicStorageBuffer,
            WriteDescriptors::InputAttachment(e) => DescriptorType::InputAttachment,
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
    pub offset: usize,
    pub range: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct CopyDescriptorSet<'a, T: 'a> {
    pub source: &'a T,
    pub source_binding: DescriptorBindingLocation,
    pub source_index: usize,
    pub destination_binding: DescriptorBindingLocation,
    pub destination_index: usize,
    pub num_elements: usize,
}
