//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use metal;
use metal::NSObjectProtocol;

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

impl core::ImageView for ImageView {}

unsafe impl Send for ImageViewData {}
unsafe impl Sync for ImageViewData {} // no interior mutability

impl ImageView {
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

unsafe impl Send for ImageData {}
unsafe impl Sync for ImageData {} // no interior mutability

impl Image {
    pub(crate) fn metal_texture(&self) -> &metal::MTLTexture {
        self.data.metal_texture.deref()
    }
}