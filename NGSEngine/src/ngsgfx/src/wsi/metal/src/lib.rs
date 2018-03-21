//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsGFX Window System Integration for Metal
//! ==========================================
//!
//! Mostly based on the [`gfx_window_metal`] crate
//!
//! [`gfx_window_metal`]: https://github.com/gfx-rs/gfx

#[macro_use]
extern crate objc;
extern crate cocoa;
extern crate cgmath;

extern crate ngsgfx_core as core;
extern crate ngsgfx_metal as backend_metal;
extern crate ngsgfx_wsi_core as wsi_core;
use backend_metal::ll as metal;
use wsi_core::winit;
use core::{Instance, DeviceBuilder};
use core::Signedness::*;
use core::Normalizedness::*;

use std::sync::Arc;
use std::{mem, fmt, ptr};
use std::os::raw::c_void;

use cgmath::Vector3;

use objc::runtime::YES;

use cocoa::base::id as cocoa_id;
use cocoa::foundation::{NSSize, NSString};
use cocoa::appkit::{NSWindow, NSView};

use winit::os::macos::WindowExt;

use backend_metal::utils::OCPtr;

#[derive(Debug)]
pub struct Drawable {
    drawable: OCPtr<metal::CAMetalDrawable>,
    image: backend_metal::imp::Image,
}

impl Drawable {
    pub fn new(drawable: metal::CAMetalDrawable) -> Drawable {
        Drawable {
            drawable: OCPtr::new(drawable).unwrap(),
            image: backend_metal::imp::Image::from_raw(drawable.texture()),
        }
    }
    pub fn drawable(&self) -> metal::CAMetalDrawable {
        *self.drawable
    }
}

impl wsi_core::Drawable for Drawable {
    type Backend = backend_metal::Backend;

    fn image(&self) -> &backend_metal::imp::Image {
        &self.image
    }

    fn acquiring_fence(&self) -> Option<&backend_metal::imp::Fence> {
        None
    }

    fn finalize(
        &self,
        command_buffer: &mut backend_metal::imp::CommandBuffer,
        _: core::PipelineStageFlags,
        _: core::AccessTypeFlags,
        _: core::ImageLayout,
    ) {
        command_buffer
            .metal_command_buffer()
            .unwrap()
            .present_drawable(self.drawable());
    }

    fn present(&self) {}
}

pub struct Swapchain {
    window: Arc<winit::Window>,
    device: Arc<backend_metal::Device>,
    layer: OCPtr<metal::CAMetalLayer>,
    color_space: wsi_core::ColorSpace,
}

impl fmt::Debug for Swapchain {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Swapchain")
            .field("device", &self.device)
            .field("layer", &self.layer)
            .field("color_space", &self.color_space)
            .finish()
    }
}

impl Swapchain {
    pub fn layer(&self) -> metal::CAMetalLayer {
        *self.layer
    }

    fn physical_size(&self) -> (u32, u32) {
        self.window.get_inner_size_pixels().unwrap()
    }

    fn resize(&self) {
        let (mut w, mut h) = self.physical_size();
        if w == 0 {
            w = 1;
        }
        if h == 0 {
            h = 1;
        }
        self.layer.set_drawable_size(
            NSSize::new(w as f64, h as f64),
        );
    }
}

impl wsi_core::Swapchain for Swapchain {
    type Backend = backend_metal::Backend;
    type Drawable = Drawable;

    fn device(&self) -> &backend_metal::Device {
        &self.device
    }

    fn next_drawable(
        &self,
        _: &wsi_core::FrameDescription,
    ) -> Result<Self::Drawable, wsi_core::SwapchainError> {
        let d_size = self.layer.drawable_size();
        let l_size = self.physical_size();
        if (d_size.width as u32, d_size.height as u32) != l_size {
            // emulate the behavior of Windows' Vulkan swapchain
            return Err(wsi_core::SwapchainError::OutOfDate);
        }
        let nd = self.layer.next_drawable();
        match nd {
            Some(nd) => Ok(Drawable::new(nd)),
            None => Err(wsi_core::SwapchainError::NotReady),
        }
    }

    fn drawable_info(&self) -> wsi_core::DrawableInfo {
        let size = self.layer.drawable_size();

        wsi_core::DrawableInfo {
            extents: Vector3::new(size.width as u32, size.height as u32, 1),
            num_array_layers: 1,
            format: backend_metal::imp::translate_metal_pixel_format(self.layer.pixel_format()),
            colorspace: self.color_space
        }
    }
}

