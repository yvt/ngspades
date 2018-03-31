//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::ops::Range;
use {base, uploader};
use base::Result;
use common::IntoWithPad;

/// A buffer staging request. Implements `UploadRequest`.
#[derive(Debug, Clone, Copy)]
pub struct StageBuffer<'a> {
    pub src_data: &'a [u8],
    pub dst_buffer: &'a base::Buffer,
    pub dst_offset: base::DeviceSize,
}

impl<'a> StageBuffer<'a> {
    /// Construct a `StageBuffer`.
    pub fn new<T: Copy>(buffer: &'a base::Buffer, offset: base::DeviceSize, data: &'a [T]) -> Self {
        use std::slice::from_raw_parts;
        use std::mem::size_of_val;
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
        staging_buffer: &base::Buffer,
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
    pub dst_image: &'a base::Image,
    pub dst_range: base::ImageLayerRange,
    pub dst_aspect: base::ImageAspect,
    pub dst_old_layout: base::ImageLayout,
    pub dst_new_layout: base::ImageLayout,
    pub dst_origin: [u32; 3],
    pub size: [u32; 3],
}

impl<'a> StageImage<'a> {
    /// Construct a `StageImage` with reasonable default settings.
    pub fn new_default<T: Copy>(
        image: &'a base::Image,
        layout: base::ImageLayout,
        data: &'a [T],
        size: &[u32],
    ) -> Self {
        use std::slice::from_raw_parts;
        use std::mem::size_of_val;
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
            dst_old_layout: base::ImageLayout::Undefined,
            dst_new_layout: layout,
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
                staging_buffer: &base::Buffer,
                staging_buffer_range: Range<base::DeviceSize>,
            ) -> Result<()> {
                let staging_layout = match self.0.dst_new_layout {
                    base::ImageLayout::CopyWrite | base::ImageLayout::General => {
                        self.0.dst_new_layout
                    }
                    _ => base::ImageLayout::CopyWrite,
                };

                if self.0.dst_old_layout != staging_layout {
                    let barrier = self.1
                        .build_barrier()
                        .image(
                            flags![base::AccessType::{}],
                            flags![base::AccessType::{CopyWrite}],
                            self.0.dst_image,
                            self.0.dst_old_layout,
                            staging_layout,
                            &self.0.dst_range.clone().into(),
                        )
                        .build()?;
                    encoder.barrier(&barrier);
                }

                encoder.copy_buffer_to_image(
                    staging_buffer,
                    &base::BufferImageRange {
                        offset: staging_buffer_range.start,
                        row_stride: self.0.src_row_stride,
                        plane_stride: self.0.src_plane_stride,
                    },
                    self.0.dst_image,
                    staging_layout,
                    self.0.dst_aspect,
                    &self.0.dst_range,
                    &self.0.dst_origin,
                    &self.0.size,
                );

                if self.0.dst_new_layout != staging_layout {
                    let barrier = self.1
                        .build_barrier()
                        .image(
                            flags![base::AccessType::{CopyWrite}],
                            flags![base::AccessType::{}],
                            self.0.dst_image,
                            staging_layout,
                            self.0.dst_new_layout,
                            &self.0.dst_range.clone().into(),
                        )
                        .build()?;
                    encoder.barrier(&barrier);
                }
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
