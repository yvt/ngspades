//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsGFX Window System Integration for Vulkan
//! ===========================================
//!
//! Partially based on [Ash]'s example.
//!
//! [Ash]: https://github.com/MaikKlein/ash

extern crate cgmath;
extern crate thunk;

extern crate ngsgfx_core as core;
extern crate ngsgfx_vulkan as backend_vulkan;
extern crate ngsgfx_wsi_core as wsi_core;

#[cfg(windows)]
extern crate winapi;
#[cfg(windows)]
extern crate user32;

mod colorspace;
mod swapchain;

use swapchain::*;
use colorspace::*;

use wsi_core::winit;
use backend_vulkan::ash;

use self::ash::vk;
use self::ash::extensions::Swapchain as SwapchainExt;
use self::ash::extensions::Surface;
use self::ash::version::{EntryV1_0, InstanceV1_0};

use std::{fmt, ptr};
use std::sync::Arc;

#[cfg(windows)]
pub type DefaultVulkanSurface = WindowsVulkanSurface;

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
pub type DefaultVulkanSurface = XlibVulkanSurface;

#[cfg(any(windows, all(unix, not(target_os = "macos"), not(target_os = "android"))))]
pub type DefaultVulkanWindow = VulkanWindow<DefaultVulkanSurface>;

pub trait VulkanSurface: 'static {
    fn create_surface<E: EntryV1_0, I: InstanceV1_0>(
        entry: &E,
        instance: &I,
        window: &winit::Window,
    ) -> Result<vk::SurfaceKHR, vk::Result>;
    fn modify_instance_builder(_: &mut backend_vulkan::InstanceBuilder) {}
}

#[cfg(windows)]
pub struct WindowsVulkanSurface;

#[cfg(windows)]
impl VulkanSurface for WindowsVulkanSurface {
    fn create_surface<E: EntryV1_0, I: InstanceV1_0>(
        entry: &E,
        instance: &I,
        window: &winit::Window,
    ) -> Result<vk::SurfaceKHR, vk::Result> {
        use ash::extensions::Win32Surface;
        use winit::os::windows::WindowExt;
        let hwnd = window.get_hwnd() as *mut winapi::windef::HWND__;
        let hinstance = unsafe { user32::GetWindow(hwnd, 0) as *const () };
        let win32_create_info = vk::Win32SurfaceCreateInfoKHR {
            s_type: vk::StructureType::Win32SurfaceCreateInfoKhr,
            p_next: ptr::null(),
            flags: Default::default(),
            hinstance: hinstance,
            hwnd: hwnd as *const (),
        };
        let win32_surface_loader =
            Win32Surface::new(entry, instance).expect("Unable to load win32 surface");
        unsafe { win32_surface_loader.create_win32_surface_khr(&win32_create_info, None) }
    }

    fn modify_instance_builder(builder: &mut backend_vulkan::InstanceBuilder) {
        use ash::extensions::Win32Surface;
        // TODO: check the result
        builder.enable_extension(Surface::name().to_str().unwrap());
        builder.enable_extension(Win32Surface::name().to_str().unwrap());
    }
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
pub struct XlibVulkanSurface;

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
impl VulkanSurface for XlibVulkanSurface {
    fn create_surface<E: EntryV1_0, I: InstanceV1_0>(
        entry: &E,
        instance: &I,
        window: &winit::Window,
    ) -> Result<vk::SurfaceKHR, vk::Result> {
        use ash::extensions::XlibSurface;
        use winit::os::unix::WindowExt;
        let x11_display = window.get_xlib_display().unwrap();
        let x11_window = window.get_xlib_window().unwrap();
        let x11_create_info = vk::XlibSurfaceCreateInfoKHR {
            s_type: vk::StructureType::XlibSurfaceCreateInfoKhr,
            p_next: ptr::null(),
            flags: Default::default(),
            window: x11_window as vk::Window,
            dpy: x11_display as *mut vk::Display,
        };
        let xlib_surface_loader =
            XlibSurface::new(entry, instance).expect("Unable to load xlib surface");
        unsafe { xlib_surface_loader.create_xlib_surface_khr(&x11_create_info, None) }
    }

    fn modify_instance_builder(builder: &mut backend_vulkan::InstanceBuilder) {
        use ash::extensions::XlibSurface;
        // TODO: check the result
        builder.enable_extension(Surface::name().to_str().unwrap());
        builder.enable_extension(XlibSurface::name().to_str().unwrap());
    }
}

// TODO: support Wayland and Mir

pub struct VulkanWindow<S: VulkanSurface> {
    window: winit::Window,
    device: Arc<backend_vulkan::Device<backend_vulkan::ManagedDeviceRef>>,
    phantom: ::std::marker::PhantomData<S>,
}

impl<S: VulkanSurface> fmt::Debug for VulkanWindow<S> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("VulkanWindow").finish()
    }
}

#[derive(Debug, Clone)]
pub enum InitializationError {
    VkError(vk::Result),
    LoadError(Vec<&'static str>),
    NoCompatibleDevice,
    NoCompatibleFormat,
    DeviceBuildError(backend_vulkan::imp::DeviceBuildError),
}

impl<S: VulkanSurface> wsi_core::NewWindow for VulkanWindow<S> {
    type Environment = backend_vulkan::ManagedEnvironment;
    type CreationError = InitializationError;

