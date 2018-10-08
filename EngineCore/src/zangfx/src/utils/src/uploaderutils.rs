//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use pod::Pod;
use std::ops::Range;
use zangfx_base::{self as base, Result};
use zangfx_common::IntoWithPad;

use crate::uploader;

/// A buffer staging request. Implements `UploadRequest`.
#[derive(Debug, Clone, Copy)]
pub struct StageBuffer<'a> {
    pub src_data: &'a [u8],
    pub dst_buffer: &'a base::BufferRef,
    pub dst_offset: base::DeviceSize,
}

impl<'a> StageBuffer<'a> {
    /// Construct a `StageBuffer`.
    pub fn new<T: Pod>(
        buffer: &'a base::BufferRef,
        offset: base::DeviceSize,
        data: &'a [T],
    ) -> Self {
        Self {
            src_data: Pod::map_slice(data).unwrap(),
            dst_buffer: buffer,
            dst_offset: offset,
        }
    }
}

impl<'a> uploader::UploadRequest for StageBuffer<'a> {
    fn size(&self) -> usize {
        self.src_data.len()
    }

    fn populate(&self, staging_buffer: &mut [u8]) {
        staging_buffer.copy_from_slice(self.src_data);
    }

    fn copy(
        &self,
        encoder: &mut dyn base::CopyCmdEncoder,
        staging_buffer: &base::BufferRef,
        staging_buffer_range: Range<base::DeviceSize>,
    ) -> Result<()> {
        encoder.copy_buffer(
            staging_buffer,
            staging_buffer_range.start,
            self.dst_buffer,
            self.dst_offset,
            self.src_data.len() as u64,
        );

        Ok(())
    }
}

/// An image staging request. Implements `UploadRequest`.
#[derive(Debug, Clone)]
pub struct StageImage<'a> {
    pub src_data: &'a [u8],
    pub src_row_stride: base::DeviceSize,
    pub src_plane_stride: base::DeviceSize,
    pub dst_image: &'a base::ImageRef,
    pub dst_range: base::ImageLayerRange,
    pub dst_aspect: base::ImageAspect,
    pub dst_origin: [u32; 3],
    pub size: [u32; 3],
}

impl<'a> StageImage<'a> {
    /// Construct a `StageImage` with reasonable default settings.
    pub fn new_default<T: Pod>(image: &'a base::ImageRef, data: &'a [T], size: &[u32]) -> Self {
        let size: [u32; 3] = size.into_with_pad(1);
        Self {
            src_data: Pod::map_slice(data).unwrap(),
            src_row_stride: size[0] as u64,
            src_plane_stride: (size[0] * size[1]) as u64,
            dst_image: image,
            dst_range: base::ImageLayerRange {
                mip_level: 0,
                layers: 0..1,
            },
            dst_aspect: base::ImageAspect::Color,
            dst_origin: [0, 0, 0],
            size,
        }
    }
}

impl<'a> uploader::UploadRequest for StageImage<'a> {
    fn size(&self) -> usize {
        self.src_data.len()
    }

    fn populate(&self, staging_buffer: &mut [u8]) {
        staging_buffer.copy_from_slice(self.src_data);
    }

    fn copy(
        &self,
        encoder: &mut dyn base::CopyCmdEncoder,
        staging_buffer: &base::BufferRef,
        staging_buffer_range: Range<base::DeviceSize>,
    ) -> Result<()> {
        encoder.copy_buffer_to_image(
            staging_buffer,
            &base::BufferImageRange {
                offset: staging_buffer_range.start,
                row_stride: self.src_row_stride,
                plane_stride: self.src_plane_stride,
            },
            self.dst_image,
            self.dst_aspect,
            &self.dst_range,
            &self.dst_origin,
            &self.size,
        );

        Ok(())
    }
}
