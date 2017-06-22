//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use metal;

use std::ops::Deref;

use {OCPtr, RefEqArc};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageView {
    data: RefEqArc<ImageViewData>,
}

#[derive(Debug)]
struct ImageViewData {
    metal_texture: OCPtr<metal::MTLTexture>,
}

impl core::Marker for ImageView {
    fn set_label(&self, label: Option<&str>) {
        self.data.metal_texture.set_label(label.unwrap_or(""));
    }
}

impl core::ImageView for ImageView {}

unsafe impl Send for ImageViewData {}
unsafe impl Sync for ImageViewData {} // no interior mutability

impl ImageView {
    pub fn new(raw: metal::MTLTexture) -> Self {
        Self { data: RefEqArc::new(ImageViewData { metal_texture: OCPtr::new(raw).unwrap() }) }
    }
    pub(crate) fn metal_texture(&self) -> &metal::MTLTexture {
        self.data.metal_texture.deref()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Image {
    data: RefEqArc<ImageData>,
}

#[derive(Debug)]
struct ImageData {
    metal_texture: OCPtr<metal::MTLTexture>,
}

impl core::Image for Image {}

impl core::Marker for Image {
    fn set_label(&self, label: Option<&str>) {
        self.data.metal_texture.set_label(label.unwrap_or(""));
    }
}

unsafe impl Send for ImageData {}
unsafe impl Sync for ImageData {} // no interior mutability

impl Image {
    pub(crate) fn metal_texture(&self) -> &metal::MTLTexture {
        self.data.metal_texture.deref()
    }
}
