//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::{any::Any, fmt, sync::Arc};

use zangfx::base as gfx;

use super::scheduler::PassInstantiationContext;
use crate::utils::any::AsAnySendSync;

pub type TransientResourceRef = Arc<dyn TransientResource>;

/// Represents a pass.
pub struct PassInfo<C: ?Sized> {
    pub transient_resource_uses: Vec<TransientResourceUse>,
    pub factory:
        Box<dyn FnOnce(&PassInstantiationContext) -> gfx::Result<Box<dyn Pass<C>>> + 'static>,
}

impl<C: ?Sized> fmt::Debug for PassInfo<C> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("PassInfo")
            .field("transient_resource_uses", &self.transient_resource_uses)
            .field("factory", &())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransientResourceUse {
    pub resource: TransientResourceId,

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

impl TransientResourceId {
    pub fn use_as_producer(self) -> TransientResourceUse {
        TransientResourceUse {
            resource: self,
            produce: true,
            aliasable: true,
        }
    }

    pub fn use_as_consumer(self) -> TransientResourceUse {
        TransientResourceUse {
            resource: self,
            produce: false,
            aliasable: true,
        }
    }

    pub fn use_as_non_aliasable(self) -> TransientResourceUse {
        TransientResourceUse {
            resource: self,
            produce: true,
            aliasable: false,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct TransientResourceId(pub(super) usize);

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

/// Represents a single transient resource.
pub trait TransientResource: AsAnySendSync + std::fmt::Debug {
    /// Retrieve a `ResourceBind` to be bound to a heap when a graph is
    /// instantiated.
    fn resource_bind(&self) -> Option<ResourceBind<'_>>;
}

impl dyn TransientResource {
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

/// Represents a single image transient resource.
#[derive(Debug)]
pub struct ImageTransientResource {
    pub image: gfx::ImageRef,
    pub memory_type: Option<gfx::MemoryType>,
}

impl ImageTransientResource {
    pub fn new(image: gfx::ImageRef, memory_type: Option<gfx::MemoryType>) -> Self {
        Self { image, memory_type }
    }
}

impl TransientResource for ImageTransientResource {
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

/// Represents a single buffer transient resource.
#[derive(Debug)]
pub struct BufferTransientResource {
    pub buffer: gfx::BufferRef,
    pub memory_type: Option<gfx::MemoryType>,
}

impl BufferTransientResource {
    pub fn new(buffer: gfx::BufferRef, memory_type: Option<gfx::MemoryType>) -> Self {
        Self {
            buffer,
            memory_type,
        }
    }
}

impl TransientResource for BufferTransientResource {
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
