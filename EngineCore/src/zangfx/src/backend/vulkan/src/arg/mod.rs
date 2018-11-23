//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of argument tables for Vulkan.
use arrayvec::ArrayVec;
use ash::vk;
use std::ops::{AddAssign, Index, IndexMut, Mul};

use zangfx_base as base;

pub mod layout;
pub mod pool;

fn translate_descriptor_type(ty: base::ArgType) -> vk::DescriptorType {
    use ash::vk::DescriptorType;
    use zangfx_base::ArgType;
    match ty {
        ArgType::StorageImage => DescriptorType::STORAGE_IMAGE,
        ArgType::SampledImage => DescriptorType::SAMPLED_IMAGE,
        ArgType::Sampler => DescriptorType::SAMPLER,
        ArgType::UniformBuffer => DescriptorType::UNIFORM_BUFFER,
        ArgType::StorageBuffer => DescriptorType::STORAGE_BUFFER,
    }
}

/// Maintains the number of descriptors for each descriptor type.
#[derive(Debug, Clone, Copy, Default)]
struct DescriptorCount([u32; 11]);

impl DescriptorCount {
    crate fn new() -> Self {
        Default::default()
    }

    crate fn from_bindings(bindings: &[vk::DescriptorSetLayoutBinding]) -> Self {
        let mut x = Self::new();
        x.add_bindings(bindings);
        x
    }

    crate fn add_bindings(&mut self, bindings: &[vk::DescriptorSetLayoutBinding]) -> &mut Self {
        for binding in bindings.iter() {
            self[binding.descriptor_type] += binding.descriptor_count;
        }
        self
    }

    crate fn as_pool_sizes(&self) -> ArrayVec<[vk::DescriptorPoolSize; 11]> {
        use ash::vk::DescriptorType;
        [
            DescriptorType::SAMPLER,
            DescriptorType::COMBINED_IMAGE_SAMPLER,
            DescriptorType::SAMPLED_IMAGE,
            DescriptorType::STORAGE_IMAGE,
            DescriptorType::UNIFORM_TEXEL_BUFFER,
            DescriptorType::STORAGE_TEXEL_BUFFER,
            DescriptorType::UNIFORM_BUFFER,
            DescriptorType::STORAGE_BUFFER,
            DescriptorType::UNIFORM_BUFFER_DYNAMIC,
            DescriptorType::STORAGE_BUFFER_DYNAMIC,
            DescriptorType::INPUT_ATTACHMENT,
        ].iter()
            .filter_map(|&ty| {
                let count = self[ty];
                if count > 0 {
                    Some(vk::DescriptorPoolSize {
                        ty,
                        descriptor_count: count,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Index<vk::DescriptorType> for DescriptorCount {
    type Output = u32;

    fn index(&self, index: vk::DescriptorType) -> &u32 {
        &self.0[index.as_raw() as usize]
    }
}

impl IndexMut<vk::DescriptorType> for DescriptorCount {
    fn index_mut(&mut self, index: vk::DescriptorType) -> &mut u32 {
        &mut self.0[index.as_raw() as usize]
    }
}

impl Mul<u32> for DescriptorCount {
    type Output = DescriptorCount;

    fn mul(self, rhs: u32) -> Self::Output {
        DescriptorCount([
            self.0[0] * rhs,
            self.0[1] * rhs,
            self.0[2] * rhs,
            self.0[3] * rhs,
            self.0[4] * rhs,
            self.0[5] * rhs,
            self.0[6] * rhs,
            self.0[7] * rhs,
            self.0[8] * rhs,
            self.0[9] * rhs,
            self.0[10] * rhs,
        ])
    }
}

impl AddAssign for DescriptorCount {
    fn add_assign(&mut self, rhs: Self) {
        for (x, y) in self.0.iter_mut().zip(rhs.0.iter()) {
            *x += *y;
        }
    }
}
