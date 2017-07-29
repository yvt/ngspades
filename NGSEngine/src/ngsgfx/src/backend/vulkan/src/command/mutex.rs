//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Tracks the duration of resources being used by the device and prevents
//! host write accesses to them which are still in use.
//! Moreover, retains a reference to them and prevents them from being
//! destroyed as long as they are still in use.
use ash::vk;
use ash::version::DeviceV1_0;
use std::collections::HashMap;
use std::sync::{Mutex, Arc, Weak};
use std::{mem, fmt, marker, ops};
use std::sync::atomic::Ordering;

use ngsgfx_common::barc::{BArc, BArcBox, BWeak};
use ngsgfx_common::atom2::AtomicArc;

use super::tokenlock::{TokenLock, Token};
use {RefEqArc, DeviceRef, AshDevice};

pub(crate) trait GetResourceFenceDependencyTable<T>
    : fmt::Debug + ResourceFence + marker::Sized {
    fn get_dependency_table<'a: 'b, 'b>(
        &'a self,
        token: &'b mut Token,
    ) -> &'b mut ResourceFenceDependencyTable<Self, T>;
}

/// Manages resources that are still in use by the device.
///
/// Users must not drop a `ResourceFenceDependencyTable` until the device's operation is
/// completed and `ResourceFenceDependencyTable::clear` is called.
#[derive(Debug)]
pub(crate) struct ResourceFenceDependencyTable<F: ResourceFence + GetResourceFenceDependencyTable<T>, T>(
    HashMap<RefEqArc<()>, Arc<ResourceMutexDeviceRef<F, T>>>,
);

impl<F: ResourceFence + GetResourceFenceDependencyTable<T>, T> ResourceFenceDependencyTable<F, T> {
    pub fn new() -> Self {
        ResourceFenceDependencyTable(HashMap::new())
    }

    /// Declare that resources associated with this fence are no longer used
    /// by the device.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

impl<F: ResourceFence + GetResourceFenceDependencyTable<T>, T> Drop
    for ResourceFenceDependencyTable<F, T> {
    fn drop(&mut self) {
        assert!(self.0.len() == 0);
    }
}

pub(crate) trait ResourceFence: fmt::Debug {
    /// Check the state of the Vulkan fence. If it is in the signalled state,
    /// release all references by calling `self`'s
    /// `ResourceFenceDependencyTable::clear`.
    fn check_fence(&self);
}

/// A reference to resource.
#[derive(Debug)]
pub(crate) struct ResourceMutexRef<F: ResourceFence + GetResourceFenceDependencyTable<T>, T>(BArc<ResourceMutexData<F, T>>);

impl<F: ResourceFence + GetResourceFenceDependencyTable<T>, T> ResourceMutexRef<F, T> {
    fn id(&self) -> &RefEqArc<()> {
        &self.0.id
    }

    /// Associate this resource with the specified fence.
    ///
    /// If this resource was previously associated with another fence, the new
    /// fence must be signalled after, or must be the same as the previous one.
    pub fn update_fence(&self, fence_token: &mut Token, fence: &Arc<F>) {
        let table_lg = fence.get_dependency_table(fence_token);
        let ref mut table: ResourceFenceDependencyTable<F, T> = *table_lg;
        if table.0.contains_key(&self.0.id) {
            return;
        }

        let mut device_ref_cell = self.0.device_ref.lock().unwrap();

        // Dissociate with the previous one
        if let Some(prev_device_ref) = device_ref_cell.upgrade() {
            prev_device_ref.0.take(Ordering::Relaxed);
        }

        // Create a new device reference
        let dr = ResourceMutexDeviceRef(
            AtomicArc::new(Some(BArc::clone(&self.0))),
            Arc::downgrade(fence),
        );
        let dr_arc = Arc::new(dr);
        let dr_weak = Arc::downgrade(&dr_arc);
        table.0.insert(self.0.id.clone(), dr_arc);

        *device_ref_cell = dr_weak;
    }
}

impl<F: ResourceFence + GetResourceFenceDependencyTable<T>, T> Clone for ResourceMutexRef<F, T> {
    fn clone(&self) -> Self {
        ResourceMutexRef(self.0.clone())
    }
}

/// A reference to resource from a device (fence).
#[derive(Debug)]
pub(crate) struct ResourceMutexDeviceRef<F: ResourceFence + GetResourceFenceDependencyTable<T>, T>(
    AtomicArc<BArc<ResourceMutexData<F, T>>>,
    Weak<F>,
);

#[derive(Debug)]
pub(crate) struct ResourceMutexData<F: ResourceFence + GetResourceFenceDependencyTable<T>, T> {
    id: RefEqArc<()>,

    /// The inner value.
    ///
    /// Must be `Some(_)`. (This has to be `Option` to implement `Drop`)
    data: Option<BArcBox<T>>,

    /// `ResourceMutexDeviceRef` that own a reference to `self`.
    device_ref: Mutex<Weak<ResourceMutexDeviceRef<F, T>>>,
}

derive_using_field! {
    (F: ResourceFence + GetResourceFenceDependencyTable<T>, T); (PartialEq, Eq, Hash) for ResourceMutexData<F, T> => id
}

impl<F: ResourceFence + GetResourceFenceDependencyTable<T>, T> ResourceMutexData<F, T> {
    fn data(&self) -> &T {
        self.data.as_ref().unwrap()
    }

