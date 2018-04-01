//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Platform-specific code for surface creation.
use winit;
use super::utils::InstanceBuilder;
use super::ash::{self, vk, extensions::Surface, version::*};

#[cfg(windows)]
mod os {
    use super::*;

    pub fn create_surface<E: EntryV1_0, I: InstanceV1_0>(
        entry: &E,
        instance: &I,
        window: &winit::Window,
    ) -> Result<vk::SurfaceKHR, vk::Result> {
        use self::ash::extensions::Win32Surface;
        use winit::os::windows::WindowExt;
        let hwnd = window.get_hwnd() as *mut winapi::windef::HWND__;
        let hinstance = unsafe { user32::GetWindow(hwnd, 0) as *const () };
        let win32_create_info = vk::Win32SurfaceCreateInfoKHR {
            s_type: vk::StructureType::Win32SurfaceCreateInfoKhr,
            p_next: ::null(),
            flags: Default::default(),
            hinstance: hinstance,
            hwnd: hwnd as *const (),
        };
        let win32_surface_loader =
            Win32Surface::new(entry, instance).expect("Unable to load win32 surface");
        unsafe { win32_surface_loader.create_win32_surface_khr(&win32_create_info, None) }
    }

    pub fn modify_instance_builder(builder: &mut InstanceBuilder) {
        use self::ash::extensions::Win32Surface;
        builder.enable_extension(Surface::name().to_str().unwrap());
        builder.enable_extension(Win32Surface::name().to_str().unwrap());
    }
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
mod os {
    use super::*;

    pub fn create_surface<E: EntryV1_0, I: InstanceV1_0>(
        entry: &E,
        instance: &I,
        window: &winit::Window,
    ) -> Result<vk::SurfaceKHR, vk::Result> {
        use self::ash::extensions::XlibSurface;
        use winit::os::unix::WindowExt;
        let x11_display = window.get_xlib_display().unwrap();
        let x11_window = window.get_xlib_window().unwrap();
        let x11_create_info = vk::XlibSurfaceCreateInfoKHR {
            s_type: vk::StructureType::XlibSurfaceCreateInfoKhr,
            p_next: ::null(),
            flags: Default::default(),
            window: x11_window as vk::Window,
            dpy: x11_display as *mut vk::Display,
        };
        let xlib_surface_loader =
            XlibSurface::new(entry, instance).expect("Unable to load xlib surface");
        unsafe { xlib_surface_loader.create_xlib_surface_khr(&x11_create_info, None) }
    }

    pub fn modify_instance_builder(builder: &mut InstanceBuilder) {
        use self::ash::extensions::XlibSurface;
        builder.enable_extension(Surface::name().to_str().unwrap());
        builder.enable_extension(XlibSurface::name().to_str().unwrap());
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
    ) -> Result<vk::SurfaceKHR, vk::Result> {
        use self::ash::extensions::MacOSSurface;
        use winit::os::macos::WindowExt;
        use objc::runtime::YES;
        use cocoa::base::id as cocoa_id;
        use cocoa::appkit::{NSView, NSWindow};
        use std::mem::transmute;
        use zangfx::backends::metal::metal;

        let view;
        unsafe {
            let wnd: cocoa_id = transmute(window.get_nswindow());
            let layer: metal::CAMetalLayer = metal::CAMetalLayer::new();
            layer.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm_sRGB);

            layer.set_edge_antialiasing_mask(0);
            layer.set_masks_to_bounds(true);
            layer.set_opaque(true);
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
            s_type: vk::StructureType::MacOSSurfaceCreateInfoMvk,
            p_next: ::null(),
            flags: Default::default(),
            p_view: unsafe { transmute(view) },
        };
        let surface_loader = MacOSSurface::new(entry, instance)
            .expect("Unable to load the macOS surface entry points.");
        unsafe { surface_loader.create_macos_surface_mvk(&create_info, None) }
    }

    pub fn modify_instance_builder(builder: &mut InstanceBuilder) {
        use self::ash::extensions::MacOSSurface;
        builder.enable_extension(Surface::name().to_str().unwrap());
        builder.enable_extension(MacOSSurface::name().to_str().unwrap());
    }
}

pub use self::os::*;
