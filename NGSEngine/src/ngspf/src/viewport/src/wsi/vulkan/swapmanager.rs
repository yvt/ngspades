//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::Arc;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError};
use std::thread;
use std::mem::ManuallyDrop;
use std::collections::HashMap;
use winit::EventsLoopProxy;

use super::ash::{extensions as ext, vk, version::*};
use super::atomic_refcell::AtomicRefCell;

use zangfx::{base as gfx, common::Result as GfxResult, prelude::*};
use super::be::cmd::semaphore::Semaphore as BeSemaphore;
use super::smartptr::{AutoPtr, UniqueFence};
use super::utils::{translate_generic_error_unwrap, vk_device_from_gfx};

pub type SurfaceId = super::SurfaceRef;

/// Maintains a multiple `VkSwapchainKHR`s and wakes up the main event loop
/// whenever some of them are ready to accept new images.
///
///  - Given a swapchain, it first calls `vkAcquireNextImageKHR` to retrieve the
///    first image used for presentation. The retrieved image and semaphore is
///    sent to the application immediately.
///
///  - The fence passed to `vkAcquireNextImageKHR` is used to meter the
///    subsequent calls to `vkAcquireNextImageKHR`. It'll wait on the fence, and
///    when it's signaled, a call to `vkAcquireNextImageKHR` is made with
///    `timeout` set to zero.
///
///    Basically, we assume that the next but one image is immediately ready
///    to be returned from `vkAcquireNextImageKHR` as soon as the next image is
///    ready to be written (i.e. the previous presentation operation of the
///    image is done).
///
///    The function might return `VK_NOT_READY`. In such cases, we don't know
///    exactly when it'll return the next image, but we can't delay the
///    processing of swapchains by using a non-zero timeout value. Therefore, we
///    switch to the polling mode for such swapchains.
///
/// `SwapchainManager` spawns a background thread which is used to wait on
/// fences. The intention of using this instead of waiting on them directly
/// from the main thread is to allow multiplexing multiple event sources
/// including other `SwapchainManager`s and the input events sent by the window
/// system.
pub(super) struct SwapchainManager {
    device: Arc<gfx::Device>,
    ext_swapchain: ext::Swapchain,

    /// `SyncSender` used to request the background thread to wait on the fences.
    fences_send: ManuallyDrop<AtomicRefCell<SyncSender<FenceSet>>>,
    /// `Receiver` via which `Vec<Fence>` is returned from the background thread.
    /// (This facilitates the reuse of `Vec`s)
    fences_recv: ManuallyDrop<AtomicRefCell<Receiver<(FenceSet, Result<(), vk::Result>)>>>,
    /// `None` if the background thread is waiting on some fences.
    empty_fence_set: Option<FenceSet>,

    retired_fences: Vec<vk::Fence>,

    join_handle: ManuallyDrop<thread::JoinHandle<()>>,

    swapchains: HashMap<SurfaceId, Swapchain>,
}

type FenceSet = Vec<vk::Fence>;

impl ::Debug for SwapchainManager {
    fn fmt(&self, fmt: &mut ::fmt::Formatter) -> ::fmt::Result {
        fmt.debug_struct("SwapchainManager")
            .field("device", &self.device)
            .field("fences_send", &self.fences_send)
            .field("fences_recv", &self.fences_recv)
            .field("empty_fence_set", &self.empty_fence_set)
            .field("retired_fences", &self.retired_fences)
            .field("join_handle", &self.join_handle)
            .field("swapchains", &self.swapchains)
            .finish()
    }
}

#[derive(Debug)]
pub(super) enum PresentInfo {
    Present {
        surface: SurfaceId,
        image_index: usize,
        wait_semaphore: BeSemaphore,
    },
    Fail {
        surface: SurfaceId,
        /// `Suboptimal`, `OutOfDate`, or `SurfaceLost`
        error: PresentError,
    },
}

#[derive(Debug)]
pub enum PresentError {
    Suboptimal,
    OutOfDate,
    SurfaceLost,
}

