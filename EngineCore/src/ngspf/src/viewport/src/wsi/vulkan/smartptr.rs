//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::ash::{self, extensions as ext, version::*, vk};
use std::borrow::Borrow;
use std::mem::forget;
use std::ops::Deref;

pub trait AutoPtr<T>: Deref<Target = T> + Sized {
    fn into_inner(self) -> T;
}

pub struct UniqueInstance(pub ash::Instance);

impl ::Debug for UniqueInstance {
    fn fmt(&self, fmt: &mut ::fmt::Formatter) -> ::fmt::Result {
        fmt.debug_tuple("UniqueInstance")
            .field(&self.0.handle())
            .finish()
    }
}

impl Drop for UniqueInstance {
    fn drop(&mut self) {
        unsafe {
            self.0.destroy_instance(None);
        }
    }
}

impl Deref for UniqueInstance {
    type Target = ash::Instance;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct UniqueDevice(pub ash::Device);

impl ::Debug for UniqueDevice {
    fn fmt(&self, fmt: &mut ::fmt::Formatter) -> ::fmt::Result {
        fmt.debug_tuple("UniqueDevice")
            .field(&self.0.handle())
            .finish()
    }
}

impl Drop for UniqueDevice {
    fn drop(&mut self) {
        unsafe {
            self.0.destroy_device(None);
        }
    }
}

impl Deref for UniqueDevice {
    type Target = ash::Device;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct UniqueSurfaceKHR<T: Borrow<ext::Surface>>(pub T, pub vk::SurfaceKHR);

impl<T: Borrow<ext::Surface>> AutoPtr<vk::SurfaceKHR> for UniqueSurfaceKHR<T> {
    fn into_inner(self) -> vk::SurfaceKHR {
        let handle = self.1;
        forget(self); // Skip `drop`
        handle
    }
}

impl<T: Borrow<ext::Surface>> Drop for UniqueSurfaceKHR<T> {
    fn drop(&mut self) {
        unsafe {
            self.0.borrow().destroy_surface_khr(self.1, None);
        }
    }
}

impl<T: Borrow<ext::Surface>> Deref for UniqueSurfaceKHR<T> {
    type Target = vk::SurfaceKHR;
    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

#[derive(Debug)]
pub struct UniqueSwapchainKHR<T: Borrow<ext::Swapchain>>(pub T, pub vk::SwapchainKHR);

impl<T: Borrow<ext::Swapchain>> AutoPtr<vk::SwapchainKHR> for UniqueSwapchainKHR<T> {
    fn into_inner(self) -> vk::SwapchainKHR {
        let handle = self.1;
        forget(self); // Skip `drop`
        handle
    }
}

impl<T: Borrow<ext::Swapchain>> Drop for UniqueSwapchainKHR<T> {
    fn drop(&mut self) {
        unsafe {
            self.0.borrow().destroy_swapchain_khr(self.1, None);
        }
    }
}

impl<T: Borrow<ext::Swapchain>> Deref for UniqueSwapchainKHR<T> {
    type Target = vk::SwapchainKHR;
    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

#[derive(Debug)]
pub struct UniqueFence<T: Borrow<ash::Device>>(pub T, pub vk::Fence);

impl<T: Borrow<ash::Device>> AutoPtr<vk::Fence> for UniqueFence<T> {
    fn into_inner(self) -> vk::Fence {
        let handle = self.1;
        forget(self); // Skip `drop`
        handle
    }
}

impl<T: Borrow<ash::Device>> Drop for UniqueFence<T> {
    fn drop(&mut self) {
        unsafe {
            self.0.borrow().destroy_fence(self.1, None);
        }
    }
}

impl<T: Borrow<ash::Device>> Deref for UniqueFence<T> {
    type Target = vk::Fence;
    fn deref(&self) -> &Self::Target {
        &self.1
    }
}
