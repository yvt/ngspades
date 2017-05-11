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

use super::{DescriptorBindingLocation, ShaderStageFlags, Sampler, ImageLayout, Resources};

pub trait PipelineLayout: Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any {}
pub trait DescriptorSetLayout: Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any {}

pub trait DescriptorPool: Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any {}

/// Handle of a descriptor set containing a set of descriptors.
///
/// Modification of a descriptor set must be synchronized or it might result in a panic.
/// A descriptor set should not be modified once it was bound to a graphics command encoder
/// until the command encoder finished the execution.
pub trait DescriptorSet<R: Resources>
    : Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any {

    /// Updates one or more descriptors in this descriptor set.
    fn update(&self, writes: &[WriteDescriptorSet<R>]);

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
    pub descriptor_set_layouts: &'a [TDescriptorSetLayout],
}

#[derive(Debug, Clone, Copy)]
pub struct DescriptorPoolDescription<'a> {
    pub max_sets: usize,
    pub pool_sizes: &'a [DescriptorPoolSize],
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
    ConstantBufferView,
    StorageBufferView,
    InputAttachment,
}

#[derive(Debug, Clone, Copy)]
pub struct WriteDescriptorSet<'a, R: Resources + 'a> {
    pub binding: DescriptorBindingLocation,
    pub start_index: usize,
    pub elements: &'a [WriteDescriptor<'a, R>],
}

#[derive(Debug, Clone, Copy)]
pub enum WriteDescriptor<'a, R: Resources> {
    StorageImage {
        image_view: &'a R::ImageView,
        image_layout: ImageLayout,
    },
    SampledImage {
        image_view: &'a R::ImageView,
        image_layout: ImageLayout,
    },
    Sampler { sampler: &'a R::Sampler },
    CombinedImageSampler {
        image_view: &'a R::ImageView,
        image_layout: ImageLayout,
        sampler: &'a R::Sampler,
    },
    ConstantBuffer {
        buffer: &'a R::Buffer,
        offset: usize,
        range: usize,
    },
    StorageBuffer {
        buffer: &'a R::Buffer,
        offset: usize,
        range: usize,
    },
    DynamicConstantBuffer {
        buffer: &'a R::Buffer,
        offset: usize,
        range: usize,
    },
    DynamicStorageBuffer {
        buffer: &'a R::Buffer,
        offset: usize,
        range: usize,
    },
    ConstantBufferView { buffer_view: &'a R::BufferView },
    StorageBufferView { buffer_view: &'a R::BufferView },
    InputAttachment {
        image_view: &'a R::ImageView,
        image_layout: ImageLayout,
    },
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
