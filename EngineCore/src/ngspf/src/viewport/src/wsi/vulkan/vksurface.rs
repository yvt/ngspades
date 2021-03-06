//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Platform-specific code for surface creation.
use super::super::WindowOptions;
use super::ash::{self, extensions::khr::Surface, version::*, vk};
use super::utils::InstanceBuilder;
use winit;

#[cfg(windows)]
mod os {
    extern crate user32;
    extern crate winapi;

    use super::*;

    pub fn create_surface<E: EntryV1_0, I: InstanceV1_0>(
        entry: &E,
        instance: &I,
        window: &winit::Window,
        _options: &WindowOptions,
    ) -> Result<vk::SurfaceKHR, vk::Result> {
        use self::ash::extensions::khr::Win32Surface;
        use winit::os::windows::WindowExt;
        let hwnd = window.get_hwnd() as *mut _;
        let hinstance = unsafe { user32::GetWindow(hwnd, 0) as *const () };
        let win32_create_info = vk::Win32SurfaceCreateInfoKHR {
            s_type: vk::StructureType::WIN32_SURFACE_CREATE_INFO_KHR,
            p_next: std::ptr::null(),
            flags: Default::default(),
            hinstance: hinstance as *const _,
            hwnd: hwnd as *const _,
        };
        let win32_surface_loader = Win32Surface::new(entry, instance);
        unsafe { win32_surface_loader.create_win32_surface(&win32_create_info, None) }
    }

    pub fn modify_instance_builder(builder: &mut InstanceBuilder) {
        use self::ash::extensions::Win32Surface;
        builder.enable_extension(Surface::name().to_str().unwrap());
        builder.enable_extension(Win32Surface::name().to_str().unwrap());
    }
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
mod os {
    use self::ash::extensions::khr::{WaylandSurface, XlibSurface};
    use super::*;

    pub fn create_surface<E: EntryV1_0, I: InstanceV1_0>(
        entry: &E,
        instance: &I,
        window: &winit::Window,
        _options: &WindowOptions,
    ) -> Result<vk::SurfaceKHR, vk::Result> {
        use winit::os::unix::WindowExt;

        // Try Wayland first
        let wl_display = window.get_wayland_display();
        let wl_surface = window.get_wayland_surface();

        if let (Some(wl_display), Some(wl_surface)) = (wl_display, wl_surface) {
            let wl_create_info = vk::WaylandSurfaceCreateInfoKHR {
                s_type: vk::StructureType::WAYLAND_SURFACE_CREATE_INFO_KHR,
                p_next: std::ptr::null(),
                flags: Default::default(),
                surface: wl_surface as *mut _,
                display: wl_display as *mut _,
            };
            let wl_surface_loader = WaylandSurface::new(entry, instance);
            unsafe {
                return wl_surface_loader.create_wayland_surface(&wl_create_info, None);
            }
        }

        let x11_display = window.get_xlib_display().unwrap();
        let x11_window = window.get_xlib_window().unwrap();
        let x11_create_info = vk::XlibSurfaceCreateInfoKHR {
            s_type: vk::StructureType::XLIB_SURFACE_CREATE_INFO_KHR,
            p_next: std::ptr::null(),
            flags: Default::default(),
            window: x11_window as vk::Window,
            dpy: x11_display as *mut vk::Display,
        };
        let xlib_surface_loader = XlibSurface::new(entry, instance);
        unsafe { xlib_surface_loader.create_xlib_surface(&x11_create_info, None) }
    }

    pub fn modify_instance_builder(builder: &mut InstanceBuilder) {
        builder.enable_extension(Surface::name().to_str().unwrap());
        builder.enable_extension(XlibSurface::name().to_str().unwrap());
        builder.enable_extension(WaylandSurface::name().to_str().unwrap());
    }
}

// TODO: support Wayland and Mir

#[cfg(target_os = "macos")]
mod os {
    use super::*;

    pub fn create_surface<E: EntryV1_0, I: InstanceV1_0>(
        entry: &E,
        instance: &I,
        window: &winit::Window,
        options: &WindowOptions,
    ) -> Result<vk::SurfaceKHR, vk::Result> {
        use self::ash::extensions::mvk::MacOSSurface;
        use cocoa::appkit::{NSView, NSWindow};
        use cocoa::base::id as cocoa_id;
        use objc::runtime::YES;
        use std::mem::transmute;
        use winit::os::macos::WindowExt;
        use zangfx::backends::metal::metal;

        let view;
        unsafe {
            let wnd: cocoa_id = transmute(window.get_nswindow());
            let layer: metal::CAMetalLayer = metal::CAMetalLayer::new();
            layer.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm_sRGB);

            layer.set_edge_antialiasing_mask(0);
            layer.set_masks_to_bounds(true);
            layer.set_opaque(!options.transparent);
            // layer.set_magnification_filter(kCAFilterNearest);
            // layer.set_minification_filter(kCAFilterNearest);
            layer.set_framebuffer_only(true);
            layer.set_presents_with_transaction(false);
            layer.remove_all_animations();

            view = wnd.contentView();
            view.setWantsLayer(YES);
            view.setLayer(transmute(layer.0));
        }

        let create_info = vk::MacOSSurfaceCreateInfoMVK {
            s_type: vk::StructureType::MACOS_SURFACE_CREATE_INFO_M,
            p_next: std::ptr::null(),
            flags: Default::default(),
            p_view: unsafe { transmute(view) },
        };
        let surface_loader = MacOSSurface::new(entry, instance);
        unsafe { surface_loader.create_mac_os_surface_mvk(&create_info, None) }
    }

    pub fn modify_instance_builder(builder: &mut InstanceBuilder) {
        use self::ash::extensions::mvk::MacOSSurface;
        builder.enable_extension(Surface::name().to_str().unwrap());
        builder.enable_extension(MacOSSurface::name().to_str().unwrap());
    }
}

pub use self::os::*;
