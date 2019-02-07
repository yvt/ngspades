//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use flags_macro::flags;

use zangfx::{base as gfx, prelude::*};

use super::info::{Resource, ResourceBind, ResourceInfo};
use super::scheduler::ResourceInstantiationContext;

/// Contains information for constructing a single image transient resource.
#[derive(Debug)]
pub struct ImageResourceInfo {
    pub extents: [u32; 2],
    pub format: gfx::ImageFormat,
    pub usage: gfx::ImageUsageFlags,
}

impl ImageResourceInfo {
    pub fn new(extents: [u32; 2], format: gfx::ImageFormat) -> Self {
        Self {
            extents,
            format,
            usage: flags![gfx::ImageUsageFlags::{RENDER | SAMPLED}],
        }
    }

    pub fn with_usage(self, usage: gfx::ImageUsageFlags) -> Self {
        Self { usage, ..self }
    }

    pub fn add_usage(&mut self, usage: gfx::ImageUsageFlags) {
        self.usage |= usage;
    }
}

impl ResourceInfo for ImageResourceInfo {
    type Resource = ImageResource;

    fn build(
        &self,
        context: &ResourceInstantiationContext<'_>,
    ) -> gfx::Result<Box<Self::Resource>> {
        let image = (context.device())
            .build_image()
            .queue(context.queue())
            .extents(&self.extents)
            .format(self.format)
            .usage(self.usage)
            .build()?;

        let memory_type = (context.device())
            .try_choose_memory_type_private(&image)?
            .expect("suitable memory type was not found - this should never happen!");

        Ok(Box::new(ImageResource {
            image,
            memory_type: Some(memory_type),
        }))
    }
}

/// Represents a single image transient resource.
#[derive(Debug)]
pub struct ImageResource {
    pub image: gfx::ImageRef,
    pub memory_type: Option<gfx::MemoryType>,
}

impl ImageResource {
    pub fn new(image: gfx::ImageRef, memory_type: Option<gfx::MemoryType>) -> Self {
        Self { image, memory_type }
    }
}

impl Resource for ImageResource {
    fn resource_bind(&self) -> Option<ResourceBind<'_>> {
        if let Some(memory_type) = self.memory_type {
            Some(ResourceBind {
                resource: (&self.image).into(),
                memory_type,
            })
        } else {
            None
        }
    }
}

/// Contains information for constructing a single buffer transient resource.
#[derive(Debug)]
pub struct BufferResourceInfo {
    pub size: gfx::DeviceSize,
    pub usage: gfx::BufferUsageFlags,
}

impl BufferResourceInfo {
    pub fn new(size: gfx::DeviceSize) -> Self {
        Self {
            size,
            usage: flags![gfx::BufferUsageFlags::{STORAGE}],
        }
    }

    pub fn with_usage(self, usage: gfx::BufferUsageFlags) -> Self {
        Self { usage, ..self }
    }

    pub fn add_usage(&mut self, usage: gfx::BufferUsageFlags) {
        self.usage |= usage;
    }
}

impl ResourceInfo for BufferResourceInfo {
    type Resource = BufferResource;

    fn build(
        &self,
        context: &ResourceInstantiationContext<'_>,
    ) -> gfx::Result<Box<Self::Resource>> {
        let buffer = (context.device())
            .build_buffer()
            .queue(context.queue())
            .size(self.size)
            .usage(self.usage)
            .build()?;

        let memory_type = (context.device())
            .try_choose_memory_type_private(&buffer)?
            .expect("suitable memory type was not found - this should never happen!");

        Ok(Box::new(BufferResource {
            buffer,
            memory_type: Some(memory_type),
        }))
    }
}

/// Represents a single buffer transient resource.
#[derive(Debug)]
pub struct BufferResource {
    pub buffer: gfx::BufferRef,
    pub memory_type: Option<gfx::MemoryType>,
}

impl BufferResource {
    pub fn new(buffer: gfx::BufferRef, memory_type: Option<gfx::MemoryType>) -> Self {
        Self {
            buffer,
            memory_type,
        }
    }
}

impl Resource for BufferResource {
    fn resource_bind(&self) -> Option<ResourceBind<'_>> {
        if let Some(memory_type) = self.memory_type {
            Some(ResourceBind {
                resource: (&self.buffer).into(),
                memory_type,
            })
        } else {
            None
        }
    }
}

/// Contains information for constructing a single argument table transient
/// resource.
#[derive(Debug)]
pub struct ArgTableResourceInfo {
    pub sig: gfx::ArgTableSigRef,
    pub count: usize,
}

impl ArgTableResourceInfo {
    pub fn new(sig: gfx::ArgTableSigRef) -> Self {
        Self { sig, count: 1 }
    }

    pub fn with_count(self, count: usize) -> Self {
        Self { count, ..self }
    }
}

impl ResourceInfo for ArgTableResourceInfo {
    type Resource = ArgTableResource;

    fn reserve_arg_pool(&self, builder: &mut gfx::ArgPoolBuilderRef) {
        builder.reserve_table_sig(self.count, &self.sig);
    }

    fn build(
        &self,
        context: &ResourceInstantiationContext<'_>,
    ) -> gfx::Result<Box<Self::Resource>> {
        let tables = context
            .arg_pool()
            .new_tables(self.count, &self.sig)?
            .expect("arg table allocation failed");

        Ok(Box::new(ArgTableResource {
            pool: context.arg_pool().clone(),
            tables,
        }))
    }
}

/// Represents a single argument table resource.
///
/// Although it's labeled as a transient resource, it's in fact intended to be
/// used as something like an immutable resource -- each of them is to be
/// associated with only one pass (as a produced resource) and written only
/// once when the pass is instantiated.
#[derive(Debug)]
pub struct ArgTableResource {
    pub pool: gfx::ArgPoolRef,
    pub tables: Vec<gfx::ArgTableRef>,
}

impl ArgTableResource {
    pub fn new(pool: gfx::ArgPoolRef, tables: Vec<gfx::ArgTableRef>) -> Self {
        Self { pool, tables }
    }
}

impl Resource for ArgTableResource {
    fn resource_bind(&self) -> Option<ResourceBind<'_>> {
        None
    }
}