    /// Constructs a new `VulkanWindow`.
    ///
    /// `format` is currently ignored.
    fn new(
        wb: winit::WindowBuilder,
        events_loop: &winit::EventsLoop,
        instance: &backend_vulkan::Instance,
        sc_desc: &wsi_core::SwapchainDescription,
    ) -> Result<Self, InitializationError> {
        use core::{Instance, DeviceBuilder};
        use backend_vulkan::DeviceRef;

        let winit_window = wb.build(events_loop).unwrap();

        let vk_entry = instance.entry();
        let vk_instance = instance.instance();
        let surface = S::create_surface(vk_entry, vk_instance, &winit_window)
            .map_err(InitializationError::VkError)?;
        // TODO: put this into a smart pointer

        let adapters = instance.adapters();
        let surface_loader = Surface::new(vk_entry, vk_instance).map_err(
            InitializationError::LoadError,
        )?;

        // Find a suitable adapter
        let adap = adapters
            .iter()
            .filter_map(|a| {
                // TODO: support queue families other than universal one for presentation
                let device_config = a.device_config();
                let univ_iq = device_config.engine_queue_mappings.universal;
                let univ_qf = device_config.queues[univ_iq].0;
                if surface_loader.get_physical_device_surface_support_khr(
                    a.physical_device(),
                    univ_qf,
                    surface,
                )
                {
                    Some(a)
                } else {
                    None
                }
            })
            .nth(0)
            .ok_or(InitializationError::NoCompatibleDevice)?;

        let mut device_builder = instance.new_device_builder(adap);
        device_builder.enable_extension(SwapchainExt::name().to_str().unwrap());

        let device = device_builder.build().map_err(
            InitializationError::DeviceBuildError,
        )?;
        let surface_formats = surface_loader
            .get_physical_device_surface_formats_khr(adap.physical_device(), surface)
            .unwrap(); // TODO: handle this error

        let (vk_format, vk_color_space) = choose_visual(sc_desc.desired_formats, || {
            surface_formats.iter().map(|x| (x.format, x.color_space))
        }).ok_or(InitializationError::NoCompatibleFormat)?;

        let surface_cap = surface_loader
            .get_physical_device_surface_capabilities_khr(adap.physical_device(), surface)
            .unwrap(); // TODO: handle this error
        let mut image_count = surface_cap.min_image_count + 1;
        if surface_cap.max_image_count > 0 && image_count > surface_cap.max_image_count {
            image_count = surface_cap.max_image_count;
        }

        // On Win32 and Xlib, `current_extent` is the window size
        let window_size = winit_window.get_inner_size_pixels().unwrap(); // we're sure the window exists
        let surface_size = match surface_cap.current_extent.width {
            std::u32::MAX => {
                vk::Extent2D {
                    width: window_size.0,
                    height: window_size.1,
                }
            }
            _ => surface_cap.current_extent,
        };

        let pre_transform = surface_cap.current_transform;

        let image_usage = backend_vulkan::imp::translate_image_usage(sc_desc.image_usage);
        assert!(
            surface_cap.supported_usage_flags.subset(image_usage),
            "specified image usage is not supported"
        ); // TODO: fall-back or something

        // `Fifo` is always supported
        let present_mode = vk::PresentModeKHR::Fifo;

        let swapchain_loader = SwapchainExt::new(vk_instance, device.device_ref().device())
            .map_err(InitializationError::LoadError)?;

        let swapchain_create_info = vk::SwapchainCreateInfoKHR {
            s_type: vk::StructureType::SwapchainCreateInfoKhr,
            p_next: ptr::null(),
            flags: vk::SwapchainCreateFlagsKHR::empty(),
            surface,
            min_image_count: image_count,
            image_format: vk_format,
            image_color_space: vk_color_space,
            image_extent: surface_size.clone(),
            image_array_layers: 1,
            image_usage,
            image_sharing_mode: vk::SharingMode::Exclusive,
            p_queue_family_indices: ptr::null(), // ignored for `Exclusive`
            queue_family_index_count: 0,
            pre_transform,
            composite_alpha: vk::COMPOSITE_ALPHA_OPAQUE_BIT_KHR, // TODO: is this required?
            present_mode,
            clipped: vk::VK_TRUE,
            old_swapchain: vk::SwapchainKHR::null(),
        };
        let swapchain = unsafe {
            swapchain_loader.create_swapchain_khr(&swapchain_create_info, None)
        }.expect("swapchain creation failed"); // TODO: handle this error
        // TODO: handle resize

        let _ = swapchain; // TODO: implement and use NgsGFX standard WSI interface

        Ok(VulkanWindow {
            window: winit_window,
            device: Arc::new(device),
            phantom: Default::default(),
        })
    }

    fn modify_instance_builder(builder: &mut backend_vulkan::InstanceBuilder) {
        S::modify_instance_builder(builder);
    }
}

impl<S: VulkanSurface> wsi_core::Window for VulkanWindow<S> {
    type Backend = backend_vulkan::ManagedBackend;
    type Swapchain = Swapchain;

    fn winit_window(&self) -> &winit::Window {
        &self.window
    }

    fn device(&self) -> &Arc<<Self::Backend as core::Backend>::Device> {
        &self.device
    }

    fn swapchain(&self) -> &Self::Swapchain {
        unimplemented!()
    }
    fn update_swapchain(&self) {
        unimplemented!()
    }
}
