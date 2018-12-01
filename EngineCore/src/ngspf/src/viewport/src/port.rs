//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use refeq::RefEqArc;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use zangfx::base as gfx;

pub use crate::wsi::GfxQueue;
use ngspf_core::PresenterFrame;

/// ZanGFX objects passed to ports.
///
/// TODO: Merge with `wsi::GfxObjects`
#[derive(Debug, Clone)]
pub struct GfxObjects {
    pub device: Arc<gfx::Device>,
    pub main_queue: GfxQueue,
    pub copy_queue: Option<GfxQueue>,
}

/// Trait for creating `PortInstance` for a specific NgsGFX device.
pub trait Port: fmt::Debug + Send + Sync + 'static {
    /// Create a port instance for a specific NgsGFX device.
    fn mount(&self, objects: &GfxObjects) -> Box<PortInstance>;
}

/// The properties of a backing store image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PortImageProps {
    /// The extents of the image.
    pub extents: [u32; 2],
    /// The image format. Currently it is always `ImageFormat::SrgbRgba8`.
    pub format: gfx::ImageFormat,
}

#[derive(Debug)]
pub struct PortRenderContext {
    /// The image to render on.
    ///
    /// The image must be transitioned to the `ShaderRead` layout after
    /// rendering. Its ownership must be transfered to `main_queue`.
    ///
    /// The image must not be accessed by GPU after `fence` was updated, nor by
    /// CPU after the command buffer where `fence` is updated was committed.
    /// (Internally, this image is allocated from `TempResPool` and is retained
    /// by `TempResFrame`, which is recycled when the device completes the
    /// execution of a command buffer.)
    pub image: gfx::ImageRef,
    pub image_props: PortImageProps,
    /// The fence to be updated after rendering.
    pub fence: gfx::FenceRef,
    /// Set this to `true` to continuously update the screen.
    pub schedule_next_frame: bool,
}

/// Trait for rendering custom contents as layer contents.
pub trait PortInstance: fmt::Debug + Send + Sync + 'static {
    /// The usage of the backing store image (`PortRenderContext::image`).
    fn image_usage(&self) -> gfx::ImageUsageFlags {
        gfx::ImageUsageFlags::Render
    }

    /// The format of the backing store image (`PortRenderContext::image`).
    fn image_format(&self) -> gfx::ImageFormat {
        gfx::ImageFormat::SrgbRgba8
    }

    /// The size of the backing store image (`PortRenderContext::image`).
    fn image_extents(&self) -> [u32; 2];

    fn render(
        &mut self,
        context: &mut PortRenderContext,
        frame: &PresenterFrame,
    ) -> gfx::Result<()>;
}

/// Maintains port instances associated with `Port`s.
#[derive(Debug)]
pub(super) struct PortManager {
    /// Set of mounted port instances.
    port_map: HashMap<RefEqArc<Port>, PortMapping>,
}

#[derive(Debug)]
struct PortMapping {
    instance: Arc<Mutex<Box<PortInstance>>>,
    used_in_last_frame: bool,
}

impl PortManager {
    pub fn new() -> Self {
        Self {
            port_map: HashMap::new(),
        }
    }

    /// Destroy out-dated port instances (that is, whose nodes are no longer
    /// on the layer tree).
    pub fn purge(&mut self) {
        use std::mem::replace;
        self.port_map
            .retain(|_, map| replace(&mut map.used_in_last_frame, false));
    }

    pub fn get(
        &mut self,
        port: &RefEqArc<Port>,
        gfx_objects: &GfxObjects,
    ) -> &Arc<Mutex<Box<PortInstance>>> {
        let ent = self.port_map.entry(RefEqArc::clone(port));
        let map = ent.or_insert_with(|| {
            // The port instance has not yet been created for the `Port`.
            // Mount the port and create the port instance.
            let instance = port.mount(gfx_objects);

            // Save the created instance and return a reference to it
            PortMapping {
                instance: Arc::new(Mutex::new(instance)),
                used_in_last_frame: true,
            }
        });
        map.used_in_last_frame = true;
        &map.instance
    }
}
