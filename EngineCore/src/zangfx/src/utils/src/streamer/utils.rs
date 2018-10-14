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

/// A request involving [`CopyCmdEncoder`].
///
/// [`CopyCmdEncoder`]: zangfx_base::CopyCmdEncoder
pub trait CopyRequest: StreamerRequest {
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

impl<'a> StreamerRequest for StageBuffer<'a> {
    fn size(&self) -> usize {
        UploadRequest::size(self)
    }

    fn populate(&mut self, staging_buffer: &mut [u8]) {
        UploadRequest::populate(self, staging_buffer)
    }
}

impl<'a> CopyRequest for StageBuffer<'a> {
    fn copy(
        &mut self,
        encoder: &mut dyn base::CopyCmdEncoder,
        staging_buffer: &base::BufferRef,
        staging_buffer_range: Range<base::DeviceSize>,
    ) -> Result<()> {
        UploadRequest::copy(self, encoder, staging_buffer, staging_buffer_range)
    }
}

impl<'a> StreamerRequest for StageImage<'a> {
    fn size(&self) -> usize {
        UploadRequest::size(self)
    }

    fn populate(&mut self, staging_buffer: &mut [u8]) {
        UploadRequest::populate(self, staging_buffer)
    }
}

impl<'a> CopyRequest for StageImage<'a> {
    fn copy(
        &mut self,
        encoder: &mut dyn base::CopyCmdEncoder,
        staging_buffer: &base::BufferRef,
        staging_buffer_range: Range<base::DeviceSize>,
    ) -> Result<()> {
        UploadRequest::copy(self, encoder, staging_buffer, staging_buffer_range)
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
}

impl<'a> CopyRequest for Stage<'a> {
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