pub struct MetalWindow {
    window: Arc<winit::Window>,
    swapchain: Swapchain,
    device: Arc<backend_metal::Device>,
}

impl fmt::Debug for MetalWindow {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("MetalWindow")
            .field("swapchain", &self.swapchain)
            .field("device", &self.device)
            .finish()
    }
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGColorSpaceCreateWithName(name: cocoa_id) -> *const c_void;
    fn CGColorSpaceRelease(space: *const c_void);
}

impl wsi_core::NewWindow for MetalWindow {
    type Environment = backend_metal::Environment;
    type CreationError = InitializationError;

    /// Constructs a new `MetalWindow`.
    ///
    /// `format` must be one of `Bgra8(Unsigned, Normalized)`, `SrgbBgra8`, and
    /// `RgbaFloat16`.
    fn new(
        wb: winit::WindowBuilder,
        events_loop: &winit::EventsLoop,
        instance: &backend_metal::Instance,
        swapchain_description: &wsi_core::SwapchainDescription,
    ) -> Result<Self, InitializationError> {
        let (fmt, color_space) = swapchain_description
            .desired_formats
            .iter()
            .filter_map(|&(fmt, color_space)| {
                let color_space = color_space.unwrap_or(wsi_core::ColorSpace::SrgbNonlinear);
                let fmt = fmt.unwrap_or(core::ImageFormat::Bgra8(Unsigned, Normalized));
                if color_space == wsi_core::ColorSpace::SrgbNonlinear &&
                    [
                        core::ImageFormat::Bgra8(Unsigned, Normalized),
                        core::ImageFormat::SrgbBgra8,
                        core::ImageFormat::RgbaFloat16,
                    ].contains(&fmt)
                {
                    Some((fmt, color_space))
                } else {
                    None
                }
            })
            .nth(0)
            .ok_or(InitializationError::IncompatibleFormat)?;

        let pixel_format = backend_metal::imp::translate_image_format(fmt).unwrap();
        let cs_name = match color_space {
            wsi_core::ColorSpace::SrgbNonlinear => "kCGColorSpaceSRGB",
        };

        let transparent = wb.window.transparent;
        let winit_window = wb.build(events_loop).unwrap();

        unsafe {
            let wnd: cocoa_id = mem::transmute(winit_window.get_nswindow());
            let layer: metal::CAMetalLayer = metal::CAMetalLayer::new();
            layer.set_pixel_format(pixel_format);

            let ns_cs_name = NSString::alloc(ptr::null_mut()).init_str(cs_name);
            let colorspace = CGColorSpaceCreateWithName(mem::transmute(ns_cs_name));
            let () = msg_send![ns_cs_name, release];

            layer.set_edge_antialiasing_mask(0);
            layer.set_masks_to_bounds(true);
            layer.set_opaque(!transparent);
            layer.set_colorspace(mem::transmute(colorspace));
            CGColorSpaceRelease(colorspace);
            // layer.set_magnification_filter(kCAFilterNearest);
            // layer.set_minification_filter(kCAFilterNearest);
            let fb_only: core::ImageUsageFlags = core::ImageUsage::ColorAttachment.into();
            layer.set_framebuffer_only(swapchain_description.image_usage == fb_only);
            layer.set_presents_with_transaction(false);
            layer.remove_all_animations();

            let view = wnd.contentView();
            view.setWantsLayer(YES);
            view.setLayer(mem::transmute(layer.0));

            let adapter = instance.default_adapter().unwrap();
            let device = instance.new_device_builder(&adapter).build().unwrap();
            let metal_device = device.metal_device();
            layer.set_device(metal_device);

            let device = Arc::new(device);
            let winit_window = Arc::new(winit_window);

            let swapchain = Swapchain {
                window: winit_window.clone(),
                device: device.clone(),
                layer: OCPtr::new(layer).unwrap(),
                color_space,
            };
            swapchain.resize();

            Ok(MetalWindow {
                window: winit_window,
                swapchain,
                device: device,
            })
        }
    }
}

impl wsi_core::Window for MetalWindow {
    type Backend = backend_metal::Backend;
    type Swapchain = Swapchain;

    fn winit_window(&self) -> &winit::Window {
        &self.window
    }

    fn device(&self) -> &Arc<backend_metal::Device> {
        &self.device
    }

    fn swapchain(&self) -> &Self::Swapchain {
        &self.swapchain
    }

    fn update_swapchain(&mut self) {
        self.swapchain.resize();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InitializationError {
    /// Could not create a window.
    Window,
    /// No compatible formats were found.
    IncompatibleFormat,
}
