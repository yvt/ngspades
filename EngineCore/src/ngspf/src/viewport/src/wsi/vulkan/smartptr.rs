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

impl crate::Debug for UniqueInstance {
    fn fmt(&self, fmt: &mut crate::fmt::Formatter) -> crate::fmt::Result {
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

impl crate::Debug for UniqueDevice {
    fn fmt(&self, fmt: &mut crate::fmt::Formatter) -> crate::fmt::Result {
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
pub struct UniqueSurfaceKHR<T: Borrow<ext::khr::Surface>>(pub T, pub vk::SurfaceKHR);

impl<T: Borrow<ext::khr::Surface>> AutoPtr<vk::SurfaceKHR> for UniqueSurfaceKHR<T> {
    fn into_inner(self) -> vk::SurfaceKHR {
        let handle = self.1;
        forget(self); // Skip `drop`
        handle
    }
}

impl<T: Borrow<ext::khr::Surface>> Drop for UniqueSurfaceKHR<T> {
    fn drop(&mut self) {
        unsafe {
            self.0.borrow().destroy_surface(self.1, None);
        }
    }
}

impl<T: Borrow<ext::khr::Surface>> Deref for UniqueSurfaceKHR<T> {
    type Target = vk::SurfaceKHR;
    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

#[derive(Debug)]
pub struct UniqueSwapchainKHR<T: Borrow<ext::khr::Swapchain>>(pub T, pub vk::SwapchainKHR);

impl<T: Borrow<ext::khr::Swapchain>> AutoPtr<vk::SwapchainKHR> for UniqueSwapchainKHR<T> {
    fn into_inner(self) -> vk::SwapchainKHR {
        let handle = self.1;
        forget(self); // Skip `drop`
        handle
    }
}

impl<T: Borrow<ext::khr::Swapchain>> Drop for UniqueSwapchainKHR<T> {
    fn drop(&mut self) {
        unsafe {
            self.0.borrow().destroy_swapchain(self.1, None);
        }
    }
}

impl<T: Borrow<ext::khr::Swapchain>> Deref for UniqueSwapchainKHR<T> {
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
