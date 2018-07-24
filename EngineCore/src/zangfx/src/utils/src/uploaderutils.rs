//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use base::Result;
use common::IntoWithPad;
use std::ops::Range;
use {base, uploader};

/// A buffer staging request. Implements `UploadRequest`.
#[derive(Debug, Clone, Copy)]
pub struct StageBuffer<'a> {
    pub src_data: &'a [u8],
    pub dst_buffer: &'a base::BufferRef,
    pub dst_offset: base::DeviceSize,
}

impl<'a> StageBuffer<'a> {
    /// Construct a `StageBuffer`.
    pub fn new<T: Copy>(
        buffer: &'a base::BufferRef,
        offset: base::DeviceSize,
        data: &'a [T],
    ) -> Self {
        use std::mem::size_of_val;
        use std::slice::from_raw_parts;
        Self {
            src_data: unsafe { from_raw_parts(data.as_ptr() as *const u8, size_of_val(data)) },
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
        encoder: &mut base::CopyCmdEncoder,
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

/// An image staging request. Use [`UploaderUtils::stage_images`] to submit
/// requests of this type to an `Uploader`.
///
/// [`UploaderUtils::stage_images`]: UploaderUtils::stage_images
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
    pub fn new_default<T: Copy>(image: &'a base::ImageRef, data: &'a [T], size: &[u32]) -> Self {
        use std::mem::size_of_val;
        use std::slice::from_raw_parts;
        let size: [u32; 3] = size.into_with_pad(1);
        Self {
            src_data: unsafe { from_raw_parts(data.as_ptr() as *const u8, size_of_val(data)) },
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

/// Utilities for `Uploader`.
pub trait UploaderUtils {
    /// Initiate image staging operations.
    fn stage_images<'a, I>(&mut self, requests: I) -> Result<uploader::SessionId>
    where
        I: Iterator<Item = StageImage<'a>> + Clone;
}

impl UploaderUtils for uploader::Uploader {
    fn stage_images<'a, I>(&mut self, requests: I) -> Result<uploader::SessionId>
    where
        I: Iterator<Item = StageImage<'a>> + Clone,
    {
        impl<'a> uploader::UploadRequest for (StageImage<'a>, &'static base::Device) {
            fn size(&self) -> usize {
                self.0.src_data.len()
            }

            fn populate(&self, staging_buffer: &mut [u8]) {
                staging_buffer.copy_from_slice(self.0.src_data);
            }

            fn copy(
                &self,
                encoder: &mut base::CopyCmdEncoder,
                staging_buffer: &base::BufferRef,
                staging_buffer_range: Range<base::DeviceSize>,
            ) -> Result<()> {
                encoder.copy_buffer_to_image(
                    staging_buffer,
                    &base::BufferImageRange {
                        offset: staging_buffer_range.start,
                        row_stride: self.0.src_row_stride,
                        plane_stride: self.0.src_plane_stride,
                    },
                    self.0.dst_image,
                    self.0.dst_aspect,
                    &self.0.dst_range,
                    &self.0.dst_origin,
                    &self.0.size,
                );

                Ok(())
            }
        }

        // Untie the lifetime in order to maintain a mutable reference to `self`.
        // The equivalent safe code would be `Arc::clone(self.device())`, but
        // I wanted to save some expensive atomic operations.
        let device = unsafe { &*(&**self.device() as *const _) };

        self.upload(requests.map(&|r| (r, device)))
    }
}
