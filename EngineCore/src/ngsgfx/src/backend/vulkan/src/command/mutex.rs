//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Tracks the duration of resources being used by the device and prevents
//! host write accesses to them which are still in use.
//! Moreover, retains a reference to them and prevents them from being
//! destroyed as long as they are still in use.
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use std::{mem, fmt, ptr};
use std::sync::atomic::Ordering;
use parking_lot::Mutex;

use ngsgfx_common::barc::{BArc, BArcBox};
use ngsgfx_common::atom2::AtomicArc;

use RefEqArc;

/// Manages resources that are still in use by the device.
///
/// Users must not drop a `ResourceFenceDependencyTable` until the device's operation is
/// completed and `ResourceFenceDependencyTable::clear` is called.
#[derive(Debug)]
pub(crate) struct ResourceFenceDependencyTable<F: ResourceFence, T>(HashMap<RefEqArc<()>, Arc<ResourceMutexDeviceRef<F, T>>>);

impl<F: ResourceFence, T> ResourceFenceDependencyTable<F, T> {
    pub fn new() -> Self {
        ResourceFenceDependencyTable(HashMap::new())
    }

    /// Declare that resources associated with this fence are no longer used
    /// by the device.
    pub fn clear(&mut self, fence: Option<&F>) {
        for (_, dr) in self.0.drain() {
            let fa: Option<Arc<FenceAccessor<F>>> = dr.1.upgrade();
            if let Some(fa) = fa {
                let mut fence_cell = fa.fence.lock();
                if let Some(fence) = fence {
                    if fence_cell.as_ref().map(
                        |fence_arc| ptr::eq(&**fence_arc, fence),
                    ) == Some(true)
                    {
                        *fence_cell = None;
                    }
                }
            }
        }
    }

    /// Associate the resource with this fence.
    ///
    /// If the resource is currently associated with another fence, it must be
    /// signalled before this fence.
    pub fn insert(&mut self, fence: &Arc<F>, mut dr: ResourceMutexDeviceRef<F, T>) {
        let data: BArc<ResourceMutexData<_, _>> = dr.0.load().unwrap();
        let f_accessor = dr.1.upgrade();
        let dr_arc = Arc::new(dr);
        let dr_weak = Arc::downgrade(&dr_arc);
        self.0.insert(Clone::clone(&data.id), dr_arc);

        if let Some(f_accessor) = f_accessor {
            let mut fence_cell = f_accessor.fence.lock();
            *fence_cell = Some(fence.clone());
        }

        let old_dr_weak = data.device_ref.swap(Some(dr_weak), Ordering::Relaxed);
        if let Some(old_dr) = old_dr_weak.and_then(|w| w.upgrade()) {
            old_dr.0.take(Ordering::Relaxed);
        }
    }
}

impl<F: ResourceFence, T> Drop for ResourceFenceDependencyTable<F, T> {
    fn drop(&mut self) {
        assert!(self.0.len() == 0);
    }
}

pub(crate) trait ResourceFence: fmt::Debug {
    /// Check the state of the Vulkan fence. If it is in the signalled state,
    /// release all references by calling `self`'s
    /// `ResourceFenceDependencyTable::clear`.
    fn check_fence(&self, wait: bool);
}

/// A reference to resource from a device (fence).
#[derive(Debug)]
pub(crate) struct ResourceMutexDeviceRef<F: ResourceFence, T>(
    AtomicArc<BArc<ResourceMutexData<F, T>>>,
    Weak<FenceAccessor<F>>
);

#[derive(Debug, Default)]
struct FenceAccessor<F: ResourceFence> {
    fence: Mutex<Option<Arc<F>>>,
}

impl<F: ResourceFence> FenceAccessor<F> {
    fn new() -> Self {
        Self { fence: Mutex::new(None) }
    }
}

#[derive(Debug)]
struct ResourceMutexData<F: ResourceFence, T> {
    id: RefEqArc<()>,

