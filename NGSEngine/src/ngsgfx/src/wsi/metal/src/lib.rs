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
extern crate winit;
extern crate cgmath;

extern crate ngsgfx_core as core;
extern crate ngsgfx_metal as backend_metal;
use backend_metal::ll as metal;

use std::sync::Arc;
use std::cell::{Cell, RefCell, Ref};
use std::{mem, fmt};

use cgmath::Vector2;

use objc::runtime::YES;

use metal::NSObjectProtocol;

use backend_metal::imp::ImageView;

use cocoa::base::id as cocoa_id;
use cocoa::foundation::NSSize;
use cocoa::appkit::{NSWindow, NSView};

use winit::os::macos::WindowExt;

pub struct MetalWindow {
    window: winit::Window,
    layer: metal::CAMetalLayer,
    drawable: Cell<metal::CAMetalDrawable>,
    image_view: RefCell<ImageView>,
    pool: Cell<metal::NSAutoreleasePool>,
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
    pub fn winit_window(&self) -> &winit::Window {
        &self.window
    }

    pub fn device(&self) -> &Arc<backend_metal::Device> {
        &self.device
    }

    pub fn image_view(&self) -> Ref<ImageView> {
        self.image_view.borrow()
    }

    pub fn size(&self) -> Vector2<u32> {
        let texture = self.drawable.get().texture();
        Vector2::new(texture.width() as u32, texture.height() as u32)
    }

    pub fn swap_buffers(&self) {
        unsafe {
            self.pool.get().release();
            self.pool.set(metal::NSAutoreleasePool::alloc().init());

            // FIXME: what if this fails?
            self.drawable.set(self.layer.next_drawable()
                .expect("I just don't know what went wrong! *hopping on a cloud*"));
            *self.image_view.borrow_mut() = ImageView::new(self.drawable.get().texture());
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InitializationError {
    /// Could not create a window.
    Window,
    /// Unable to find a supported driver type.
    DriverType,
}

/// Constructs a new `MetalWindow`.
///
/// `format` must be one of `Bgra8(Unsigned, Normalized)`, `SrgbBgra8`, and
/// `RgbaFloat16`.
pub fn make_window(wb: winit::WindowBuilder, events_loop: &winit::EventsLoop, format: core::ImageFormat)
    -> Result<MetalWindow, InitializationError>
{
    let pixel_format = backend_metal::imp::translate_image_format(format)
        .expect("unsupported ImageFormat");
    let winit_window = wb.build(events_loop).unwrap();

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

        Ok(MetalWindow{
            window: winit_window,
            layer: layer,
            drawable: Cell::new(drawable),
            image_view: RefCell::new(ImageView::new(drawable.texture())),
            pool: Cell::new(metal::NSAutoreleasePool::alloc().init()),
            device: Arc::new(device),
        })
    }
}
