//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use flags_macro::flags;
use std::{any::Any, fmt, sync::Arc};

use zangfx::{base as gfx, prelude::*};

use super::scheduler::{PassInstantiationContext, ResourceInstantiationContext};
use crate::utils::any::AsAnySendSync;

pub type ResourceRef = Arc<dyn Resource>;

/// Represents a pass.
pub struct PassInfo<C: ?Sized> {
    pub resource_uses: Vec<ResourceUse>,
    pub factory:
        Box<dyn FnOnce(&PassInstantiationContext) -> gfx::Result<Box<dyn Pass<C>>> + 'static>,
}

impl<C: ?Sized> fmt::Debug for PassInfo<C> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("PassInfo")
            .field("resource_uses", &self.resource_uses)
            .field("factory", &())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceUse {
    pub resource: ResourceId,

    /// Indicates whether this use indicates the production of the resource
    /// contents.
    ///
    /// For each resource, only one producing use can be defined in a graph.
    /// The producing pass is responsible for producing the contents of the
    /// resource.
    ///
    /// Cannot be `false` is `alisable` is `true`.
    pub produce: bool,

    /// Indicates whether the resource is aliasable, i.e., it may be aliased
    /// with other aliasable resources.
    pub aliasable: bool,
}

impl ResourceId {
    pub fn use_as_producer(self) -> ResourceUse {
        ResourceUse {
            resource: self,
            produce: true,
            aliasable: true,
        }
    }

    pub fn use_as_consumer(self) -> ResourceUse {
        ResourceUse {
            resource: self,
            produce: false,
            aliasable: true,
        }
    }

    pub fn use_as_non_aliasable(self) -> ResourceUse {
        ResourceUse {
            resource: self,
            produce: true,
            aliasable: false,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct ResourceId(pub(super) usize);

pub trait Pass<C: ?Sized>: std::fmt::Debug + Send + Sync {
    // TODO: Add context data?

    /// Return the number of fences updated by the commands encoded by
    /// `self.encode`.
    ///
    /// The default value is `1`. The returned value usually matches the number
    /// of output-generating command passes encoded by `encode`.
    fn num_update_fences(&self) -> usize {
        1
    }

    /// Encode commands for the corresponding pass.
    ///
    /// Command passes and commands must be encoded in accordance with the
    /// following rules:
    ///
    ///  - `CmdBuffer::invalidate_image` must be called for every produced
    ///    image before encoding a command pass that produces the image.
    ///  - Every encoded command pass must wait for the fences specified by
    ///    `wait_fences`. The access type flags must include both of read
    ///    accesses (to ensure the input is complete at the point of execution)
    ///    and write accesses (to prevent overwriting the data which is still
    ///    being read).
    ///  - Similarly, every encoded command pass must update the fence
    ///    specified by `update_fences`. `update_fences.len()` matches
    ///    `self.num_update_fences()`.
    ///
    fn encode(
        &mut self,
        cmd_buffer: &mut gfx::CmdBufferRef,
        wait_fences: &[&gfx::FenceRef],
        update_fences: &[&gfx::FenceRef],
        context: &C,
    ) -> gfx::Result<()>;
}

/// Stores information used to construct a single transient resource.
pub trait ResourceInfo: AsAnySendSync + std::fmt::Debug {
    // The type of the resource object constructed by the `build` method.
    // TODO: type Resource: Resource;

    /// Instantiate a transient resource.
    ///
    /// Instantiating a resource usually involves the construction of a resource
    /// object such as `ImageRef`, but does not mean it is bound to device
    /// memory. Binding resources to device memory is done by the scheduler,
    /// which calls `Resource::resource_bind` after `Resource`s are constructed.
    ///
    /// Returns a boxed `Self::Resource` on success.
    fn build(&self, context: &ResourceInstantiationContext<'_>) -> gfx::Result<Box<dyn Resource>>;
}

impl dyn ResourceInfo {
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        (*self).as_any().downcast_ref()
    }

    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        (*self).as_any_mut().downcast_mut()
    }
}

/// Represents a single transient resource.
pub trait Resource: AsAnySendSync + std::fmt::Debug {
    /// Retrieve a `ResourceBind` to be bound to a heap when a graph is
    /// instantiated.
    fn resource_bind(&self) -> Option<ResourceBind<'_>>;
}

impl dyn Resource {
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        (*self).as_any().downcast_ref()
    }

    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        (*self).as_any_mut().downcast_mut()
    }
}

/// A pair of a resource and a memory type the resource should be bound to.
#[derive(Debug, Clone, Copy)]
pub struct ResourceBind<'a> {
    pub resource: gfx::ResourceRef<'a>,
    pub memory_type: gfx::MemoryType,
}

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
            usage: flags![gfx::ImageUsageFlags::{Render | Sampled}],
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
    fn build(&self, context: &ResourceInstantiationContext<'_>) -> gfx::Result<Box<dyn Resource>> {
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
            usage: flags![gfx::BufferUsageFlags::{Storage}],
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
    fn build(&self, context: &ResourceInstantiationContext<'_>) -> gfx::Result<Box<dyn Resource>> {
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
