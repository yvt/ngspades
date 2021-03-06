//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! The Vulkan WSI backend.
//!
//! # Naming conventions
//!
//! - `GfxXXX`: ZanGFX base abstract objects
//! - `BeXXX`: ZanGFX Vulkan backend's concrete objects, or something that
//!   pertains to thme
//! - `VkXXX`: Vulkan type, or something that pertains to it
extern crate atomic_refcell;

use flags_macro::flags;
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::sync::Arc;
use winit::{EventsLoopProxy, Window};

use zangfx::{
    backends::vulkan::{
        self as be,
        ash::{self, extensions as ext, version::*, vk},
        cmd::buffer::CmdBuffer as BeCmdBuffer,
        cmd::queue::CmdQueue as BeCmdQueue,
        cmd::semaphore::Semaphore as BeSemaphore,
    },
    base::{self as gfx, Error, ErrorKind, Result as GfxResult},
    prelude::*,
    utils::CbStateTracker,
};

use super::{AppInfo, GfxQueue, Painter, SurfaceProps, WindowOptions, WmDevice};

mod debugreport;
mod smartptr;
mod swapmanager;
mod utils;
mod vksurface;
use self::smartptr::{AutoPtr, UniqueDevice, UniqueInstance, UniqueSurfaceKHR, UniqueSwapchainKHR};
use self::swapmanager::{PresentError, PresentInfo, SwapchainManager};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceRef(DeviceId, SurfaceId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DeviceId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SurfaceId(u32);

pub struct WindowManager<P: Painter> {
    painter: P,
    events_loop_proxy: EventsLoopProxy,
    entry: ash::Entry,
    instance: ManuallyDrop<UniqueInstance>,
    surface_loader: ext::khr::Surface,
    report_conduit: ManuallyDrop<Option<debugreport::DebugReportConduit>>,

    /// Known and compatible physical devices.
    phys_device_info_list: Vec<Arc<PhysicalDeviceInfo>>,
    phys_device_list: HashMap<DeviceId, PhysicalDevice<P>>,

    next_device_id: u32,
    next_surface_id: u32,
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
            .field("entry", &())
            .field("instance", &self.instance)
            .field("surface_loader", &())
            .field("report_conduit", &self.report_conduit)
            .field("phys_device_info_list", &self.phys_device_info_list)
            .field("phys_device_list", &self.phys_device_list)
            .field("next_device_id", &self.next_device_id)
            .field("next_surface_id", &self.next_surface_id)
            .finish()
    }
}

impl<P: Painter> Drop for WindowManager<P> {
    fn drop(&mut self) {
        for (_, mut phys_device) in self.phys_device_list.drain() {
            phys_device.finalize(&mut self.painter, &self.surface_loader);
        }

        unsafe {
            ManuallyDrop::drop(&mut self.report_conduit);
            ManuallyDrop::drop(&mut self.instance);
        }
    }
}

