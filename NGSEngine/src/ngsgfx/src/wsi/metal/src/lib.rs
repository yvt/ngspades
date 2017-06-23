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

extern crate objc;
extern crate cocoa;
extern crate cgmath;

extern crate ngsgfx_core as core;
extern crate ngsgfx_metal as backend_metal;
extern crate ngsgfx_wsi_core as wsi_core;
use backend_metal::ll as metal;
use wsi_core::winit;

use std::sync::Arc;
use std::cell::{Cell, RefCell};
use std::{mem, fmt};
use std::ops::Deref;

use cgmath::Vector2;

use objc::runtime::YES;

use metal::NSObjectProtocol;

use backend_metal::imp::ImageView;

use cocoa::base::id as cocoa_id;
use cocoa::foundation::NSSize;
use cocoa::appkit::{NSWindow, NSView};

use winit::os::macos::WindowExt;

pub struct MetalWindow<T> {
    events_loop: T,
    window: winit::Window,
    layer: metal::CAMetalLayer,
    drawable: Cell<metal::CAMetalDrawable>,
    image_view: RefCell<ImageView>,
    pool: Cell<metal::NSAutoreleasePool>,
    device: Arc<backend_metal::Device>,
}

impl<T> fmt::Debug for MetalWindow<T>
where
    T: Deref<Target = winit::EventsLoop>,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("MetalWindow")
            .field("layer", &self.layer)
            .field("drawable", &self.drawable)
            .field("pool", &self.pool)
            .field("device", &self.device)
            .finish()
    }
}

impl<T> MetalWindow<T>
where
    T: Deref<Target = winit::EventsLoop>,
{
    /// Retrieve the current drawable.
    ///
    /// The returned `CAMetalDrawable` is valid until this object is dropped or
    /// `swap_buffers` is called.
    pub fn drawable(&self) -> metal::CAMetalDrawable {
        self.drawable.get()
    }
}

impl<T> wsi_core::NewWindow<T> for MetalWindow<T>
where
    T: Deref<Target = winit::EventsLoop>,
{
    /// Constructs a new `MetalWindow`.
    ///
    /// `format` must be one of `Bgra8(Unsigned, Normalized)`, `SrgbBgra8`, and
    /// `RgbaFloat16`.
    fn new(
        wb: winit::WindowBuilder,
        events_loop: T,
        format: core::ImageFormat,
    ) -> Result<Self, InitializationError> {
        let pixel_format =
            backend_metal::imp::translate_image_format(format).expect("unsupported ImageFormat");
        let winit_window = wb.build(events_loop.deref()).unwrap();

        unsafe {
            let wnd: cocoa_id = mem::transmute(winit_window.get_nswindow());
            let layer: metal::CAMetalLayer = metal::CAMetalLayer::new();
            layer.set_pixel_format(pixel_format);

            let draw_size = winit_window.get_inner_size().unwrap();
            layer.set_edge_antialiasing_mask(0);
            layer.set_masks_to_bounds(true);
            // layer.set_magnification_filter(kCAFilterNearest);
            // layer.set_minification_filter(kCAFilterNearest);
            layer.set_drawable_size(NSSize::new(draw_size.0 as f64, draw_size.1 as f64));
            layer.set_presents_with_transaction(false);
            layer.remove_all_animations();

            let view = wnd.contentView();
            view.setWantsLayer(YES);
            view.setLayer(mem::transmute(layer.0));

            let metal_device = metal::create_system_default_device();
            layer.set_device(metal_device);

            let device = backend_metal::Device::new(metal_device);

            let drawable = layer.next_drawable().unwrap();

            Ok(MetalWindow {
                events_loop,
                window: winit_window,
                layer: layer,
                drawable: Cell::new(drawable),
                image_view: RefCell::new(ImageView::new(drawable.texture())),
                pool: Cell::new(metal::NSAutoreleasePool::alloc().init()),
                device: Arc::new(device),
            })
        }
    }
}

impl<T> wsi_core::Window for MetalWindow<T>
where
    T: Deref<Target = winit::EventsLoop>,
{
    type Backend = backend_metal::Backend;
    type CreationError = InitializationError;

    fn events_loop(&self) -> &winit::EventsLoop {
        self.events_loop.deref()
    }

    fn winit_window(&self) -> &winit::Window {
        &self.window
    }

    fn device(&self) -> &Arc<backend_metal::Device> {
        &self.device
    }

    fn acquire_framebuffer(&self) -> backend_metal::imp::ImageView {
        self.image_view.borrow().clone()
    }

    fn finalize_commands(&self, buffer: &mut backend_metal::imp::CommandBuffer) {
        buffer.metal_command_buffer().unwrap().present_drawable(
            self.drawable(),
        );
    }

    fn swap_buffers(&self) {
        unsafe {
            self.pool.get().release();
            self.pool.set(metal::NSAutoreleasePool::alloc().init());

            // FIXME: what if this fails?
            self.drawable.set(self.layer.next_drawable().expect(
                "I just don't know what went wrong! *hopping on a cloud*",
            ));
            *self.image_view.borrow_mut() = ImageView::new(self.drawable.get().texture());
        }
    }

    fn size(&self) -> Vector2<u32> {
        let texture = self.drawable.get().texture();
        Vector2::new(texture.width() as u32, texture.height() as u32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InitializationError {
    /// Could not create a window.
    Window,
    /// Unable to find a supported driver type.
    DriverType,
}