#[derive(Debug)]
struct Swapchain {
    vk_swapchain: vk::SwapchainKHR,
    vk_fence: vk::Fence,
    /// The semaphore used to wait until the presentation image is available.
    be_semaphore: BeSemaphore,
    /// Indicates whether the swapchain is in the polling mode.
    /// If it is, `vkAcquireNextImageKHR` must be called periodically until a
    /// new image is acquired, or the swapchain is destroyed.
    polling: bool,
}

impl Drop for SwapchainManager {
    fn drop(&mut self) {
        let vk_device = vk_device_from_gfx(&*self.device);

        unsafe {
            ManuallyDrop::drop(&mut self.fences_send);
        }
        use std::ptr::read;
        let join_handle = unsafe { read(&*self.join_handle) };
        join_handle.join().unwrap();
        unsafe {
            ManuallyDrop::drop(&mut self.fences_recv);
        }

        for (_, swapchain) in self.swapchains.drain() {
            unsafe {
                vk_device.destroy_fence(swapchain.vk_fence, None);
            }
        }
    }
}

impl SwapchainManager {
    pub fn new(
        device: &Arc<gfx::Device>,
        ext_swapchain: ext::Swapchain,
        events_loop_proxy: EventsLoopProxy,
    ) -> Self {
        let device = Arc::clone(device);
        let (wait_fence_send, wait_fence_recv) = sync_channel::<FenceSet>(1);
        let (return_fence_send, return_fence_recv) = sync_channel(1);

        let join_handle = {
            let device = device.clone();
            thread::Builder::new()
                .name("NgsPF fence manager".to_string())
                .spawn(move || {
                    let vk_device = vk_device_from_gfx(&*device);
                    for fence_set in wait_fence_recv.iter() {
                        let result = unsafe {
                            vk_device.wait_for_fences(
                                fence_set.as_slice(),
                                false,
                                <u64>::max_value(),
                            )
                        };
                        return_fence_send.send((fence_set, result)).unwrap();

                        // Wake up the main event loop. This'll get
                        // `self.update` called.
                        let _ = events_loop_proxy.wakeup();
                    }
                })
                .unwrap()
        };

        Self {
            device,
            ext_swapchain,
            fences_send: ManuallyDrop::new(AtomicRefCell::new(wait_fence_send)),
            fences_recv: ManuallyDrop::new(AtomicRefCell::new(return_fence_recv)),
            empty_fence_set: Some(FenceSet::new()),
            retired_fences: Vec::new(),
            join_handle: ManuallyDrop::new(join_handle),
            swapchains: HashMap::new(),
        }
    }