impl<P: Painter> WindowManager<P> {
    pub fn new(painter: P, events_loop_proxy: EventsLoopProxy, app_info: &AppInfo) -> Self {
        // Initialize Vulkan
        let entry = ash::Entry::new().expect("Failed to load the Vulkan runtime library");

        let instance;
        let enable_debug_report;
        {
            let mut builder = utils::InstanceBuilder::new(&entry).unwrap();

            // Enable the debug report if supported and we're running the debug
            // build.
            let debug_report_name = "VK_EXT_debug_report";
            enable_debug_report =
                cfg!(debug_assertions) && builder.supports_extension(debug_report_name);
            if enable_debug_report {
                builder.enable_extension(debug_report_name);
            }

            // Enable the validation layers if we're running the debug build.
            let standard_validation_name = "VK_LAYER_LUNARG_standard_validation";
            if cfg!(debug_assertions) && builder.supports_layer(standard_validation_name) {
                builder.enable_layer(standard_validation_name);
            }

            // Enable surface extensions
            vksurface::modify_instance_builder(&mut builder);

            instance = builder
                .build(app_info)
                .expect("Failed to create a Vulkan instance.");
        }

        // Set up the debug report handler
        let report_conduit = if enable_debug_report {
            let mut report_conduit = debugreport::DebugReportConduit::new(&entry, &instance);

            let flags = flags![debugreport::DebugReportTypeFlags::
                {WARNING | PERFORMANCE_WARNING | ERROR}];
            report_conduit
                .add_handler(flags, Arc::new(debugreport::PrintDebugReportHandler::new()));

            Some(report_conduit)
        } else {
            None
        };

        let surface_loader = ext::khr::Surface::new(&entry, &*instance);

        // Enumerate physical devices
        let vk_phys_devices = unsafe { instance.enumerate_physical_devices() }
            .expect("Failed to enumerate available Vulkan physical devices.");
        let phys_device_info_list: Vec<_> = vk_phys_devices
            .iter()
            .filter_map(|vk_phys_device| {
                match PhysicalDeviceInfo::new(&*instance, *vk_phys_device) {
                    Ok(Some(x)) => Some(Ok(Arc::new(x))),
                    Ok(None) => None,
                    Err(x) => Some(Err(x)),
                }
            })
            .collect::<Result<_, _>>()
            .expect("Failed to examine the properties of Vulkan physical devices.");

        Self {
            painter,
            events_loop_proxy,
            entry,
            instance: ManuallyDrop::new(instance),
            surface_loader,
            report_conduit: ManuallyDrop::new(report_conduit),

            phys_device_info_list,
            phys_device_list: HashMap::new(),

            next_device_id: 1,
            next_surface_id: 1,
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
        let vk_surface = vksurface::create_surface(&self.entry, &**self.instance, &window, options)
            .expect("Failed to create a Vulkan surface.");
        let vk_surface = UniqueSurfaceKHR(&self.surface_loader, vk_surface);

        // Try to reuse an existing `PhysicalDevice`
        let mut phys_device_id = None;

        for (&id, phys_device) in self.phys_device_list.iter() {
            if phys_device.is_compatible_with_surface(&self.surface_loader, *vk_surface) {
                phys_device_id = Some(id);
                break;
            }
        }

        if phys_device_id.is_none() {
            // Find a compatible `PhysicalDeviceInfo` and create a new
            // `PhysicalDevice`
            let (info, queue_family) = self
                .phys_device_info_list
                .iter()
                .filter_map(|info| {
                    info.queue_family_compatible_with_surface(&self.surface_loader, *vk_surface)
                        .map(|qf| (info, qf))
                })
                .nth(0)
                .expect("Failed to find a compatible Vulkan physical device for a surface.");

            self.next_device_id = self.next_device_id.checked_add(1).unwrap();
            let device_id = DeviceId(self.next_device_id);
            phys_device_id = Some(device_id);

            self.phys_device_list.reserve(1);

            let phys_device = PhysicalDevice::new(
                &**self.instance,
                info,
                queue_family,
                &mut self.painter,
                self.events_loop_proxy.clone(),
            )
            .expect("Failed to initialize a Vulkan device.");

            self.phys_device_list.insert(device_id, phys_device);
        }

        let phys_device_id = phys_device_id.unwrap();

        self.next_surface_id = self.next_surface_id.checked_add(1).unwrap();
        let surface_id = SurfaceId(self.next_surface_id);
        let surface_ref = SurfaceRef(phys_device_id, surface_id);

        self.phys_device_list
            .get_mut(&phys_device_id)
            .unwrap()
            .add_surface(
                window,
                options,
                surface_ref,
                param,
                vk_surface,
                &self.surface_loader,
                &mut self.painter,
            );

        surface_ref
    }

    pub fn remove_surface(&mut self, surface_ref: SurfaceRef) {
        self.phys_device_list
            .get_mut(&surface_ref.0)
            .unwrap()
            .remove_surface(surface_ref, &self.surface_loader, &mut self.painter);

        // Defer the device deletion for faster recreation of surfaces
    }

    pub fn get_winit_window(&self, surface_ref: SurfaceRef) -> Option<&Window> {
        self.phys_device_list[&surface_ref.0].get_winit_window(surface_ref)
    }

    pub fn update(&mut self, update_param: &P::UpdateParam) {
        for (_, phys_device) in self.phys_device_list.iter_mut() {
            phys_device.update(update_param, &self.surface_loader, &mut self.painter);
        }
    }
}

/// A single Vulkan physical device recognized and activated by `WindowManager`.
///
/// Note that one `PhysicalDeviceInstance` can only have a single presentation
/// queue.
struct PhysicalDevice<P: Painter> {
    info: Arc<PhysicalDeviceInfo>,

    vk_device: UniqueDevice,
    swapchain_loader: ext::khr::Swapchain,

    swapchain_manager: ManuallyDrop<SwapchainManager>,
    surfaces: HashMap<SurfaceRef, Surface<P>>,

    wm_device: ManuallyDrop<WmDevice>,

    /// The queue used for presentation. Identical with `wm_device.main_queue`
    /// iff `presentation_queue_family == info.main_queue_family`
    presentation_queue: ManuallyDrop<Arc<gfx::CmdQueue>>,
    /// The queue family index used for presentation.
    presentation_queue_family: gfx::QueueFamily,

