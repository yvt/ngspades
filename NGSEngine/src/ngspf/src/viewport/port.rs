//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::{any, fmt};
use std::sync::Arc;
use std::collections::HashMap;
use atomic_refcell::AtomicRefCell;
use refeq::RefEqArc;
use gfx::core::Backend;

use context::PresenterFrame;
use super::WorkspaceDevice;

/// Trait for creating `PortInstance` for a specific NgsGFX backend.
pub trait Port: fmt::Debug + Send + Sync + 'static {
    /// Create a port instance for a specific NgsGFX backend.
    ///
    /// The callee must find an appropriate implementation for the actual
    /// backend by calling `PortMountContext::downcast_mut` with a known set
    /// of backends.
    ///
    ///     use ngspf::viewport::{Port, PortMountContext};
    ///     use ngspf::gfx::backends::DefaultBackend;
    ///     #[derive(Debug)]
    ///     struct MyPort;
    ///
    ///     impl Port for MyPort {
    ///         fn mount(&self, context: &mut PortMountContext) {
    ///             if let Some(mut context) = context.downcast_mut::<DefaultBackend>() {
    ///                 context.set_instance(panic!("provide instance here"));
    ///             }
    ///         }
    ///     }
    ///
    fn mount(&self, context: &mut PortMountContext);
}

#[derive(Debug)]
pub struct PortMountContext<'a> {
    pub(super) workspace_device: &'a any::Any,
    pub(super) result_instance: &'a mut any::Any,
}

#[derive(Debug)]
pub struct PortMountContextWithBackend<'a, B: Backend> {
    pub(super) workspace_device: &'a WorkspaceDevice<B>,
    pub(super) result_instance: &'a mut Option<Box<PortInstance<B>>>,
}

impl<'a> PortMountContext<'a> {
    pub fn downcast_mut<B: Backend>(&mut self) -> Option<PortMountContextWithBackend<B>> {
        if let Some(result_instance) = self.result_instance.downcast_mut() {
            Some(PortMountContextWithBackend {
                workspace_device: self.workspace_device.downcast_ref().unwrap(),
                result_instance,
            })
        } else {
            None
        }
    }
}

impl<'a, B: Backend> PortMountContextWithBackend<'a, B> {
    pub fn workspace_device(&self) -> &'a WorkspaceDevice<B> {
        self.workspace_device
    }

    pub fn set_instance(&mut self, instance: Box<PortInstance<B>>) {
        *self.result_instance = Some(instance);
    }

    fn upcast_mut(&mut self) -> PortMountContext {
        PortMountContext {
            workspace_device: self.workspace_device,
            result_instance: self.result_instance,
        }
    }
}

#[derive(Debug)]
pub struct PortRenderContext<'a> {
    pub workspace_device: &'a WorkspaceDevice,

    /// Set this to `true` to continuously update the screen.
    pub schedule_next_frame: bool,
}

/// Trait for rendering custom contents as layer contents.
pub trait PortInstance: fmt::Debug + Send + Sync + 'static {
    fn render(
        &mut self,
        context: &mut PortRenderContext<B>,
        frame: &PresenterFrame,
    ) -> B::ImageView;
}

/// Maintains port instances associated with `Port`s.
#[derive(Debug)]
pub(super) struct PortManager<B: Backend> {
    /// Set of mounted port instances.
    port_map: HashMap<RefEqArc<Port>, PortMapping<B>>,
}

#[derive(Debug)]
struct PortMapping<B: Backend> {
    instance: Option<Box<PortInstance<B>>>,
    used_in_last_frame: bool,
}

impl<B: Backend> PortManager<B> {
    pub fn new() -> Self {
        Self {
            port_map: HashMap::new(),
        }
    }

    /// Mark the start of a new frame.
    ///
    /// Destroys out-dated port instances (that is, whose nodes are no longer
    /// on the layer tree).
    pub fn prepare_frame(&mut self) {
        use std::mem::replace;
        self.port_map
            .retain(|_, map| replace(&mut map.used_in_last_frame, false));
    }

    pub fn get(
        &mut self,
        port: &RefEqArc<Port>,
        workspace_device: &WorkspaceDevice<B>,
    ) -> Option<&mut Box<PortInstance<B>>> {
        let ent = self.port_map.entry(RefEqArc::clone(port));
        let map = ent.or_insert_with(|| {
            // The port instance has not yet been created for the `Port`.
            // Mount the port and create the port instance.

            let mut instance_cell: Option<Box<PortInstance<B>>> = None;

            {
                let mut mount_context = PortMountContextWithBackend {
                    workspace_device,
                    result_instance: &mut instance_cell,
                };
                port.mount(&mut mount_context.upcast_mut());
            }

            // Save the created instance and return a reference to it
            PortMapping {
                instance: instance_cell,
                used_in_last_frame: true,
            }
        });
        map.used_in_last_frame = true;
        map.instance.as_mut()
    }
}
