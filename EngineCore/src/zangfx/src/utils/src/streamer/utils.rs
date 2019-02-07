//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use arrayvec::ArrayVec;
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

    /// Return a [`QueueOwnershipTransfer`] describing a queue family ownership
    /// acquire operation to be performed before copy commands.
    ///
    /// The returned value is ignored if [`CopyCmdGenerator::src_queue_family`]
    /// is `None`.
    ///
    /// The default implementation returns `None`.
    ///
    /// [`QueueOwnershipTransfer`]: zangfx_base::QueueOwnershipTransfer
    fn queue_ownership_acquire(&self) -> Option<base::QueueOwnershipTransfer<'_>> {
        None
    }

    /// Return a [`QueueOwnershipTransfer`] describing a queue family ownership
    /// release operation to be performed after copy commands.
    ///
    /// The returned value is ignored if [`CopyCmdGenerator::dst_queue_family`]
    /// is `None`.
    ///
    /// The default implementation returns `None`.
    ///
    /// [`QueueOwnershipTransfer`]: zangfx_base::QueueOwnershipTransfer
    fn queue_ownership_release(&self) -> Option<base::QueueOwnershipTransfer<'_>> {
        None
    }
}

/// Processes [`CopyRequest`]s.
///
/// This `CmdGenerator` is intended to be used for transferring data between the
/// host and the device using copy commands, optionally accompanied with queue
/// family ownership transfer operations.
///
/// For a given batch, it generates a command buffer made up of three parts:
///
///  1. Queue family ownership acquire operations described by
///     [`CopyRequest::queue_ownership_acquire`]. This part is omitted if
///     `src_queue_family` is `None`.
///
///  2. A copy pass including copy commands generated by
///     [`CopyRequest::copy`].
///
///  3. Queue family ownership release operations described by
///     [`CopyRequest::queue_ownership_release`]. This part is omitted if
///     `dst_queue_family` is `None`.
///
#[derive(Debug, Default, Clone, Copy)]
pub struct CopyCmdGenerator {
    /// If set, specifies the source queue family of queue family ownership
    /// acquire operation.
    pub src_queue_family: Option<base::QueueFamily>,
    /// If set, specifies the destination queue family of queue family ownership
    /// release operation.
    pub dst_queue_family: Option<base::QueueFamily>,
}

impl CopyCmdGenerator {
    /// Construct a `CopyCmdGenerator` with default field values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return `self` with a new value for the `src_queue_family` field.
    pub fn with_src_queue_family(self, src_queue_family: Option<base::QueueFamily>) -> Self {
        Self {
            src_queue_family,
            ..self
        }
    }

    /// Return `self` with a new value for the `dst_queue_family` field.
    pub fn with_dst_queue_family(self, dst_queue_family: Option<base::QueueFamily>) -> Self {
        Self {
            dst_queue_family,
            ..self
        }
    }
}

impl<T: CopyRequest> CmdGenerator<T> for CopyCmdGenerator {
    fn encode(
        &mut self,
        cmd_buffer: &mut base::CmdBufferRef,
        staging_buffer: &base::BufferRef,
        requests: &mut [(T, Range<DeviceSize>)],
    ) -> Result<()> {
        if let Some(src_queue_family) = self.src_queue_family {
            for requests in requests.chunks(64) {
                let ops: ArrayVec<[_; 64]> = requests
                    .iter()
                    .filter_map(|x| x.0.queue_ownership_acquire())
                    .collect();
                if ops.len() > 0 {
                    cmd_buffer.queue_ownership_acquire(
                        src_queue_family,
                        base::AccessTypeFlags::COPY_READ,
                        &ops,
                    );
                }
            }
        }

        {
            let encoder = cmd_buffer.encode_copy();
            for (request, range) in requests.iter_mut() {
                request.copy(encoder, staging_buffer, range.clone())?;
            }
        }

        if let Some(dst_queue_family) = self.dst_queue_family {
            for requests in requests.chunks(64) {
                let ops: ArrayVec<[_; 64]> = requests
                    .iter()
                    .filter_map(|x| x.0.queue_ownership_release())
                    .collect();
                if ops.len() > 0 {
                    cmd_buffer.queue_ownership_release(
                        dst_queue_family,
                        base::AccessTypeFlags::COPY_WRITE,
                        &ops,
                    );
                }
            }
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

    fn queue_ownership_release(&self) -> Option<base::QueueOwnershipTransfer<'_>> {
        let buffer = self.dst_buffer.borrow();
        let range =
            self.dst_offset..self.dst_offset + (self.src_data.borrow().len() as base::DeviceSize);
        Some(base::QueueOwnershipTransfer::Buffer {
            buffer,
            range: Some(range),
        })
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

    fn queue_ownership_release(&self) -> Option<base::QueueOwnershipTransfer<'_>> {
        let image = self.dst_image.borrow();
        let src_layout = base::ImageLayout::CopyWrite;
        let dst_layout = base::ImageLayout::CopyWrite; // FIXME: Specify desired layout?
        let range = self.dst_range.clone().into();
        Some(base::QueueOwnershipTransfer::Image {
            image,
            src_layout,
            dst_layout,
            range,
        })
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

    fn queue_ownership_release(&self) -> Option<base::QueueOwnershipTransfer<'_>> {
        match self {
            Stage::Buffer(inner) => CopyRequest::queue_ownership_release(inner),
            Stage::Image(inner) => CopyRequest::queue_ownership_release(inner),
        }
    }
}