    device_data: Option<P::DeviceData>,
}

impl<P: Painter> crate::Debug for PhysicalDevice<P>
where
    P::DeviceData: crate::Debug,
    P::SurfaceData: crate::Debug,
{
    fn fmt(&self, fmt: &mut crate::fmt::Formatter) -> crate::fmt::Result {
        fmt.debug_struct("PhysicalDevice")
            .field("info", &self.info)
            .field("vk_device", &self.vk_device)
            .field("wm_device", &self.wm_device)
            .field("presentation_queue", &self.presentation_queue)
            .field("presentation_queue_family", &self.presentation_queue_family)
            .field("device_data", &self.device_data)
            .field("swapchain_manager", &self.swapchain_manager)
            .field("surfaces", &self.surfaces)
            .finish()
    }
}

impl<P: Painter> Drop for PhysicalDevice<P> {
    fn drop(&mut self) {
        assert!(self.device_data.is_none());
        assert!(self.surfaces.len() == 0);

        // Drop the GFX `Device` before destroying `VkDevice`
        unsafe {
            ManuallyDrop::drop(&mut self.swapchain_manager);
            ManuallyDrop::drop(&mut self.presentation_queue);
        }

        // Drop objects in the right order
        use std::ptr::read;
        let mut wm_device = unsafe { read(&*self.wm_device) };
        drop(wm_device.main_queue);
        drop(wm_device.copy_queue);

        be::device::Device::teardown_ref(&mut wm_device.device);
        drop(wm_device.device);

        // Alleviate some instabilities with error handling by inserting a device-global
        // sync here. (Usually, the device is supposed to be idle here)
        let _ = unsafe { self.vk_device.device_wait_idle() };
    }
}

impl<P: Painter> PhysicalDevice<P> {
    fn new(
        instance: &ash::Instance,
        info: &Arc<PhysicalDeviceInfo>,
        presentation_queue_family: gfx::QueueFamily,
        painter: &mut P,
        events_loop_proxy: EventsLoopProxy,
    ) -> GfxResult<Self> {
        let mut config = be::limits::DeviceConfig::new();

        // The number of queues for each queue family
        let mut num_queues = [0u32; 32];

        macro_rules! push_queue {
            ($queue_family:expr) => {{
                config
                    .queues
                    .push(($queue_family, num_queues[$queue_family as usize]));
                num_queues[$queue_family as usize] += 1;
            }};
        }

        push_queue!(info.main_queue_family);
        if let Some(queue_family) = info.copy_queue_family {
            push_queue!(queue_family);
        }
        if presentation_queue_family != info.main_queue_family {
            push_queue!(presentation_queue_family);
        }

        let queue_create_infos: Vec<_> = num_queues
            .iter()
            .enumerate()
            .filter_map(|(queue_family, &count)| {
                if count > 0 {
                    Some(ash::vk::DeviceQueueCreateInfo {
                        s_type: ash::vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
                        p_next: crate::null(),
                        flags: ash::vk::DeviceQueueCreateFlags::empty(),
                        queue_family_index: queue_family as u32,
                        queue_count: count,
                        p_queue_priorities: [0.5f32, 0.5f32].as_ptr(),
                    })
                } else {
                    None
                }
            })
            .collect();

        let vk_device = {
            let mut builder =
                utils::DeviceBuilder::new(instance, info.vk_phys_device).map_err(|e| {
                    Error::with_detail(
                        ErrorKind::Other,
                        format!(
                            "Failed to query the properties of a Vulkan physical device.: {:?}",
                            e
                        ),
                    )
                })?;

            builder.enable_extension(ext::khr::Swapchain::name().to_str().unwrap());

            builder
                .build(queue_create_infos.as_slice(), &info.enabled_features)
                .map_err(|e| {
                    Error::with_detail(
                        ErrorKind::Other,
                        format!("Failed to create a Vulkan device.: {:?}", e),
                    )
                })?
        };

        let swapchain_loader = ext::khr::Swapchain::new(instance, &*vk_device);

        let gfx_device: Box<gfx::Device> = Box::new(unsafe {
            be::device::Device::new(ash::Device::clone(&vk_device), info.info.clone(), config)?
        });

        let main_queue = gfx_device
            .build_cmd_queue()
            .queue_family(info.main_queue_family)
            .build()?;
        let main_queue = Arc::from(main_queue);

        let copy_queue = if let Some(qf) = info.copy_queue_family {
            Some(gfx_device.build_cmd_queue().queue_family(qf).build()?)
        } else {
            None
        };

        let presentation_queue: Arc<gfx::CmdQueue> =
            if presentation_queue_family == info.main_queue_family {
                Arc::clone(&main_queue)
            } else {
                gfx_device
                    .build_cmd_queue()
                    .queue_family(presentation_queue_family)
                    .build()?
                    .into()
            };

        let wm_device = WmDevice {
            device: gfx_device.into(),
            main_queue: GfxQueue {
                queue: main_queue,
                queue_family: info.main_queue_family,
            },
            copy_queue: copy_queue.map(|q| GfxQueue {
                queue: q.into(),
                queue_family: info.copy_queue_family.unwrap(),
            }),
        };

        let swapchain_manager = SwapchainManager::new(
            &wm_device.device,
            swapchain_loader.clone(),
            events_loop_proxy,
        );

        let device_data = painter.add_device(&wm_device);

        Ok(Self {
            info: Arc::clone(info),

            vk_device,
            swapchain_loader,

            swapchain_manager: ManuallyDrop::new(swapchain_manager),
            surfaces: HashMap::new(),

            wm_device: ManuallyDrop::new(wm_device),

            presentation_queue: ManuallyDrop::new(presentation_queue),
            presentation_queue_family,

            device_data: Some(device_data),
        })
    }

    /// Return whether `self` is compatible with the surface.
    fn is_compatible_with_surface(
        &self,
        surface_loader: &ext::khr::Surface,
        vk_surface: vk::SurfaceKHR,
    ) -> bool {
        unsafe {
            surface_loader.get_physical_device_surface_support(
                self.info.vk_phys_device,
                self.presentation_queue_family,
                vk_surface,
            )
        }
    }

    fn finalize(&mut self, painter: &mut P, surface_loader: &ext::khr::Surface) {
        // Drop all surfaces
        while let Some((&surface_ref, _)) = { self.surfaces.iter().next() } {
            self.remove_surface(surface_ref, surface_loader, painter);
        }

        // Unregister the device
        painter.remove_device(&self.wm_device, self.device_data.take().unwrap());
    }

    fn add_surface<S>(
        &mut self,
        window: Window,
        options: &WindowOptions,
        surface_ref: SurfaceRef,
        surface_param: P::SurfaceParam,
        vk_surface: S,
        surface_loader: &ext::khr::Surface,
        painter: &mut P,
    ) where
        S: AutoPtr<vk::SurfaceKHR>,
    {
        let vk_props = optimal_props(
            &window,
            options,
            *vk_surface,
            None,
            self.info.vk_phys_device,
            surface_loader,
        )
        .expect("Failed to compute the optimal surface properties.");

        let vk_create_info = vk_props.to_create_info(*vk_surface, vk::SwapchainKHR::null());

        self.surfaces.reserve(1);

        let surface_data = painter.add_surface(
            &self.wm_device,
            self.device_data.as_mut().unwrap(),
            &surface_ref,
            surface_param,
            &vk_props.to_wsi_surface_props(),
        );

        // Create a swapchain now if possible
        let swapchain;

        if let Some(vk_create_info) = vk_create_info {
            // Hopefully we get a graceful error handling someday...
            let vk_swapchain = unsafe {
                self.swapchain_loader
                    .create_swapchain(&vk_create_info, None)
            }
            .unwrap();
            let vk_swapchain = UniqueSwapchainKHR(&self.swapchain_loader, vk_swapchain);

            self.swapchain_manager
                .add_swapchain(surface_ref, *vk_swapchain)
                .expect("Failed to setup a swapchain.");

            let import_image = vk_props.to_import_image();

            let main_queue: &BeCmdQueue = self.wm_device.main_queue.queue.query_ref().unwrap();

            swapchain = Some(
                Swapchain::new(
                    *vk_swapchain,
                    &self.swapchain_loader,
                    &import_image,
                    main_queue,
                )
                .expect("Failed to acquire images from a swapchain."),
            );

            vk_swapchain.into_inner(); // Release
        } else {
            swapchain = None;
        }

        self.surfaces.insert(
            surface_ref,
            Surface {
                vk_surface: vk_surface.into_inner(),
                window,
                window_options: options.clone(),
                swapchain,
                surface_data,
                vk_props,
                last_error: None,
            },
        );
    }

