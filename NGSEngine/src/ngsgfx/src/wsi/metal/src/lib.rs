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

use std::sync::Arc;
use std::cell::RefCell;
use std::{mem, fmt, ptr};
use std::os::raw::c_void;

use cgmath::Vector2;

use objc::runtime::YES;

use backend_metal::imp::ImageView;

use cocoa::base::id as cocoa_id;
use cocoa::foundation::{NSSize, NSString};
use cocoa::appkit::{NSWindow, NSView};

use winit::os::macos::WindowExt;

mod utils;
use utils::OCPtr;

pub struct MetalWindow {
    window: winit::Window,
    layer: OCPtr<metal::CAMetalLayer>,
    drawable: RefCell<Option<OCPtr<metal::CAMetalDrawable>>>,
    pool: RefCell<Option<OCPtr<metal::NSAutoreleasePool>>>,
    device: Arc<backend_metal::Device>,
}

impl fmt::Debug for MetalWindow {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("MetalWindow")
            .field("layer", &self.layer)
            .field("drawable", &self.drawable)
            .field("pool", &self.pool)
            .field("device", &self.device)
            .finish()
    }
}

impl MetalWindow {
    /// Retrieve the current drawable.
    ///
    /// The returned `CAMetalDrawable` is valid until this object is dropped or
    /// `swap_buffers` or `set_framebuffer_size` is called.
    pub fn drawable(&self) -> metal::CAMetalDrawable {
        self.ensure_have_drawable();
        **self.drawable.borrow().as_ref().unwrap()
    }

    fn update_drawable(&self) {
        // FIXME: what if this fails?
        let nd = self.layer.next_drawable().expect(
            "I just don't know what went wrong! *hopping on a cloud*",
        );
        *self.drawable.borrow_mut() = Some(OCPtr::new(nd).unwrap());
    }

    fn ensure_have_drawable(&self) {
        if self.drawable.borrow().is_none() {
            self.update_drawable();
        }
    }

    fn forget_drawable(&self) {
        *self.drawable.borrow_mut() = None;
    }
}

#[link(name = "ApplicationServices", kind = "framework")]
extern {
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
        format: core::ImageFormat,
    ) -> Result<Self, InitializationError> {
        let pixel_format =
            backend_metal::imp::translate_image_format(format).expect("unsupported ImageFormat");
        let winit_window = wb.build(events_loop).unwrap();

        unsafe {
            let wnd: cocoa_id = mem::transmute(winit_window.get_nswindow());
            let layer: metal::CAMetalLayer = metal::CAMetalLayer::new();
            layer.set_pixel_format(pixel_format);

            let cs_name = NSString::alloc(ptr::null_mut()).init_str("kCGColorSpaceSRGB");
            let colorspace = CGColorSpaceCreateWithName(mem::transmute(cs_name));
            msg_send![cs_name, release];

            let draw_size = winit_window.get_inner_size().unwrap();
            layer.set_edge_antialiasing_mask(0);
            layer.set_masks_to_bounds(true);
            layer.set_colorspace(mem::transmute(colorspace));
            CGColorSpaceRelease(colorspace);
            // layer.set_magnification_filter(kCAFilterNearest);
            // layer.set_minification_filter(kCAFilterNearest);
            layer.set_drawable_size(NSSize::new(draw_size.0 as f64, draw_size.1 as f64));
            layer.set_presents_with_transaction(false);
            layer.remove_all_animations();

            let view = wnd.contentView();
            view.setWantsLayer(YES);
            view.setLayer(mem::transmute(layer.0));

            let adapter = instance.default_adapter().unwrap();
            let device = instance.new_device_builder(&adapter).build().unwrap();
            let metal_device = device.metal_device();
            layer.set_device(metal_device);

            Ok(MetalWindow {
                window: winit_window,
                layer: OCPtr::new(layer).unwrap(),
                drawable: RefCell::new(None),
                pool: RefCell::new(Some(OCPtr::from_raw(metal::NSAutoreleasePool::alloc().init()).unwrap())),
                device: Arc::new(device),
            })
        }
    }
}

impl wsi_core::Window for MetalWindow {
    type Backend = backend_metal::Backend;

    fn winit_window(&self) -> &winit::Window {
        &self.window
    }

    fn device(&self) -> &Arc<backend_metal::Device> {
        &self.device
    }

    fn acquire_framebuffer(&self) -> backend_metal::imp::ImageView {
        self.ensure_have_drawable();

        ImageView::new(self.drawable.borrow().as_ref().unwrap().texture())
    }

    fn finalize_commands(&self, buffer: &mut backend_metal::imp::CommandBuffer) {
        self.ensure_have_drawable();

        buffer.metal_command_buffer().unwrap().present_drawable(
            self.drawable(),
        );
    }

    fn swap_buffers(&self) {
        unsafe {
            self.forget_drawable();

            self.pool.borrow_mut().take();
            *self.pool.borrow_mut() = Some(OCPtr::from_raw(metal::NSAutoreleasePool::alloc().init()).unwrap());
        }
    }

    fn framebuffer_size(&self) -> Vector2<u32> {
        self.ensure_have_drawable();

        let texture = self.drawable.borrow().as_ref().unwrap().texture();
        Vector2::new(texture.width() as u32, texture.height() as u32)
    }

    fn set_framebuffer_size(&self, size: Vector2<u32>) {
        self.layer.set_drawable_size(NSSize::new(size.x as f64, size.y as f64));
        self.forget_drawable();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InitializationError {
    /// Could not create a window.
    Window,
    /// Unable to find a supported driver type.
    DriverType,
}
