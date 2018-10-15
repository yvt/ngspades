//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::ops::Range;
use zangfx_base::{self as base, Result};
use zangfx_common::IntoWithPad;

use super::*;

/// A request involving [`CopyCmdEncoder`].
///
/// [`CopyCmdEncoder`]: zangfx_base::CopyCmdEncoder
pub trait CopyRequest: Request {
    /// Encode copy commands.
    fn copy(
        &mut self,
        _encoder: &mut dyn base::CopyCmdEncoder,
        _staging_buffer: &base::BufferRef,
        _staging_buffer_range: Range<DeviceSize>,
    ) -> Result<()> {
        Ok(())
    }
}

/// Processes [`CopyRequest`]s.
#[derive(Debug)]
pub struct CopyCmdGenerator;

impl<T: CopyRequest> CmdGenerator<T> for CopyCmdGenerator {
    fn encode(
        &mut self,
        cmd_buffer: &mut base::CmdBufferRef,
        staging_buffer: &base::BufferRef,
        requests: &mut [(T, Range<DeviceSize>)],
    ) -> Result<()> {
        let encoder = cmd_buffer.encode_copy();
        for (request, range) in requests {
            request.copy(encoder, staging_buffer, range.clone())?;
        }
        Ok(())
    }
}

/// A buffer staging request implementing [`CopyRequest`].
#[derive(Debug, Clone, Copy)]
pub struct StageBuffer<T, B> {
    pub src_data: T,
    pub dst_buffer: B,
    pub dst_offset: base::DeviceSize,
}

impl<T, B> StageBuffer<T, B> {
    /// Construct a `StageBuffer`.
    pub fn new(buffer: B, offset: base::DeviceSize, data: T) -> Self {
        Self {
            src_data: data,
            dst_buffer: buffer,
            dst_offset: offset,
        }
    }
}

impl<T: Borrow<[u8]>, B> Request for StageBuffer<T, B> {
    fn size(&self) -> usize {
        self.src_data.borrow().len()
    }

    fn populate(&mut self, staging_buffer: &mut [u8]) {
        staging_buffer.copy_from_slice(self.src_data.borrow());
    }
}

impl<T: Borrow<[u8]>, B: Borrow<base::BufferRef>> CopyRequest for StageBuffer<T, B> {
    fn copy(
        &mut self,
        encoder: &mut dyn base::CopyCmdEncoder,
        staging_buffer: &base::BufferRef,
        staging_buffer_range: Range<base::DeviceSize>,
    ) -> Result<()> {
        encoder.copy_buffer(
            staging_buffer,
            staging_buffer_range.start,
            self.dst_buffer.borrow(),
            self.dst_offset,
            staging_buffer_range.end - staging_buffer_range.start,
        );

        Ok(())
    }
}

/// An image staging request implementing [`CopyRequest`].
#[derive(Debug, Clone)]
pub struct StageImage<T, I> {
    pub src_data: T,
    pub src_row_stride: base::DeviceSize,
    pub src_plane_stride: base::DeviceSize,
    pub dst_image: I,
    pub dst_range: base::ImageLayerRange,
    pub dst_aspect: base::ImageAspect,
    pub dst_origin: [u32; 3],
    pub size: [u32; 3],
}

impl<T, I> StageImage<T, I> {
    /// Construct a `StageImage` with reasonable default settings.
    pub fn new_default(image: I, data: T, size: &[u32]) -> Self {
        let size: [u32; 3] = size.into_with_pad(1);
        Self {
            src_data: data,
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

impl<T: Borrow<[u8]>, I> Request for StageImage<T, I> {
    fn size(&self) -> usize {
        self.src_data.borrow().len()
    }

    fn populate(&mut self, staging_buffer: &mut [u8]) {
        staging_buffer.copy_from_slice(self.src_data.borrow());
    }
}

impl<T: Borrow<[u8]>, I: Borrow<base::ImageRef>> CopyRequest for StageImage<T, I> {
    fn copy(
        &mut self,
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
            self.dst_image.borrow(),
            self.dst_aspect,
            &self.dst_range,
            &self.dst_origin,
            &self.size,
        );

        Ok(())
    }
}

/// A tagged union of [`StageBuffer`] and [`StageImage`] implementing
/// [`CopyRequest`].
#[derive(Debug, Clone)]
pub enum Stage<T, B, I> {
    Buffer(StageBuffer<T, B>),
    Image(StageImage<T, I>),
}

impl<T, B, I> From<StageBuffer<T, B>> for Stage<T, B, I> {
    fn from(x: StageBuffer<T, B>) -> Self {
        Stage::Buffer(x)
    }
}

impl<T, B, I> From<StageImage<T, I>> for Stage<T, B, I> {
    fn from(x: StageImage<T, I>) -> Self {
        Stage::Image(x)
    }
}

impl<T: Borrow<[u8]>, B, I> Request for Stage<T, B, I> {
    fn size(&self) -> usize {
        match self {
            Stage::Buffer(inner) => Request::size(inner),
            Stage::Image(inner) => Request::size(inner),
        }
    }

    fn populate(&mut self, staging_buffer: &mut [u8]) {
        match self {
            Stage::Buffer(inner) => Request::populate(inner, staging_buffer),
            Stage::Image(inner) => Request::populate(inner, staging_buffer),
        }
    }
}

impl<T: Borrow<[u8]>, B: Borrow<base::BufferRef>, I: Borrow<base::ImageRef>> CopyRequest
    for Stage<T, B, I>
{
    fn copy(
        &mut self,
        encoder: &mut dyn base::CopyCmdEncoder,
        staging_buffer: &base::BufferRef,
        staging_buffer_range: Range<base::DeviceSize>,
    ) -> Result<()> {
        match self {
            Stage::Buffer(inner) => {
                CopyRequest::copy(inner, encoder, staging_buffer, staging_buffer_range)
            }
            Stage::Image(inner) => {
                CopyRequest::copy(inner, encoder, staging_buffer, staging_buffer_range)
            }
        }
    }
}