    fn remove_surface(
        &mut self,
        surface_ref: SurfaceRef,
        surface_loader: &ext::khr::Surface,
        painter: &mut P,
    ) {
        let surface = self.surfaces.remove(&surface_ref).unwrap();

        painter.remove_surface(
            &self.wm_device,
            self.device_data.as_mut().unwrap(),
            &surface_ref,
            surface.surface_data,
        );

        let _vk_surface = UniqueSurfaceKHR(surface_loader, surface.vk_surface);
        if let Some(swapchain) = surface.swapchain {
            let _vk_swapchain = UniqueSwapchainKHR(&self.swapchain_loader, swapchain.vk_swapchain);
            if let Some(ref cb_state_tracker) = swapchain.cb_state_tracker {
                cb_state_tracker.wait();
            }
            self.swapchain_manager.remove_swapchain(surface_ref);
        }
    }

    fn get_winit_window(&self, surface_ref: SurfaceRef) -> Option<&Window> {
        self.surfaces.get(&surface_ref).map(|x| &x.window)
    }

    fn update(
        &mut self,
        update_param: &P::UpdateParam,
        surface_loader: &ext::khr::Surface,
        painter: &mut P,
    ) {
        // Check the properties of swapchains and renew them if they are out-dated
        for (&surface_ref, surface) in self.surfaces.iter_mut() {
            // Always recreate a swapchain if we get these errors last time we
            // update the image
            let out_dated = match surface.last_error {
                Some(PresentError::OutOfDate) | Some(PresentError::Suboptimal) => true,
                _ => false,
            };
            surface.last_error = None;

            let new_props = surface.optimal_props(
                if out_dated {
                    None
                } else {
                    Some(&surface.vk_props)
                },
                self.info.vk_phys_device,
                surface_loader,
            );
            if new_props.is_err() {
                // TODO: Handle surface errors
                // e.g., AMD driver seems to return ErrorInitializationFailed after the window is closed
                if let Some(old_swapchain) = surface.swapchain.take() {
                    self.swapchain_manager.remove_swapchain(surface_ref);
                    unsafe {
                        self.swapchain_loader
                            .destroy_swapchain(old_swapchain.vk_swapchain, None);
                    }
                }

                continue;
            }
            let new_props = new_props.unwrap();

            if out_dated || new_props != surface.vk_props || surface.swapchain.is_none() {
                // Recreate the swapchain
                let base = surface
                    .swapchain
                    .as_ref()
                    .map(|x| x.vk_swapchain)
                    .unwrap_or(vk::SwapchainKHR::null());
                let vk_create_info = new_props.to_create_info(surface.vk_surface, base);

                let swapchain;
                if let Some(vk_create_info) = vk_create_info {
                    let vk_swapchain = match unsafe {
                        self.swapchain_loader
                            .create_swapchain(&vk_create_info, None)
                    } {
                        Ok(x) => x,
                        Err(x) => {
                            // Hopefully we get a graceful error handling someday...
                            panic!("Failed to create a swapchain.: {:?}", x);
                        }
                    };
                    let vk_swapchain = UniqueSwapchainKHR(&self.swapchain_loader, vk_swapchain);

                    self.swapchain_manager.remove_swapchain(surface_ref);
                    self.swapchain_manager
                        .add_swapchain(surface_ref, *vk_swapchain)
                        .expect("Failed to setup a swapchain.");

                    let import_image = new_props.to_import_image();

                    let main_queue: &BeCmdQueue =
                        self.wm_device.main_queue.queue.query_ref().unwrap();

                    swapchain = Some(
                        Swapchain::new(
                            *vk_swapchain,
                            &self.swapchain_loader,
                            &import_image,
                            main_queue,
                        )
                        .expect("Failed to acquire images from a swapchain."),
                    );
                    surface.vk_props = new_props.clone();
                    vk_swapchain.into_inner(); // Release
                } else {
                    swapchain = None;
                }

                use std::mem::replace;
                let old_swapchain = replace(&mut surface.swapchain, swapchain);
                if let Some(ref old_swapchain) = old_swapchain {
                    unsafe {
                        self.swapchain_loader
                            .destroy_swapchain(old_swapchain.vk_swapchain, None);
                    }
                }

                // Notify the change to the upstream
                painter.update_surface(
                    &self.wm_device,
                    self.device_data.as_mut().unwrap(),
                    &surface_ref,
                    &mut surface.surface_data,
                    &new_props.to_wsi_surface_props(),
                );
            }
        }

        // Update swapchains
        let ref mut surfaces = self.surfaces;
        let ref wm_device = self.wm_device;
        let device_data = self.device_data.as_mut().unwrap();
        let ref presentation_queue = &*self.presentation_queue;
        let presentation_queue_family = self.presentation_queue_family;
        let ref swapchain_loader = self.swapchain_loader;

        self.swapchain_manager
            .update(|present_info| match present_info {
                PresentInfo::Present {
                    surface: surface_ref,
                    image_index,
                    wait_semaphore,
                } => {
                    let surface = surfaces.get_mut(&surface_ref).unwrap();
                    let swapchain = surface.swapchain.as_mut().unwrap();

                    let surface_props = surface.vk_props.to_wsi_surface_props();
                    let result = swapchain.update(
                        image_index,
                        surface.vk_props.pixel_ratio,
                        painter,
                        &wait_semaphore,
                        &surface_props,
                        wm_device,
                        swapchain_loader,
                        presentation_queue,
                        presentation_queue_family,
                        device_data,
                        &surface_ref,
                        &mut surface.surface_data,
                        update_param,
                    );

                    match result {
                        Ok(false) => {}
                        Ok(true) => {
                            surface.last_error = Some(PresentError::Suboptimal);
                        }
                        Err(SwapchainUpdateError::PresentError(e)) => {
                            surface.last_error = Some(e);
                        }
                        Err(SwapchainUpdateError::Other(e)) => return Err(e),
                    }

                    Ok(())
                }
                PresentInfo::Fail {
                    surface: surface_ref,
                    error,
                } => {
                    surfaces.get_mut(&surface_ref).unwrap().last_error = Some(error);
                    Ok(())
                }
            })
            .expect("Failed to update some swapchains.");
    }
}

struct Surface<P: Painter> {
    vk_surface: vk::SurfaceKHR,
    window: Window,
    window_options: WindowOptions,
    swapchain: Option<Swapchain>,
    surface_data: P::SurfaceData,
    vk_props: VkSurfaceProps,
    last_error: Option<PresentError>,
}

impl<P: Painter> crate::Debug for Surface<P>
where
    P::SurfaceData: crate::Debug,
{
    fn fmt(&self, fmt: &mut crate::fmt::Formatter) -> crate::fmt::Result {
        fmt.debug_struct("Surface")
            .field("vk_surface", &self.vk_surface)
            .field("window", &())
            .field("window_options", &self.window_options)
            .field("swapchain", &self.swapchain)
            .field("surface_data", &self.surface_data)
            .field("vk_props", &self.vk_props)
            .field("last_error", &self.last_error)
            .finish()
    }
}

#[derive(Debug)]
struct Swapchain {
    vk_swapchain: vk::SwapchainKHR,
    images: Vec<be::image::Image>,
    cb_state_tracker: Option<CbStateTracker>,
}

impl<P: Painter> Surface<P> {
    fn optimal_props(
        &self,
        base: Option<&VkSurfaceProps>,
        vk_phys_device: vk::PhysicalDevice,
        surface_loader: &ext::khr::Surface,
    ) -> Result<VkSurfaceProps, SurfaceError> {
        optimal_props(
            &self.window,
            &self.window_options,
            self.vk_surface,
            base,
            vk_phys_device,
            surface_loader,
        )
    }
}

impl Swapchain {
    fn new(
        vk_swapchain: vk::SwapchainKHR,
        swapchain_loader: &ext::khr::Swapchain,
        import_image: &be::image::ImportImage,
        queue: &BeCmdQueue,
    ) -> Result<Self, SurfaceError> {
        let vk_images = unsafe { swapchain_loader.get_swapchain_images(vk_swapchain) }
            .map_err(SurfaceError::from)?;

        let images = vk_images
            .iter()
            .map(|&vk_image| unsafe {
                be::image::ImportImage {
                    vk_image,
                    ..import_image.clone()
                }
                .build(queue)
            })
            .collect::<GfxResult<_>>()?;

        Ok(Self {
            vk_swapchain,
            images,
            cb_state_tracker: None,
        })
    }

