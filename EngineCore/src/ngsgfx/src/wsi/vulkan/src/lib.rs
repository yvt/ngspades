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
//!
//! Restrictions
//! ------------
//!
//!  - You must specify exactly one internal queue by `FrameDescription::acquiring_engines`.
//!    This is due to the `VK_KHR_swapchain`'s restriction. There exist no efficient solutions.
//!  - After calling `Swapchain::next_drawable`, you must submit a command buffer containing
//!    a fence wait operation on the returned drawable's `acquiring_fence` before caling
//!    `Swapchain::next_drawable` again. This should not be a problem for most applications.
//!

extern crate cgmath;
extern crate thunk;

extern crate ngsgfx_core as core;
extern crate ngsgfx_vulkan as backend_vulkan;
extern crate ngsgfx_wsi_core as wsi_core;
extern crate ngsgfx_common;

#[cfg(windows)]
extern crate winapi;
#[cfg(windows)]
extern crate user32;

mod colorspace;
mod swapchain;

pub use swapchain::*;
pub use colorspace::*;

use wsi_core::winit;
use backend_vulkan::ash;

use self::ash::vk;
use self::ash::extensions::Swapchain as SwapchainExt;
use self::ash::extensions::Surface;
use self::ash::version::{EntryV1_0, InstanceV1_0};

use std::{fmt, ptr};
use std::sync::Arc;

pub type ManagedDevice = backend_vulkan::Device<backend_vulkan::ManagedDeviceRef>;
pub type ManagedImage = backend_vulkan::imp::Image<backend_vulkan::ManagedDeviceRef>;

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

// TODO: Specifying surface extension via generic parameter turned out to be a bad idea. Remove it

pub struct VulkanWindow<S: VulkanSurface> {
    window: winit::Window,
    device: Arc<ManagedDevice>,
    phys_device: vk::PhysicalDevice,
    surface_loader: Surface,
    surface: vk::SurfaceKHR,
    swapchain: Option<Swapchain>,
    transparent: bool,

    scd_desired_formats: Vec<(Option<core::ImageFormat>, Option<wsi_core::ColorSpace>)>,
    scd_image_usage: core::ImageUsageFlags,

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
        use ash::version::DeviceV1_0;

        let transparent = wb.window.transparent;
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

        let swapchain_loader = SwapchainExt::new(vk_instance, device.device_ref().device())
            .map_err(InitializationError::LoadError)?;

        let swapchain_create_info = make_swapchain_create_info(
            sc_desc,
            adap.physical_device(),
            &surface_loader,
            surface,
            &winit_window,
            transparent,
        )?;

        let swapchain = unsafe {
            swapchain_loader.create_swapchain_khr(&swapchain_create_info, None)
        }.expect("swapchain creation failed"); // TODO: handle this error
        // TODO: handle resize

        let device = Arc::new(device);

        // TODO: support presentation from other than the universal queue
        let univ_iq = device.config().engine_queue_mappings.universal;
        let univ_qf_qi = device.config().queues[univ_iq];
        let univ_q = unsafe {
            device.device_ref().device().get_device_queue(
                univ_qf_qi.0,
                univ_qf_qi.1,
            )
        };
        let wsi_swapchain = unsafe {
            Swapchain::from_raw(SwapchainConfig {
                device: device.clone(),
                swapchain_loader,
                swapchain,
                present_queue: univ_q,
                present_queue_family: univ_qf_qi.0,
                info: drawable_info_from_swapchain_info(&swapchain_create_info),
            })
        }.expect("Swapchain::from_raw failed"); // TDOO: handle this error

        Ok(VulkanWindow {
            window: winit_window,
            device,
            phys_device: adap.physical_device(),
            surface_loader,
            surface,
            transparent,
            swapchain: Some(wsi_swapchain),
            scd_desired_formats: sc_desc.desired_formats.iter().map(Clone::clone).collect(),
            scd_image_usage: sc_desc.image_usage,
            phantom: Default::default(),
        })
    }

    fn modify_instance_builder(builder: &mut backend_vulkan::InstanceBuilder) {
        S::modify_instance_builder(builder);
    }
}

fn make_swapchain_create_info(
    sc_desc: &wsi_core::SwapchainDescription,
    phys_device: vk::PhysicalDevice,
    surface_loader: &Surface,
    surface: vk::SurfaceKHR,
    winit_window: &winit::Window,
    transparent: bool,
) -> Result<vk::SwapchainCreateInfoKHR, InitializationError> {
    let surface_formats = surface_loader
        .get_physical_device_surface_formats_khr(phys_device, surface)
        .unwrap(); // TODO: handle this error

    let (vk_format, vk_color_space) = choose_visual(sc_desc.desired_formats, || {
        surface_formats.iter().map(|x| (x.format, x.color_space))
    }).ok_or(InitializationError::NoCompatibleFormat)?;

    let surface_cap = surface_loader
        .get_physical_device_surface_capabilities_khr(phys_device, surface)
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

    let composite_alpha = if transparent {
        if surface_cap.supported_composite_alpha.intersects(
            vk::COMPOSITE_ALPHA_PRE_MULTIPLIED_BIT_KHR,
        )
        {
            vk::COMPOSITE_ALPHA_PRE_MULTIPLIED_BIT_KHR
        } else {
            // fall-back
            vk::COMPOSITE_ALPHA_OPAQUE_BIT_KHR
        }
    } else {
        vk::COMPOSITE_ALPHA_OPAQUE_BIT_KHR
    };

    // `Fifo` is always supported
    let present_mode = vk::PresentModeKHR::Fifo;

    Ok(vk::SwapchainCreateInfoKHR {
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
        composite_alpha,
        present_mode,
        clipped: vk::VK_TRUE,
        old_swapchain: vk::SwapchainKHR::null(),
    })
}

impl<S: VulkanSurface> Drop for VulkanWindow<S> {
    fn drop(&mut self) {
        self.swapchain.take();
        unsafe {
            self.surface_loader.destroy_surface_khr(self.surface, None);
        }
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
        self.swapchain.as_ref().unwrap()
    }
    fn update_swapchain(&mut self) {
        let mut new_info = make_swapchain_create_info(
            &wsi_core::SwapchainDescription {
                desired_formats: &self.scd_desired_formats,
                image_usage: self.scd_image_usage,
            },
            self.phys_device,
            &self.surface_loader,
            self.surface,
            &self.window,
            self.transparent,
        ).unwrap();
        let sc_cfg = self.swapchain().config().clone();
        new_info.old_swapchain = sc_cfg.swapchain;

        let swapchain = unsafe {
            sc_cfg.swapchain_loader.create_swapchain_khr(
                &new_info,
                None,
            )
        }.expect("swapchain creation failed"); // TODO: handle this error

        self.swapchain = Some(
            unsafe {
                Swapchain::from_raw(SwapchainConfig {
                    device: self.device.clone(),
                    swapchain_loader: sc_cfg.swapchain_loader.clone(),
                    swapchain,
                    present_queue: sc_cfg.present_queue,
                    present_queue_family: sc_cfg.present_queue_family,
                    info: drawable_info_from_swapchain_info(&new_info),
                })
            }.expect("Swapchain::from_raw failed"),
        );
    }
}