    /// The inner value.
    ///
    /// Must be `Some(_)`. (This has to be `Option` to implement `Drop`)
    data: Option<BArcBox<T>>,

    /// `ResourceMutexDeviceRef` that own a reference to `self`.
    device_ref: AtomicArc<Weak<ResourceMutexDeviceRef<F, T>>>,
}

derive_using_field! {
    (F: ResourceFence, T); (PartialEq, Eq, Hash) for ResourceMutexData<F, T> => id
}

impl<F: ResourceFence, T> ResourceMutexData<F, T> {
    #[allow(dead_code)]
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
pub(crate) struct ResourceMutex<F: ResourceFence, T>(
    ResourceMutexState<F, T>,
    Option<Arc<FenceAccessor<F>>>
);

#[derive(Debug)]
enum ResourceMutexState<F: ResourceFence, T> {
    /// The device might be accessing the object.
    Limbo(BArc<ResourceMutexData<F, T>>),

    /// A full host accessbility is guaranteed.
    Owned(BArcBox<ResourceMutexData<F, T>>),

    Invalid,
}

impl<F: ResourceFence, T> ResourceMutex<F, T> {
    pub fn new(x: T, mutable: bool) -> Self {
        let data = ResourceMutexData {
            id: RefEqArc::new(()),
            data: Some(BArcBox::new(x)),
            device_ref: AtomicArc::new(Some(Weak::new())),
        };
        ResourceMutex(
            if mutable {
                ResourceMutexState::Owned(BArcBox::new(data))
            } else {
                ResourceMutexState::Limbo(BArc::new(data))
            },
            if mutable {
                Some(Arc::new(FenceAccessor::new()))
            } else {
                None
            },
        )
    }

    /// Deny further host write accesses.
    #[allow(dead_code)]
    pub fn make_immutable(&mut self) {
        self.1 = None;

        match self.0 {
            ResourceMutexState::Owned(_) => {
                // We can't take the data in this `match` block
                // So first we need to leave from it
            }
            ResourceMutexState::Limbo(_) => {
                return;
            }
            ResourceMutexState::Invalid => unreachable!(),
        }

        // Take value
        let data_box = match mem::replace(&mut self.0, ResourceMutexState::Invalid) {
            ResourceMutexState::Owned(data_box) => data_box,
            _ => unreachable!(),
        };
        let data_arc = BArcBox::into_arc(data_box);
        self.0 = ResourceMutexState::Limbo(data_arc);
    }

    /// Acquire a host read accessibility to the inner value and return it.
    ///
    /// This always succeeds.
    #[allow(dead_code)]
    pub fn get_host_read(&self) -> &T {
        match self.0 {
            ResourceMutexState::Owned(ref data) => data.data(),
            ResourceMutexState::Limbo(ref data) => data.data(),
            ResourceMutexState::Invalid => unreachable!(),
        }
    }

    /// Acquire a host write accessbility to the inner value and return it.
    ///
    /// ## Panics
    ///
    /// Panics if it is still being accessed by the device.
    pub fn lock_host_write(&mut self) -> &mut T {
        // Actually, this error message is inaccurate becuase a lock failure
        // can occur for other reasons including:
        //  - The resource is marked as immutable.
        self.try_lock_host_write(false).expect(
            "cannot acquire a host write access permission because \
            the device might be still accessing it",
        )
    }