    /// Submit device commands that generate and present the new contents of
    /// the swapchain.
    ///
    /// Returns `Ok(false)` if it was successful and `Ok(true)` if it was
    /// successful but the swapchain is no longer "optimal".
    fn update<P: Painter>(
        &mut self,
        image_index: usize,
        pixel_ratio: f32,
        painter: &mut P,
        be_semaphore: &BeSemaphore,
        surface_props: &SurfaceProps,
        device: &WmDevice,
        swapchain_loader: &ext::khr::Swapchain,
        presentation_queue: &Arc<gfx::CmdQueue>,
        presentation_queue_family: gfx::QueueFamily,
        device_data: &mut P::DeviceData,
        surface_ref: &SurfaceRef,
        surface_data: &mut P::SurfaceData,
        update_param: &P::UpdateParam,
    ) -> Result<bool, SwapchainUpdateError> {
        struct Drawable<'a> {
            device: &'a WmDevice,
            swapchain_loader: &'a ext::khr::Swapchain,
            vk_swapchain: vk::SwapchainKHR,
            image: gfx::ImageRef,
            image_index: u32,
            pixel_ratio: f32,
            surface_props: &'a SurfaceProps,
            be_semaphore: &'a BeSemaphore,
            presentation_queue: &'a Arc<gfx::CmdQueue>,
            presentation_queue_family: gfx::QueueFamily,
            needs_ownership_transfer: Option<gfx::QueueFamily>,
            queue_present_result: Option<Result<bool, SwapchainUpdateError>>,
            cb_state_tracker: &'a mut Option<CbStateTracker>,
        }

        impl<'a> super::Drawable for Drawable<'a> {
            fn image(&self) -> &gfx::ImageRef {
                &self.image
            }

            fn surface_props(&self) -> &SurfaceProps {
                self.surface_props
            }

            fn pixel_ratio(&self) -> f32 {
                self.pixel_ratio
            }

