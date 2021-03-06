//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use self::metal::NSObjectProtocol;
use std::collections::HashMap;
use std::mem::{self, ManuallyDrop};
use std::os::raw::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use winit::os::macos::WindowExt;
use winit::{EventsLoopProxy, Window};
use zangfx::backends::metal::metal;

use cocoa::appkit::{NSView, NSWindow};
use cocoa::base::id as cocoa_id;
use cocoa::foundation::{NSSize, NSString};
use objc::{msg_send, runtime::YES, sel, sel_impl};

use zangfx::backends::metal as be;
use zangfx::base as gfx;

use super::{AppInfo, GfxQueue, Painter, SurfaceProps, WindowOptions, WmDevice};
use crate::metalutils::OCPtr;

use super::cvdisplaylink::CVDisplayLink;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceRef(u32);

pub struct WindowManager<P: Painter> {
    painter: P,
    wm_device: ManuallyDrop<WmDevice>,
    device_data: ManuallyDrop<P::DeviceData>,
    surfaces: HashMap<SurfaceRef, Surface<P::SurfaceData>>,
    next_surface_id: u32,
    display_link: ManuallyDrop<CVDisplayLink>,
    refresh_state: Arc<RefreshState>,
}

struct Surface<D> {
    surface_data: D,
    layer: OCPtr<metal::CAMetalLayer>,
    window: Window,
}

#[derive(Debug)]
struct RefreshState {
    needs_update: AtomicBool,
}

impl<P: Painter> crate::Debug for WindowManager<P>
where
    P: crate::Debug,
    P::DeviceData: crate::Debug,
    P::SurfaceData: crate::Debug,
{
    fn fmt(&self, fmt: &mut crate::fmt::Formatter) -> crate::fmt::Result {
        fmt.debug_struct("WindowManager")
            .field("painter", &self.painter)
            .field("wm_device", &self.wm_device)
            .field("device_data", &self.device_data)
            .field("surfaces", &self.surfaces)
            .field("next_surface_id", &self.next_surface_id)
            .finish()
    }
}

impl<D> crate::Debug for Surface<D>
where
    D: crate::Debug,
{
    fn fmt(&self, fmt: &mut crate::fmt::Formatter) -> crate::fmt::Result {
        fmt.debug_struct("Surface")
            .field("surface_data", &self.surface_data)
            .field("layer", &self.layer)
            .finish()
    }
}

impl<P: Painter> Drop for WindowManager<P> {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.display_link) };

        // Remove all surfaces first
        for (surface_ref, surface) in self.surfaces.drain() {
            self.painter.remove_surface(
                &self.wm_device,
                &mut self.device_data,
                &surface_ref,
                surface.surface_data,
            );
        }

        // Remove the device from the painter
        let device_data = unsafe { ptr::read(&*self.device_data) };
        self.painter.remove_device(&self.wm_device, device_data);

        // Drop objects in the intended order
        let wm_device = unsafe { ptr::read(&*self.wm_device) };
        drop(wm_device.main_queue);
        drop(wm_device.device);
    }
}

fn resize_drawable(layer: &OCPtr<metal::CAMetalLayer>, window: &Window) -> bool {
    // we're sure the window exists
    let dpi_factor = window.get_hidpi_factor();
    let (mut w, mut h) = (window.get_inner_size().unwrap())
        .to_physical(dpi_factor)
        .into();
    if w == 0 {
        w = 1;
    }
    if h == 0 {
        h = 1;
    }

    let old_size = layer.drawable_size();
    let new_size = NSSize::new(w as f64, h as f64);

    (old_size.width == new_size.width && old_size.height == new_size.height) || {
        layer.set_drawable_size(new_size);
        false
    }
}

fn surface_props_from_layer(layer: &OCPtr<metal::CAMetalLayer>) -> SurfaceProps {
    let size = layer.drawable_size();

    SurfaceProps {
        extents: [size.width as u32, size.height as u32],
        format: be::formats::translate_metal_pixel_format(layer.pixel_format()),
    }
}

impl<P: Painter> WindowManager<P> {
    pub fn new(mut painter: P, events_loop_proxy: EventsLoopProxy, _app_info: &AppInfo) -> Self {
        let device = unsafe { be::device::Device::new_system_default().unwrap() };
        let device: gfx::DeviceRef = Arc::new(device);

        let main_queue = device
            .build_cmd_queue()
            .queue_family(be::QUEUE_FAMILY_UNIVERSAL)
            .build()
            .unwrap();

        let wm_device = WmDevice {
            device,
            main_queue: GfxQueue {
                queue: main_queue.into(),
                queue_family: be::QUEUE_FAMILY_UNIVERSAL,
            },
            copy_queue: None,
        };

        let device_data = painter.add_device(&wm_device);

        // Set up the display link
        let refresh_state = Arc::new(RefreshState {
            needs_update: AtomicBool::new(false),
        });

        let display_link = CVDisplayLink::new().unwrap();
        {
            let refresh_state = refresh_state.clone();
            display_link
                .set_output_callback(move |_, _, _, _| {
                    refresh_state.needs_update.store(true, Ordering::Relaxed);
                    let _ = events_loop_proxy.wakeup();
                })
                .unwrap();
        }
        display_link.start().unwrap();

        Self {
            painter,
            wm_device: ManuallyDrop::new(wm_device),
            device_data: ManuallyDrop::new(device_data),
            surfaces: HashMap::new(),
            next_surface_id: 0,
            display_link: ManuallyDrop::new(display_link),
            refresh_state,
        }
    }

    #[allow(dead_code)]
    pub fn painter_ref(&self) -> &P {
        &self.painter
    }

