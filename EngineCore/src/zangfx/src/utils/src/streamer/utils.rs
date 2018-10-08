//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::ops::Range;
use zangfx_base::{self as base, Result};

use super::*;
use crate::uploader::UploadRequest;

#[doc(no_inline)]
pub use crate::uploader::{StageBuffer, StageImage};

impl<'a> StreamerRequest for StageBuffer<'a> {
    fn size(&self) -> usize {
        UploadRequest::size(self)
    }

    fn populate(&mut self, staging_buffer: &mut [u8]) {
        UploadRequest::populate(self, staging_buffer)
    }

    fn copy(
        &mut self,
        encoder: &mut dyn base::CopyCmdEncoder,
        staging_buffer: &base::BufferRef,
        staging_buffer_range: Range<base::DeviceSize>,
        _phase: u32,
    ) -> Result<()> {
        UploadRequest::copy(self, encoder, staging_buffer, staging_buffer_range)
    }
}

impl<'a> StreamerRequest for StageImage<'a> {
    fn size(&self) -> usize {
        self.src_data.len()
    }

    fn populate(&mut self, staging_buffer: &mut [u8]) {
        staging_buffer.copy_from_slice(self.src_data);
    }

    fn copy(
        &mut self,
        encoder: &mut dyn base::CopyCmdEncoder,
        staging_buffer: &base::BufferRef,
        staging_buffer_range: Range<base::DeviceSize>,
        _phase: u32,
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

/// A tagged union of [`StageBuffer`] and [`StageImage`], implementing
/// [`StreamerRequest`].
#[derive(Debug, Clone)]
pub enum Stage<'a> {
    Buffer(StageBuffer<'a>),
    Image(StageImage<'a>),
}

impl<'a> From<StageBuffer<'a>> for Stage<'a> {
    fn from(x: StageBuffer<'a>) -> Self {
        Stage::Buffer(x)
    }
}

impl<'a> From<StageImage<'a>> for Stage<'a> {
    fn from(x: StageImage<'a>) -> Self {
        Stage::Image(x)
    }
}

impl<'a> StreamerRequest for Stage<'a> {
    fn size(&self) -> usize {
        match self {
            Stage::Buffer(inner) => StreamerRequest::size(inner),
            Stage::Image(inner) => StreamerRequest::size(inner),
        }
    }

    fn populate(&mut self, staging_buffer: &mut [u8]) {
        match self {
            Stage::Buffer(inner) => StreamerRequest::populate(inner, staging_buffer),
            Stage::Image(inner) => StreamerRequest::populate(inner, staging_buffer),
        }
    }

    fn copy(
        &mut self,
        encoder: &mut dyn base::CopyCmdEncoder,
        staging_buffer: &base::BufferRef,
        staging_buffer_range: Range<base::DeviceSize>,
        phase: u32,
    ) -> Result<()> {
        match self {
            Stage::Buffer(inner) => {
                StreamerRequest::copy(inner, encoder, staging_buffer, staging_buffer_range, phase)
            }
            Stage::Image(inner) => {
                StreamerRequest::copy(inner, encoder, staging_buffer, staging_buffer_range, phase)
            }
        }
    }
}