            fn encode_prepare_present(
                &mut self,
                cmd_buffer: &mut gfx::CmdBufferRef,
                queue_family: gfx::QueueFamily,
                stage: gfx::StageFlags,
                access: gfx::AccessTypeFlags,
            ) {
                let gfx_semaphore: gfx::SemaphoreRef = self.be_semaphore.clone().into();
                cmd_buffer.wait_semaphore(&gfx_semaphore, stage);

                // Perform image layout transition (the "present" image layout is
                // out of the scope of ZanGFX)
                {
                    let cmd_buffer: &mut BeCmdBuffer = cmd_buffer.query_mut().unwrap();
                    let image: &be::image::Image = self.image.downcast_ref().unwrap();

                    assert_eq!(access, gfx::AccessTypeFlags::COLOR_WRITE);
                    assert_eq!(stage, gfx::StageFlags::RENDER_OUTPUT);

                    let mut barrier = vk::ImageMemoryBarrier {
                        s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
                        p_next: crate::null(),
                        src_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                        dst_access_mask: vk::AccessFlags::empty(),
                        old_layout: image.translate_layout(gfx::ImageLayout::Render),
                        new_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                        src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        image: image.vk_image(),
                        subresource_range: vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            base_array_layer: 0,
                            level_count: vk::REMAINING_MIP_LEVELS,
                            layer_count: vk::REMAINING_ARRAY_LAYERS,
                        },
                    };

                    if queue_family != self.presentation_queue_family {
                        // Perform the releasing part of queue ownership transfer operation if needed
                        barrier.src_queue_family_index = queue_family;
                        barrier.dst_queue_family_index = self.presentation_queue_family;
                    }

                    let vk_cmd_buffer = cmd_buffer.vk_cmd_buffer().unwrap();
                    let be_device: &be::device::Device = self.device.device.query_ref().unwrap();

                    unsafe {
                        be_device.vk_device().cmd_pipeline_barrier(
                            vk_cmd_buffer,
                            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                            vk::DependencyFlags::empty(),
                            &[],
                            &[],
                            &[barrier],
                        );
                    }
                }

                cmd_buffer.signal_semaphore(&gfx_semaphore, stage);
            }

            fn enqueue_present(&mut self) {
                let gfx_semaphore: gfx::SemaphoreRef = self.be_semaphore.clone().into();

                // Perform the acquiring part of queue ownership transfer operation if needed
                if let Some(src_queue_family) = self.needs_ownership_transfer {
                    let mut cmd_buffer = self
                        .presentation_queue
                        .new_cmd_buffer()
                        .expect("Failed to create a command buffer.");
                    cmd_buffer.wait_semaphore(&gfx_semaphore, flags![gfx::StageFlags::{}]);

                    {
                        let cmd_buffer: &mut BeCmdBuffer = cmd_buffer.query_mut().unwrap();
                        let image: &be::image::Image = self.image.downcast_ref().unwrap();

                        let barrier = vk::ImageMemoryBarrier {
                            s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
                            p_next: crate::null(),
                            src_access_mask: vk::AccessFlags::empty(),
                            dst_access_mask: vk::AccessFlags::empty(),
                            old_layout: image.translate_layout(gfx::ImageLayout::Render),
                            new_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                            src_queue_family_index: src_queue_family,
                            dst_queue_family_index: self.presentation_queue_family,
                            image: image.vk_image(),
                            subresource_range: vk::ImageSubresourceRange {
                                aspect_mask: vk::ImageAspectFlags::COLOR,
                                base_mip_level: 0,
                                base_array_layer: 0,
                                level_count: vk::REMAINING_MIP_LEVELS,
                                layer_count: vk::REMAINING_ARRAY_LAYERS,
                            },
                        };

                        let vk_cmd_buffer = cmd_buffer.vk_cmd_buffer().unwrap();
                        let be_device: &be::device::Device =
                            self.device.device.query_ref().unwrap();

                        unsafe {
                            be_device.vk_device().cmd_pipeline_barrier(
                                vk_cmd_buffer,
                                vk::PipelineStageFlags::TOP_OF_PIPE,
                                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                                vk::DependencyFlags::empty(),
                                &[],
                                &[],
                                &[barrier],
                            );
                        }
                    }

                    if let Some(cb_state_tracker) = self.cb_state_tracker.take() {
                        cb_state_tracker.wait();
                    }
                    *self.cb_state_tracker = Some(CbStateTracker::new(&mut *cmd_buffer));

                    cmd_buffer.signal_semaphore(&gfx_semaphore, flags![gfx::StageFlags::{}]);
                    cmd_buffer
                        .commit()
                        .expect("Failed to commit a command buffer.");
                    self.presentation_queue.flush();
                }

                // Enqueue the present request
                let be_presentation_queue: &BeCmdQueue =
                    self.presentation_queue.query_ref().unwrap();
                let vk_semaphore = self.be_semaphore.vk_semaphore();

                let present_info = vk::PresentInfoKHR {
                    s_type: vk::StructureType::PRESENT_INFO_KHR,
                    p_next: crate::null(),
                    wait_semaphore_count: 1,
                    p_wait_semaphores: &vk_semaphore,
                    swapchain_count: 1,
                    p_swapchains: &self.vk_swapchain,
                    p_image_indices: &self.image_index,
                    p_results: crate::null_mut(),
                };

                let result = unsafe {
                    self.swapchain_loader
                        .queue_present(be_presentation_queue.vk_queue(), &present_info)
                };

                self.queue_present_result = Some(result.map_err(Into::into));
            }
        }