    /// Called by the window manager when the event loop is woken up or
    /// something happens.
    pub fn update<F>(&mut self, mut f: F) -> GfxResult<()>
    where
        F: FnMut(PresentInfo) -> GfxResult<()>,
    {
        loop {
            let fence_set;

            if let Some(x) = self.empty_fence_set.take() {
                fence_set = Some(x);
            } else {
                match self.fences_recv.borrow_mut().try_recv() {
                    Ok((got_fence, result)) => {
                        result.map_err(translate_generic_error_unwrap)?;
                        // Some fences were signaled. Now find out which one.
                        fence_set = Some(got_fence);
                    }
                    Err(TryRecvError::Empty) => {
                        fence_set = None;
                    }
                    Err(TryRecvError::Disconnected) => {
                        // We haven't torn down the thread yet; something nasty is going on!
                        unreachable!()
                    }
                }
            }

            // Skip the check if we have some fences being waited on, and none of
            // them are signaled yet.
            if fence_set.is_none() {
                return Ok(());
            }

            let vk_device = vk_device_from_gfx(&*self.device);

            // Destroy all retired fences
            for vk_fence in self.retired_fences.drain(..) {
                unsafe {
                    vk_device.destroy_fence(vk_fence, None);
                }
            }

            // Keep it running in case of an error...
            self.empty_fence_set = Some(FenceSet::new());

            let mut fence_set = fence_set.unwrap();
            fence_set.clear();
            fence_set.reserve(self.swapchains.len());

            let mut active = false;

            for (&surface_id, swapchain) in self.swapchains.iter_mut() {
                if !swapchain.polling {
                    // This swapchain is not in the polling mode. We request the
                    // next image only if the fence is signaled.
                    match unsafe { vk_device.get_fence_status(swapchain.vk_fence) } {
                        Ok(()) => {} // signaled
                        Err(vk::Result::NotReady) => {
                            // unsignaled
                            continue;
                        }
                        Err(x) => return Err(translate_generic_error_unwrap(x)),
                    }
                }

                swapchain.polling = true;

                unsafe { vk_device.reset_fences(&[swapchain.vk_fence]) }
                    .map_err(translate_generic_error_unwrap)?;

                active = true;

                match unsafe {
                    self.ext_swapchain.acquire_next_image_khr(
                        swapchain.vk_swapchain,
                        0,
                        swapchain.be_semaphore.vk_semaphore(),
                        swapchain.vk_fence,
                    )
                } {
                    Ok(image_index) => {
                        swapchain.polling = false;
                        fence_set.push(swapchain.vk_fence);

                        let wait_semaphore = swapchain.be_semaphore.clone().into();
                        if let Err(e) = f(PresentInfo::Present {
                            surface: surface_id,
                            image_index: image_index as _,
                            wait_semaphore,
                        }) {
                            // TODO: Handle update failure gracefully
                            return Err(e);
                        }
                    }
                    Err(e) => {
                        // `Suboptimal` isn't actually an error, but `ash`'s
                        // `acquire_next_image_khr` won't return the image index in
                        // such a case
                        match e {
                            vk::Result::Timeout => {
                                // Enter the polling mode
                            }
                            vk::Result::SuboptimalKhr => {
                                f(PresentInfo::Fail {
                                    surface: surface_id,
                                    error: PresentError::Suboptimal,
                                })?;
                            }
                            vk::Result::ErrorOutOfDateKhr => {
                                f(PresentInfo::Fail {
                                    surface: surface_id,
                                    error: PresentError::OutOfDate,
                                })?;
                            }
                            vk::Result::ErrorSurfaceLostKhr => {
                                f(PresentInfo::Fail {
                                    surface: surface_id,
                                    error: PresentError::SurfaceLost,
                                })?;
                            }
                            _ => return Err(translate_generic_error_unwrap(e)),
                        }
                    }
                }
            }

            if fence_set.len() == 0 {
                self.empty_fence_set = Some(fence_set);
            } else {
                self.empty_fence_set = None;
                self.fences_send.borrow_mut().send(fence_set).unwrap();
            }

            if !active {
                return Ok(());
            }
        }
    }

    pub fn add_swapchain(
        &mut self,
        surface_id: SurfaceId,
        vk_swapchain: vk::SwapchainKHR,
    ) -> GfxResult<()> {
        self.swapchains.reserve(1);

        let vk_device = vk_device_from_gfx(&*self.device);

        let vk_fence = unsafe {
            vk_device.create_fence(
                &vk::FenceCreateInfo {
                    s_type: vk::StructureType::FenceCreateInfo,
                    p_next: ::null(),
                    flags: vk::FenceCreateFlags::empty(),
                },
                None,
            )
        }.map_err(translate_generic_error_unwrap)?;
        let vk_fence = UniqueFence(vk_device, vk_fence);

        let gfx_semaphore = self.device.new_semaphore()?;
        let be_semaphore: &BeSemaphore = gfx_semaphore.downcast_ref().expect("bad semaphore type");

        let swapchain = Swapchain {
            vk_swapchain,
            vk_fence: vk_fence.1,
            be_semaphore: be_semaphore.clone(),
            polling: true,
        };

        self.swapchains.insert(surface_id, swapchain);
        vk_fence.into_inner();
        Ok(())
    }

    pub fn remove_swapchain(&mut self, surface_id: SurfaceId) {
        self.retired_fences.reserve(1);

        let swapchain = self.swapchains.remove(&surface_id).unwrap();

        if !swapchain.polling {
            // The presentation engine might be still accessing the fence.
            let vk_device = vk_device_from_gfx(&*self.device);
            unsafe {
                let _ = vk_device.wait_for_fences(&[swapchain.vk_fence], true, <u64>::max_value());
            }
        }

        // Can't destroy a fence while it's being `vkWaitForFence`-ed.
        self.retired_fences.push(swapchain.vk_fence);
    }
}
