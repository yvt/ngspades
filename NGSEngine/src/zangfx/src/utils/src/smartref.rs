//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Smart pointers for resource handles.
use std::ops::{Deref, DerefMut};
use std::borrow::Borrow;
use base;

pub trait Destroy {
    fn destroy(&self, device: &base::Device);
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Hash)]
pub struct GfxDeleter;

impl Destroy for base::Buffer {
    fn destroy(&self, device: &base::Device) {
        device.borrow().destroy_buffer(self).unwrap();
    }
}

impl Destroy for base::Image {
    fn destroy(&self, device: &base::Device) {
        device.borrow().destroy_image(self).unwrap();
    }
}

impl Destroy for base::ImageView {
    fn destroy(&self, device: &base::Device) {
        device.borrow().destroy_image_view(self).unwrap();
    }
}

impl Destroy for base::Sampler {
    fn destroy(&self, device: &base::Device) {
        device.borrow().destroy_sampler(self).unwrap();
    }
}

/// A smart pointer for a resource handle.
///
/// Destroys the contained object automatically when dropped.
#[derive(Debug)]
pub struct Unique<D: Borrow<base::Device>, T: Destroy> {
    device: D,
    obj: T,
}

impl<D: Borrow<base::Device>, T: Destroy> Unique<D, T> {
    pub fn new(device: D, obj: T) -> Self {
        Self { device, obj }
    }

    /// Unwrap the contained object.
    pub fn into_inner(self) -> (D, T) {
        use std::ptr::read;
        use std::mem::forget;
        let device = unsafe { read(&self.device) };
        let obj = unsafe { read(&self.obj) };
        forget(self);
        (device, obj)
    }
}

impl<D: Borrow<base::Device>, T: Destroy> Deref for Unique<D, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.obj
    }
}

impl<D: Borrow<base::Device>, T: Destroy> DerefMut for Unique<D, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.obj
    }
}

impl<D: Borrow<base::Device>, T: Destroy> Drop for Unique<D, T> {
    fn drop(&mut self) {
        self.obj.destroy(self.device.borrow());
    }
}

pub type UniqueBuffer<D> = Unique<D, base::Buffer>;

pub type UniqueImage<D> = Unique<D, base::Image>;

pub type UniqueImageView<D> = Unique<D, base::ImageView>;

pub type UniqueSampler<D> = Unique<D, base::Sampler>;