    /// Attempts to acquire a host write accessbility to the inner value.
    /// If it succeeds, returns the inner value.
    ///
    /// If `wait` is set to `true` and the resource is currently being accessed
    /// by the device, the current thread will be suspended until the resouce
    /// is available for a host write access. Even if `wait` is set to `true`,
    /// this method might return `None` immediately if this resource is not
    /// associated to any fences yet.
    pub fn try_lock_host_write(&mut self, wait: bool) -> Option<&mut T> {
        match self.0 {
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
            ResourceMutexState::Limbo(data_arc) => {
                // There must not be another strong reference to this to perform a host write.
                if BArc::strong_count(&data_arc) > 1 {
                    // Try `ResourceFence::check_fence` first to make a fence relinquish the ownership
                    if let Some(accessor) = self.1.as_ref() {
                        let fence = accessor.fence.lock().clone();
                        if let Some(fence) = fence {
                            fence.check_fence(wait);
                        }
                    }
                }

                match BArc::try_into_box(data_arc) {
                    Ok(data_box) => {
                        // Successfully acquired the accessbility.

                        ResourceMutexState::Owned(data_box)
                    }
                    Err(data_arc) => ResourceMutexState::Limbo(data_arc),
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

    pub fn is_host_writable(&self) -> bool {
        match self.0 {
            ResourceMutexState::Owned(_) => true,
            ResourceMutexState::Limbo(ref data_arc) => {
                // There must not be another strong reference to this to perform a host write.
                if BArc::strong_count(data_arc) > 1 {
                    // Try `ResourceFence::check_fence` first to make a fence relinquish the ownership
                    if let Some(accessor) = self.1.as_ref() {
                        let fence = accessor.fence.lock().clone();
                        if let Some(fence) = fence {
                            fence.check_fence(false);
                        }
                    }
                }

                BArc::strong_count(&data_arc) == 1
            }
            ResourceMutexState::Invalid => unreachable!(),
        }
    }

    pub fn wait_host_writable(&self) {
        if let Some(accessor) = self.1.as_ref() {
            let fence = accessor.fence.lock().clone();
            if let Some(fence) = fence {
                fence.check_fence(true);
            }
        }
    }

    /// Acquire a device read accessbility to the inner value and return it.
    ///
    /// The device accessbility lasts until the execution of command buffers
    /// associated with the given fence complete.
    pub fn lock_device(&mut self) -> ResourceMutexDeviceRef<F, T> {
        match self.0 {
            ResourceMutexState::Owned(_) => {
                // We can't take the data in this `match` block
                // So first we need to leave from it
            }
            ResourceMutexState::Limbo(ref data_ref) => {
                return ResourceMutexDeviceRef(
                    AtomicArc::new(Some(Clone::clone(data_ref))),
                    if let Some(ref fa) = self.1 {
                        Arc::downgrade(fa)
                    } else {
                        Weak::new()
                    },
                );
            }
            ResourceMutexState::Invalid => unreachable!(),
        }

        // Take value
        let data_box = match mem::replace(&mut self.0, ResourceMutexState::Invalid) {
            ResourceMutexState::Owned(data_box) => data_box,
            _ => unreachable!(),
        };
        let data_arc = BArcBox::into_arc(data_box);
        self.0 = ResourceMutexState::Limbo(Clone::clone(&data_arc));

        ResourceMutexDeviceRef(
            AtomicArc::new(Some(data_arc)),
            if let Some(ref fa) = self.1 {
                Arc::downgrade(fa)
            } else {
                Weak::new()
            },
        )
    }

    /// `lock_device` that only requires an immutable reference to `Self`.
    ///
    /// This might panic when called on a mutable resource because it cannot
    /// revoke the host write accessibility. For this reason, this must be used
    /// only with immutable resources (specified at the creation time, or by
    /// calling `make_immutable`).
    pub fn expect_device_access(&self) -> (ResourceMutexDeviceRef<F, T>, &T) {
        match self.0 {
            ResourceMutexState::Owned(_) => {
                panic!("cannot revoke host write accessibility");
            }
            ResourceMutexState::Limbo(ref data_ref) => {
                (
                    ResourceMutexDeviceRef(
                        AtomicArc::new(Some(Clone::clone(data_ref))),
                        if let Some(ref fa) = self.1 {
                            Arc::downgrade(fa)
                        } else {
                            Weak::new()
                        },
                    ),
                    data_ref.data(),
                )
            }
            ResourceMutexState::Invalid => unreachable!(),
        }
    }
}
