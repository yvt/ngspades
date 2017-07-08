//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Defines an interface to Vulkan ICD.

use std::ops::Deref;
use ash::Device;
use ash::version::{V1_0, DeviceV1_0};
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
/// `DeviceRef`. This is implemented using `std::sync::Arc` and have a considerable
/// amount of overhead for atomic operations. You can implement one by your own
/// if you need a extra performance.
///
/// [`OwnedDeviceRef`]: struct.OwnedDeviceRef.html
pub unsafe trait DeviceRef: Clone + Send + Sync + fmt::Debug + 'static {
    fn device(&self) -> &AshDevice;
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
    /// Constructs an `OwnedDeviceRef` with a given device object.
    ///
    /// The ownership of `device` is transfered into the created `OwnedDeviceRef`.
    /// If you have any `ash::Device` that points the same instance of the device
    /// (for example, `ash::Device::clone` creates a new reference to the same instance),
    /// you must not destroy it after calling this function (this is one of the reasons why
    /// this function is marked as unsafe).
    pub unsafe fn from_raw(device: AshDevice) -> Self {
        Self { device: Arc::new(UniqueDevice(device)) }
    }

    /// Return the contained `ash::Device` if there is exactly one reference to it.
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