    #[allow(dead_code)]
    pub fn painter_mut(&mut self) -> &mut P {
        &mut self.painter
    }

    pub fn add_surface(
        &mut self,
        window: Window,
        options: &WindowOptions,
        param: P::SurfaceParam,
    ) -> SurfaceRef {
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn CGColorSpaceCreateWithName(name: cocoa_id) -> *const c_void;
            fn CGColorSpaceRelease(space: *const c_void);
        }

        self.next_surface_id = self.next_surface_id.checked_add(1).unwrap();
        let surface_id = SurfaceRef(self.next_surface_id);

        unsafe {
            let wnd: cocoa_id = mem::transmute(window.get_nswindow());
            let layer: metal::CAMetalLayer = metal::CAMetalLayer::new();
            layer.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm_sRGB);

            let cs_name = "kCGColorSpaceSRGB";
            let ns_cs_name = NSString::alloc(ptr::null_mut()).init_str(cs_name);
            let colorspace = CGColorSpaceCreateWithName(mem::transmute(ns_cs_name));
            let () = msg_send![ns_cs_name, release];

            layer.set_edge_antialiasing_mask(0);
            layer.set_masks_to_bounds(true);
            layer.set_opaque(!options.transparent);
            layer.set_colorspace(mem::transmute(colorspace));
            CGColorSpaceRelease(colorspace);
            // layer.set_magnification_filter(kCAFilterNearest);
            // layer.set_minification_filter(kCAFilterNearest);
            layer.set_framebuffer_only(true);
            layer.set_presents_with_transaction(false);
            layer.remove_all_animations();

            let view = wnd.contentView();
            view.setWantsLayer(YES);
            view.setLayer(mem::transmute(layer.0));

            let gfx_device: &be::device::Device = self.wm_device.device.query_ref().unwrap();
            layer.set_device(gfx_device.metal_device());

            let layer = OCPtr::new(layer).unwrap();

            resize_drawable(&layer, &window);
            let surface_props = surface_props_from_layer(&layer);
            let surface_data = self.painter.add_surface(
                &self.wm_device,
                &mut self.device_data,
                &surface_id,
                param,
                &surface_props,
            );

            let surface = Surface {
                surface_data,
                layer,
                window,
            };
            self.surfaces.insert(surface_id, surface);
        }

        surface_id
    }

    pub fn remove_surface(&mut self, surface_ref: SurfaceRef) {
        let surface = self.surfaces.remove(&surface_ref).unwrap();
        self.painter.remove_surface(
            &self.wm_device,
            &mut self.device_data,
            &surface_ref,
            surface.surface_data,
        );
    }

    pub fn get_winit_window(&self, surface_ref: SurfaceRef) -> Option<&Window> {
        self.surfaces.get(&surface_ref).map(|s| &s.window)
    }

    pub fn update(&mut self, update_param: &P::UpdateParam) {
        let needs_update = self
            .refresh_state
            .needs_update
            .swap(false, Ordering::Relaxed);
        if !needs_update {
            return;
        }

        struct Drawable {
            image: gfx::ImageRef,
            surface_props: SurfaceProps,
            metal_drawable: Option<OCPtr<metal::CAMetalDrawable>>,
            pixel_ratio: f32,
        }

        impl super::Drawable for Drawable {
            fn image(&self) -> &gfx::ImageRef {
                &self.image
            }

            fn pixel_ratio(&self) -> f32 {
                self.pixel_ratio
            }

            fn surface_props(&self) -> &SurfaceProps {
                &self.surface_props
            }

            fn encode_prepare_present(
                &mut self,
                cmd_buffer: &mut gfx::CmdBufferRef,
                _: gfx::QueueFamily,
                _: gfx::StageFlags,
                _: gfx::AccessTypeFlags,
            ) {
                let be_cb: &mut be::cmd::buffer::CmdBuffer =
                    cmd_buffer.query_mut().expect("bad command buffer type");

                let metal_cb = be_cb.metal_cmd_buffer().expect("CB is already committed");

                let metal_drawable = self
                    .metal_drawable
                    .take()
                    .expect("can't prepare the presentation twice");

                metal_cb.present_drawable(*metal_drawable);
            }

            fn enqueue_present(&mut self) {}
        }

        super::autorelease_pool_scope(|arp| {
            for (surface_ref, surface) in self.surfaces.iter_mut() {
                let ref layer = surface.layer;
                let ref window = surface.window;

                let surface_props;

                if resize_drawable(&layer, &window) {
                    // The window was resized -- send a notification
                    surface_props = surface_props_from_layer(&layer);
                    self.painter.update_surface(
                        &self.wm_device,
                        &mut self.device_data,
                        surface_ref,
                        &mut surface.surface_data,
                        &surface_props,
                    );
                } else {
                    surface_props = surface_props_from_layer(&layer);
                }

                if let Some(metal_drawable) = layer.next_drawable() {
                    let metal_texture = metal_drawable.texture();
                    unsafe {
                        metal_texture.retain();
                    }

                    let mut drawable = Drawable {
                        image: unsafe { be::image::Image::from_raw(metal_texture) }.into(),
                        surface_props,
                        metal_drawable: Some(OCPtr::new(metal_drawable).unwrap()),
                        pixel_ratio: window.get_hidpi_factor() as f32,
                    };

                    self.painter.paint(
                        &self.wm_device,
                        &mut self.device_data,
                        surface_ref,
                        &mut surface.surface_data,
                        update_param,
                        &mut drawable,
                    );
                }

                arp.drain();
            }
        });
    }
}
