//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::Arc;
use zangfx::base as gfx;

#[cfg(target_os = "macos")]
mod cvdisplaylink;
#[cfg(target_os = "macos")]
mod metal;
#[cfg(target_os = "macos")]
pub use self::metal::*;

#[cfg(not(target_os = "macos"))]
mod vulkan;
#[cfg(not(target_os = "macos"))]
pub use self::vulkan::*;

mod autoreleasepool;
pub use self::autoreleasepool::*;

#[derive(Debug, Clone)]
pub struct GfxQueue {
    pub queue: Arc<gfx::CmdQueue>,
    pub queue_family: gfx::QueueFamily,
}

/// ZanGFX device and relevant objects managed by the window manager.
#[derive(Debug, Clone)]
pub struct WmDevice {
    pub device: Arc<gfx::Device>,
    pub main_queue: GfxQueue,
    pub copy_queue: Option<GfxQueue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SurfaceProps {
    pub extents: [u32; 2],
    pub format: gfx::ImageFormat,
}

/// Provides the graphics content of windows.
pub trait Painter {
    /// Type of an object associated with each ZanGFX device.
    type DeviceData;

    /// Passed by the owner of `WindowManager`.
    type SurfaceParam;

    /// Type of an object associated with each surface.
    type SurfaceData;

    /// Passed by the owner of `WindowManager`.
    type UpdateParam;

    /// Prepare the painter for a newly discovered ZanGFX device.
    fn add_device(&mut self, device: &WmDevice) -> Self::DeviceData;

    fn remove_device(&mut self, device: &WmDevice, data: Self::DeviceData);

    /// Prepare the painter for a newly created surface.
    fn add_surface(
        &mut self,
        device: &WmDevice,
        device_data: &mut Self::DeviceData,
        surface: &SurfaceRef,
        param: Self::SurfaceParam,
        surface_props: &SurfaceProps,
    ) -> Self::SurfaceData;

    fn remove_surface(
        &mut self,
        device: &WmDevice,
        device_data: &mut Self::DeviceData,
        surface: &SurfaceRef,
        data: Self::SurfaceData,
    ) -> Self::SurfaceParam;

    /// Notify the change of `SurfaceProps`.
    fn update_surface(
        &mut self,
        device: &WmDevice,
        device_data: &mut Self::DeviceData,
        surface: &SurfaceRef,
        data: &mut Self::SurfaceData,
        surface_props: &SurfaceProps,
    );

    /// Encode commands.
    fn paint(
        &mut self,
        device: &WmDevice,
        device_data: &mut Self::DeviceData,
        surface: &SurfaceRef,
        surface_data: &mut Self::SurfaceData,
        update_param: &Self::UpdateParam,
        drawable: &mut Drawable,
    );
}

/// Passed by the window manager.
pub trait Drawable {
    fn image(&self) -> &gfx::Image;
    fn surface_props(&self) -> &SurfaceProps;

    fn pixel_ratio(&self) -> f32 {
        1.0
    }

    /// Encode commands into the command buffer that initiates the presentation
    /// operation.
    ///
    /// This must be called on the command buffer where the contents of the
    /// drawable image is written.
    ///
    /// The drawable image must be transitioned to the `Present` layout by the
    /// end of the command buffer.
    ///
    /// - `queue_family` indicates the queue family where the drawable image
    ///   was generated.
    /// - `stage` indicates the pipeline stage where the drawable image was
    ///   written.
    ///
    fn encode_prepare_present(
        &mut self,
        cmd_buffer: &mut gfx::CmdBuffer,
        queue_family: gfx::QueueFamily,
        stage: gfx::StageFlags,
        access: gfx::AccessTypeFlags,
    );

    /// Enqueue the presentation operation. Must be called *after* the command
    /// buffer on which `encode_present` was called was enqueued (i.e., the
    /// command buffer was `commit`ed and then the queue was `flush`ed).
    fn enqueue_present(&mut self);
}
