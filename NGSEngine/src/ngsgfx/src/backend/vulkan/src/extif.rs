//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Defines an interface to Vulkan ICD.

use std::ops::Deref;
use ash::Device;
use ash::version::{V1_0, DeviceV1_0};
use ash::vk;
use std::sync::Arc;
use std::{fmt, mem};

/// `ash::Device` of the version used by this backend.
pub type AshDevice = Device<V1_0>;

/// Represents a reference to a `ash::Device` object.
///
///  - `deref()` returns the underlying device object. The returned object must
///    be valid.
///  - `clone()` creates a new reference that points the same device object.
///    The device must not be destroyed until all references are removed.
///
/// The backend provides [`OwnedDeviceRef`], which is a safe implementation of
/// `DeviceRef`. This is implemented by using `std::sync::Arc` and might have
/// some overhead for atomic operations. You can implement one by your own
/// if you need a extra performance.
///
/// [`OwnedDeviceRef`]: struct.OwnedDeviceRef.html
pub unsafe trait DeviceRef: Clone + Send + Sync + fmt::Debug + 'static {
    fn device(&self) -> &AshDevice;

    /// Retrieve `AllocationCallbacks` used to perform host memory allocations.
    ///
    /// Since this trait requires `Sync` and this function's return type is
    /// a reference to `AllocationCallbacks`, the allocation functions are
    /// required to be thread-safe. (I wish Rust had higher-kinded types)
    ///
    /// Returns `None` by default.
    fn allocation_callbacks(&self) -> Option<&vk::AllocationCallbacks> { None }
}

/// Destroys the contained `AshDevice` automatically when dropped.
struct UniqueDevice(AshDevice);

impl Drop for UniqueDevice {
    fn drop(&mut self) {
        unsafe { self.0.destroy_device(None) };
    }
}

impl fmt::Debug for UniqueDevice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("UniqueDevice")
            .field(&self.0.handle())
            .finish()
    }
}

/// `DeviceRef` with an owned reference to `ash::Device`.
///
/// The device will be destroyed automatically when all references are removed.
#[derive(Debug, Clone)]
pub struct OwnedDeviceRef {
    device: Arc<UniqueDevice>,
}

unsafe impl DeviceRef for OwnedDeviceRef {
    fn device(&self) -> &AshDevice {
        &self.device.0
    }
}

impl OwnedDeviceRef {
    /// Construct an `OwnedDeviceRef` with a given device object.
    ///
    /// The ownership of `device` will be transfered into the created `OwnedDeviceRef`.
    /// If you have any `ash::Device` that points the same instance of the device
    /// (for example, `ash::Device::clone` creates a new reference to the same instance),
    /// you must not destroy it after calling this function (this is one of the reasons why
    /// this function is marked as unsafe).
    ///
    /// `ash::Device` owned by `OwnedDeviceRef` will not be destroyed until all
    /// instances of `OwnedDeviceRef` are dropped. The Vulkan specification states
    /// that all child objects must be destroyed before their parent object is
    /// destroyed, which means you cannot destroy the originating `ash::Instance`
    /// safely until it is confirmed that all `ash::Device`s created from that are
    /// destroyed. One way to ensure this is calling `OwnedDeviceRef::try_take`,
    /// which will cause it to relinquish the ownership of the contained `ash::Device`
    /// if there are no remaining references to it. Only if this function returns `Ok(x)`,
    /// you can destroy `x` as well as the originating `ash::Instance` safely (supposing
    /// all other objects created on it have been already destroyed).
    pub unsafe fn from_raw(device: AshDevice) -> Self {
        Self { device: Arc::new(UniqueDevice(device)) }
    }

    /// Return the contained `ash::Device` if there is exactly one reference to it
    /// i.e. there exists no other `OwnedDeviceRef` pointing at the same `ash::Device`.
    pub fn try_take(self) -> Result<AshDevice, Self> {
        match Arc::try_unwrap(self.device) {
            Ok(dev) => {
                let ret = dev.0.clone();
                mem::forget(dev); // prevent `drop()`
                Ok(ret)
            }
            Err(arc_dev) => Err(Self { device: arc_dev }),
        }
    }
}
