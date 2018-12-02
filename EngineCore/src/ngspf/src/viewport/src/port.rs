//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::fmt;
use std::sync::Arc;

use zangfx::base as gfx;

pub use crate::wsi::GfxQueue;
use ngspf_core::PresenterFrame;

/// ZanGFX objects passed to ports.
///
/// TODO: Merge with `wsi::GfxObjects`
#[derive(Debug, Clone)]
pub struct GfxObjects {
    pub device: Arc<gfx::Device>,
    pub main_queue: GfxQueue,
    pub copy_queue: Option<GfxQueue>,
}

/// Trait for creating `PortInstance` for a specific NgsGFX device.
pub trait Port: fmt::Debug + Send + Sync + 'static {
    /// Create a port instance for a specific NgsGFX device.
    fn mount(&self, objects: &GfxObjects) -> Box<dyn PortInstance>;
}

/// The properties of a backing store image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PortImageProps {
    /// The extents of the image.
    pub extents: [u32; 2],
    /// The image format. Currently it is always `ImageFormat::SrgbRgba8`.
    pub format: gfx::ImageFormat,
}

#[derive(Debug)]
pub struct PortRenderContext {
    /// The image to render on.
    ///
    /// The image must be transitioned to the `ShaderRead` layout after
    /// rendering. Its ownership must be transfered to `main_queue`.
    ///
    /// The image must not be accessed by GPU after `fence` was updated, nor by
    /// CPU after the command buffer where `fence` is updated was committed.
    /// (Internally, this image is allocated from `TempResPool` and is retained
    /// by `TempResFrame`, which is recycled when the device completes the
    /// execution of a command buffer.)
    pub image: gfx::ImageRef,
    pub image_props: PortImageProps,
    /// The fence to be updated after rendering.
    pub fence: gfx::FenceRef,
    /// Set this to `true` to continuously update the screen.
    pub schedule_next_frame: bool,
}

/// Trait for rendering custom contents as layer contents.
pub trait PortInstance: fmt::Debug + Send + Sync + 'static {
    /// Start rendering a frame.
    ///
    /// The system will inquire the required properties of the rendered image
    /// using the methods of `PortFrame`. After that, the system will allocate
    /// an image and request to start a rendering operation via
    /// `PortFrame::render`.
    ///
    /// The implementation might want to store `frame` in the returned
    /// `PortFrame` so it can read properties from `Port`.
    fn start_frame<'a>(
        &'a mut self,
        frame: &'a PresenterFrame,
    ) -> gfx::Result<Box<dyn PortFrame + 'a>>;
}

pub trait PortFrame: fmt::Debug + Send {
    /// The usage of the backing store image (`PortRenderContext::image`).
    fn image_usage(&mut self) -> gfx::ImageUsageFlags {
        gfx::ImageUsageFlags::Render
    }

    /// The format of the backing store image (`PortRenderContext::image`).
    fn image_format(&mut self) -> gfx::ImageFormat {
        gfx::ImageFormat::SrgbRgba8
    }

    /// The size of the backing store image (`PortRenderContext::image`).
    fn image_extents(&mut self) -> [u32; 2];

    fn render(&mut self, context: &mut PortRenderContext) -> gfx::Result<()>;
}