    fn data_mut(&mut self) -> &mut T {
        self.data.as_mut().unwrap()
    }
}

/// Represents an object that must not be accessed (in some means) by the host
/// and device at the same time.
///
/// Moreover, ensures the wrapped object is alive as long as it is being used by
/// the device.
///
/// The examples include:
///
///  - Descriptor sets
///  - Command buffers
///
/// Access Types
/// ------------
///
/// The following three access types are defined:
///
///  - **Host read** - This type of access can be started via `get_host_read`.
///    The access is considered active until the returned reference is dropped.
///    This only requires an immutable reference to the `ResourceMutex`.
///
///  - **Host write** - This type of access can be started via `try_lock_host_write`
///    or `lock_host_write`.
///    The access is considered active until the returned reference is dropped.
///    This requires a mutable reference to the `ResourceMutex`. Because of this,
///    the *host write* access cannot occur at the same time as the *host read*
///    access. If an attempt was made to start the *host write* access while
///    the *device read* access is active, a panic will occur.
///
///  - **Device read** - This type of access can be started via `lock_device`.
///    The duration of access is tracked using a fence.
///    TODO: how to associate it with a fence
///
#[derive(Debug)]
pub(crate) struct ResourceMutex<F: ResourceFence + GetResourceFenceDependencyTable<T>, T>(ResourceMutexState<F, T>);

#[derive(Debug)]
enum ResourceMutexState<F: ResourceFence + GetResourceFenceDependencyTable<T>, T> {
    /// The device might be accessing the object.
    Limbo(ResourceMutexRef<F, T>),

    /// A full host accessbility is guaranteed.
    Owned(BArcBox<ResourceMutexData<F, T>>),

    Invalid,
}

impl<F: ResourceFence + GetResourceFenceDependencyTable<T>, T> ResourceMutex<F, T> {
    pub fn new(x: T) -> Self {
        let data = ResourceMutexData {
            id: RefEqArc::new(()),
            data: Some(BArcBox::new(x)),
            device_ref: Mutex::new(Weak::new()),
        };
        ResourceMutex(ResourceMutexState::Owned(BArcBox::new(data)))
    }

    /// Acquire a host read accessibility to the inner value and return it.
    ///
    /// This always succeeds.
    pub fn get_host_read(&self) -> &T {
        match self.0 {
            ResourceMutexState::Owned(ref data) => data.data(),
            ResourceMutexState::Limbo(ResourceMutexRef(ref data)) => data.data(),
            ResourceMutexState::Invalid => unreachable!(),
        }
    }

    /// Acquire a host write accessbility to the inner value and return it.
    ///
    /// ## Panics
    ///
    /// Panics if it is still being accessed by the device.
    pub fn lock_host_write(&mut self) -> &mut T {
        self.try_lock_host_write().expect(
            "cannot acquire a host write access permission because \
            the device might be still accessing it",
        )
    }

    /// Attempts to acquire a host write accessbility to the inner value.
    /// If it succeeds, returns the inner value.
    pub fn try_lock_host_write(&mut self) -> Option<&mut T> {
        let data_box = match self.0 {
            ResourceMutexState::Owned(ref mut data) => {
                return Some(data.data_mut());
            }
            ResourceMutexState::Limbo(_) => {
                // Cannot move the value out without leaving this block
            }
            ResourceMutexState::Invalid => unreachable!(),
        };

        // Take `limbo_arc`
        let next_state = match mem::replace(&mut self.0, ResourceMutexState::Invalid) {
            ResourceMutexState::Limbo(ResourceMutexRef(data_arc)) => {
                // There must not be a weak reference of this.
                if BArc::strong_count(&data_arc) > 1 {
                    // Try `ResourceFence::check_fence` first to make a fence relinquish the ownership
                    let device_ref_cell = data_arc.device_ref.lock().unwrap();
                    if let Some(device_ref) = device_ref_cell.upgrade() {
                        if let Some(fence) = device_ref.1.upgrade() {
                            fence.check_fence();
                        }
                    }
                }

                match BArc::try_into_box(data_arc) {
                    Ok(data_box) => {
                        // Successfully acquired the accessbility.

                        ResourceMutexState::Owned(data_box)
                    }
                    Err(data_arc) => ResourceMutexState::Limbo(ResourceMutexRef(data_arc)),
                }
            }
            _ => unreachable!(),
        };
        self.0 = next_state;

        match self.0 {
            ResourceMutexState::Owned(ref mut data) => Some(data.data_mut()),
            ResourceMutexState::Limbo(_) => None,
            _ => unreachable!(),
        }
    }

    /// Acquire a device read accessbility to the inner value and return it.
    ///
    /// The device accessbility lasts until the execution of command buffers
    /// associated with the given fence complete.
    pub fn lock_device(&mut self) -> &ResourceMutexRef<F, T> {
        match self.0 {
            ResourceMutexState::Owned(_) => {
                // We can't take the data in this `match` block
                // So first we need to leave from it
            }
            ResourceMutexState::Limbo(ref data_ref) => {
                return data_ref;
            }
            ResourceMutexState::Invalid => unreachable!(),
        }

        // Take value
        let data_box = match mem::replace(&mut self.0, ResourceMutexState::Invalid) {
            ResourceMutexState::Owned(data_box) => data_box,
            _ => unreachable!(),
        };
        let data_arc = BArcBox::into_arc(data_box);
        self.0 = ResourceMutexState::Limbo(ResourceMutexRef(data_arc));

        match self.0 {
            ResourceMutexState::Limbo(ref data_ref) => data_ref,
            _ => unreachable!(),
        }
    }
}
