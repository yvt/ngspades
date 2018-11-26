//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::{any::Any, fmt, marker::PhantomData};

use zangfx::base as gfx;

use super::scheduler::{PassInstantiationContext, ResourceInstantiationContext};
use crate::utils::any::AsAnySendSync;

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

/// A strongly typed version of `ResourceId`.
///
/// This is a simple wrapper around `ResourceId` that adds concrete type
/// information.
///
/// The reason that this is defined as a type alias is to circumvent the
/// restrictions of `derive` macros that trait bounds are not generated properly
/// for tricky cases of generic types.
pub type ResourceRef<T> = ResourceRefInner<fn(T) -> T>;

/// The internal implementation of `ResourceRef`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceRefInner<T> {
    id: ResourceId,
    _phantom: PhantomData<T>,
}

impl<T> ResourceRef<T> {
    pub fn new(id: ResourceId) -> Self {
        Self {
            id,
            _phantom: PhantomData,
        }
    }

    /// Get a raw (untyped) Resource identifier.
    pub fn id(&self) -> ResourceId {
        self.id
    }
}

impl<T> std::ops::Deref for ResourceRef<T> {
    type Target = ResourceId;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

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
    /// The type of the resource object constructed by the `build` method.
    type Resource: Resource;

    /// Pre-allocate a space in an argument pool.
    ///
    /// If the implementation intends to allocate argument table(s) from
    /// `ResourceInstantiationContext::arg_pool()` when `build` is called, it
    /// must pre-allocate a space when this method is called.
    fn reserve_arg_pool(&self, _builder: &mut gfx::ArgPoolBuilderRef) {}

    /// Instantiate a transient resource.
    ///
    /// Instantiating a resource usually involves the construction of a resource
    /// object such as `ImageRef`, but does not mean it is bound to device
    /// memory. Binding resources to device memory is done by the scheduler,
    /// which calls `Resource::resource_bind` after `Resource`s are constructed.
    ///
    /// Returns a boxed `Self::Resource` on success.
    fn build(&self, context: &ResourceInstantiationContext<'_>)
        -> gfx::Result<Box<Self::Resource>>;
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