        let mut drawable = Drawable {
            device,
            swapchain_loader,
            image: self.images[image_index].clone().into(),
            image_index: image_index as u32,
            pixel_ratio,
            vk_swapchain: self.vk_swapchain,
            surface_props,
            be_semaphore,
            presentation_queue,
            presentation_queue_family,
            needs_ownership_transfer: None,
            queue_present_result: None,
            cb_state_tracker: &mut self.cb_state_tracker,
        };

        painter.paint(
            device,
            device_data,
            surface_ref,
            surface_data,
            update_param,
            &mut drawable,
        );

        // Return the result of the present command (whether it's an error or not)
        drawable
            .queue_present_result
            .expect("enqueue_present was not called")
    }
}

/// Compute optimal surface properties for a window and its surface.
///
/// If `base` is specified, only `extents` and some minimal number of fields
/// are updated with fresh values.
fn optimal_props(
    window: &Window,
    options: &WindowOptions,
    vk_surface: vk::SurfaceKHR,
    base: Option<&VkSurfaceProps>,
    vk_phys_device: vk::PhysicalDevice,
    surface_loader: &ext::khr::Surface,
) -> Result<VkSurfaceProps, SurfaceError> {
    let surface_caps = unsafe {
        surface_loader.get_physical_device_surface_capabilities(vk_phys_device, vk_surface)
    }
    .map_err(SurfaceError::from)?;

    let window_extents = window.get_inner_size().unwrap(); // we're sure the window exists
    let pixel_ratio = window.get_hidpi_factor();
    let phys_extents = window_extents.to_physical(pixel_ratio);
    let extents = match surface_caps.current_extent.width {
        x if x == <u32>::max_value() => [phys_extents.width as u32, phys_extents.height as u32],
        _ => [
            surface_caps.current_extent.width,
            surface_caps.current_extent.height,
        ],
    };

    use std::cmp::{max, min};
    let image_count = max(
        min(2, surface_caps.max_image_count),
        surface_caps.min_image_count,
    );

    let pre_transform = surface_caps.current_transform;

    let composite_alpha_candidates = if options.transparent {
        &[
            vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED,
            vk::CompositeAlphaFlagsKHR::INHERIT,
            vk::CompositeAlphaFlagsKHR::OPAQUE,
        ]
    } else {
        &[
            vk::CompositeAlphaFlagsKHR::OPAQUE,
            vk::CompositeAlphaFlagsKHR::INHERIT,
            vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED,
        ]
    };
    let composite_alpha = composite_alpha_candidates
        .iter()
        .cloned()
        .find(|&x| surface_caps.supported_composite_alpha.intersects(x))
        .expect("Failed to find a compatible composite alpha mode.");

    // Take the fast path if we're asked to do so
    if let Some(base) = base {
        return Ok(VkSurfaceProps {
            extents,
            min_image_count: image_count,
            pre_transform,
            composite_alpha,
            ..base.clone()
        });
    }

    // Perform a full computation
    let present_mode = vk::PresentModeKHR::FIFO;

    let surface_formats =
        unsafe { surface_loader.get_physical_device_surface_formats(vk_phys_device, vk_surface) }
            .map_err(SurfaceError::from)?;

    // Choose the format we like
    let surface_format = choose_surface_format(
        surface_formats.iter().cloned(),
        &[
            (
                Some(gfx::ImageFormat::SrgbBgra8),
                Some(vk::ColorSpaceKHR::SRGB_NONLINEAR),
            ),
            (
                Some(gfx::ImageFormat::SrgbRgba8),
                Some(vk::ColorSpaceKHR::SRGB_NONLINEAR),
            ),
            (
                Some(<u8>::as_rgba_norm()),
                Some(vk::ColorSpaceKHR::SRGB_NONLINEAR),
            ),
            (Some(gfx::ImageFormat::SrgbBgra8), None),
            (Some(gfx::ImageFormat::SrgbRgba8), None),
            (Some(<u8>::as_rgba_norm()), None),
            (None, None),
        ],
    );
    let (format, color_space) =
        surface_format.expect("Failed to find a compatible surface format.");

    Ok(VkSurfaceProps {
        extents,
        min_image_count: image_count,
        pre_transform,
        composite_alpha,
        present_mode,
        format,
        color_space,
        pixel_ratio: pixel_ratio as f32,
    })
}

fn choose_surface_format<I>(
    formats: I,
    preferences: &[(Option<gfx::ImageFormat>, Option<vk::ColorSpaceKHR>)],
) -> Option<(gfx::ImageFormat, vk::ColorSpaceKHR)>
where
    I: Clone + Iterator<Item = vk::SurfaceFormatKHR>,
{
    // For each search criteria...
    preferences
        .iter()
        .filter_map(|&(format, color_space)| {
            // The filtered set of formats supported by ZanGFX
            let mut gfx_supported_formats = formats.clone().filter_map(|x| {
                be::formats::reverse_translate_image_format(x.format)
                    .map(|gfx_format| (gfx_format, x.color_space))
            });

            // Return the first one that matches the search criteria
            gfx_supported_formats
                .find(|x| x.0 == format.unwrap_or(x.0) && x.1 == color_space.unwrap_or(x.1))
        })
        .nth(0)
}

#[derive(Debug, Clone, PartialEq)]
struct VkSurfaceProps {
    extents: [u32; 2],
    format: gfx::ImageFormat,
    color_space: vk::ColorSpaceKHR,
    min_image_count: u32,
    pre_transform: vk::SurfaceTransformFlagsKHR,
    composite_alpha: vk::CompositeAlphaFlagsKHR,
    present_mode: vk::PresentModeKHR,
    pixel_ratio: f32,
}

impl VkSurfaceProps {
    fn to_wsi_surface_props(&self) -> SurfaceProps {
        SurfaceProps {
            extents: self.extents,
            format: self.format,
        }
    }

    fn to_import_image(&self) -> be::image::ImportImage {
        be::image::ImportImage {
            vk_image: vk::Image::null(),
            format: be::formats::translate_image_format(self.format).unwrap(),
            view_type: vk::ImageViewType::TYPE_2D,
            num_mip_levels: 1,
            num_layers: 1,
            usage: flags![gfx::ImageUsageFlags::{}],
            aspects: vk::ImageAspectFlags::COLOR,
            destroy_manually: true,
        }
    }

    /// Construct a `SwapchainCreateInfoKHR`. Returns `None` if a swapchain
    /// cannot be created from these properties.
    fn to_create_info(
        &self,
        surface: vk::SurfaceKHR,
        old_swapchain: vk::SwapchainKHR,
    ) -> Option<vk::SwapchainCreateInfoKHR> {
        if self.extents[0] == 0 || self.extents[1] == 0 {
            return None;
        }
        Some(vk::SwapchainCreateInfoKHR {
            s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
            p_next: crate::null(),
            flags: vk::SwapchainCreateFlagsKHR::empty(),
            surface,
            min_image_count: self.min_image_count,
            image_format: be::formats::translate_image_format(self.format).unwrap(),
            image_color_space: self.color_space,
            image_extent: vk::Extent2D {
                width: self.extents[0],
                height: self.extents[1],
            },
            image_array_layers: 1,
            image_usage: vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: crate::null(),
            pre_transform: self.pre_transform,
            composite_alpha: self.composite_alpha,
            present_mode: self.present_mode,
            clipped: vk::FALSE,
            old_swapchain,
        })
    }
}

#[derive(Debug)]
enum SwapchainUpdateError {
    PresentError(PresentError),
    Other(Error),
}

impl From<vk::Result> for SwapchainUpdateError {
    fn from(x: vk::Result) -> Self {
        match x {
            x if x == vk::Result::ERROR_OUT_OF_DATE_KHR => {
                SwapchainUpdateError::PresentError(PresentError::OutOfDate)
            }
            x if x == vk::Result::ERROR_SURFACE_LOST_KHR => {
                SwapchainUpdateError::PresentError(PresentError::SurfaceLost)
            }
            x => SwapchainUpdateError::Other(utils::translate_generic_error_unwrap(x)),
        }
    }
}

#[derive(Debug)]
enum SurfaceError {
    SurfaceLost,
    Other(Error),
}

impl From<vk::Result> for SurfaceError {
    fn from(x: vk::Result) -> Self {
        // Certain drivers return `InitializationFailed` when a surface is lost
        if x == vk::Result::ERROR_SURFACE_LOST_KHR || x == vk::Result::ERROR_INITIALIZATION_FAILED {
            SurfaceError::SurfaceLost
        } else {
            SurfaceError::Other(utils::translate_generic_error_unwrap(x))
        }
    }
}

impl From<Error> for SurfaceError {
    fn from(x: Error) -> Self {
        SurfaceError::Other(x)
    }
}

#[derive(Debug)]
struct PhysicalDeviceInfo {
    vk_phys_device: vk::PhysicalDevice,
    info: be::limits::DeviceInfo,
    enabled_features: vk::PhysicalDeviceFeatures,
    main_queue_family: gfx::QueueFamily,
    copy_queue_family: Option<gfx::QueueFamily>,
}

impl PhysicalDeviceInfo {
    /// Examine the properties of the given physical device. Returns `Self`
    /// if the device is compatible with NgsPF.
    fn new(
        instance: &ash::Instance,
        vk_phys_device: vk::PhysicalDevice,
    ) -> GfxResult<Option<Self>> {
        let available_features = unsafe { instance.get_physical_device_features(vk_phys_device) };

        let enabled_features = vk::PhysicalDeviceFeatures {
            robust_buffer_access: if cfg!(debug_assertions) {
                // Enable robust buffer access only in the debug build since it
                // may incur significant performance penalties
                available_features.robust_buffer_access
            } else {
                vk::FALSE
            },
            ..Default::default()
        };

        let info = be::limits::DeviceInfo::from_physical_device(
            instance,
            vk_phys_device,
            &enabled_features,
        )?;

        let main_queue_family;
        let copy_queue_family;
        {
            let choose = |f: &Fn(_) -> bool| {
                info.queue_families
                    .iter()
                    .enumerate()
                    .find(|&(_, info)| f(info.caps))
                    .map(|x| x.0 as gfx::QueueFamily)
            };

            // Choose the main queue. (Mandatory)
            let result = choose(&|caps| {
                caps.contains(flags![gfx::QueueFamilyCapsFlags::{RENDER | COMPUTE | COPY}])
            });
            main_queue_family = if let Some(x) = result {
                x
            } else {
                return Ok(None);
            };

            // Choose the copy queue. Popular discrete GPUs have one or more
            // DMA engines dedicated for copy operations.
            copy_queue_family = choose(&|caps| caps == gfx::QueueFamilyCapsFlags::COPY);
        }

        Ok(Some(Self {
            vk_phys_device,
            info,
            enabled_features,
            main_queue_family,
            copy_queue_family,
        }))
    }

    /// Return the queue family compatible with the surface.
    fn queue_family_compatible_with_surface(
        &self,
        surface_loader: &ext::khr::Surface,
        vk_surface: vk::SurfaceKHR,
    ) -> Option<gfx::QueueFamily> {
        for i in 0..self.info.queue_families.len() {
            if unsafe {
                surface_loader.get_physical_device_surface_support(
                    self.vk_phys_device,
                    i as _,
                    vk_surface,
                )
            } {
                return Some(i as _);
            }
        }
        return None;
    }
}
