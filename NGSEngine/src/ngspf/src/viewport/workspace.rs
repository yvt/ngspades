//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::Arc;
use std::collections::{HashMap, HashSet};

use winit::{self, EventsLoop};

use gfx;
use gfx::backends::{DefaultBackend, DefaultEnvironment};
use gfx::wsi::{DefaultWindow, NewWindow, Window, Swapchain, Drawable};
use gfx::core::{Environment, InstanceBuilder};
use gfx::prelude::*;

use context::{Context, KeyedProperty, NodeRef, KeyedPropertyAccessor, PropertyAccessor,
              for_each_node, PresenterFrame};
use super::{WindowFlagsBit, WorkspaceDevice};
use super::compositor::{Compositor, CompositorInstance, CompositeContext, CompositorWindow};
use prelude::*;

pub struct Workspace {
    events_loop: EventsLoop,
    context: Arc<Context>,
    windows: WorkspaceWindowSet,
    root: RootRef,
}

struct WorkspaceWindowSet {
    windows: HashSet<NodeRef>,
    device_windows: Vec<DeviceAndWindows<DefaultWindow>>,
    gfx_instance: <DefaultEnvironment as Environment>::Instance,
}

/// `WorkspaceDevice` and set of `Window`s rendered by the device.
#[derive(Debug)]
struct DeviceAndWindows<W: Window> {
    device: Arc<WorkspaceDevice<W::Backend>>,
    windows: HashMap<NodeRef, WorkspaceWindow<W>>,
    compositor: Arc<CompositorInstance<W::Backend>>,
}

#[derive(Debug)]
struct WorkspaceWindow<W: Window> {
    gfx_window: W,
    compositor_window: CompositorWindow<W::Backend>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WorkspaceError {
    OsError(String),
}

#[derive(Debug)]
struct Root {
    windows: KeyedProperty<Option<NodeRef>>,
}

pub struct RootRef(Arc<Root>);

impl Workspace {
    pub fn new() -> Result<Self, WorkspaceError> {
        let events_loop = EventsLoop::new();
        let context = Arc::new(Context::new());
        let root = Root { windows: KeyedProperty::new(&context, None) };

        Ok(Self {
            events_loop,
            context,
            windows: WorkspaceWindowSet::new(),
            root: RootRef(Arc::new(root)),
        })
    }

    pub fn context(&self) -> &Arc<Context> {
        &self.context
    }

    pub fn root(&self) -> &RootRef {
        &self.root
    }

    pub fn enter_main_loop(&mut self) -> Result<(), WorkspaceError> {
        DefaultBackend::autorelease_pool_scope(|arp| {
            loop {
                let ref mut events_loop = self.events_loop;
                events_loop.poll_events(|_| {
                    // TODO
                });

                {
                    let frame = self.context.lock_presenter_frame().expect(
                        "failed to acquire a presenter frame",
                    );

                    let windows = self.root.windows();
                    let windows = windows.get_presenter_ref(&frame).unwrap().as_ref();
                    self.windows.reconcile(windows, &frame, events_loop);
                    self.windows.update(&frame);
                }

                arp.drain();
            }
        })
    }
}

impl RootRef {
    pub fn windows<'a>(&'a self) -> impl PropertyAccessor<Option<NodeRef>> + 'a {
        fn select(this: &Arc<Root>) -> &KeyedProperty<Option<NodeRef>> {
            &this.windows
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }
}

impl WorkspaceWindowSet {
    fn new() -> Self {
        let mut instance_builder = <DefaultEnvironment as Environment>::InstanceBuilder::new()
            .expect("InstanceBuilder::new() have failed");
        DefaultWindow::modify_instance_builder(&mut instance_builder);
        instance_builder.enable_debug_report(
            gfx::core::DebugReportType::Information | gfx::core::DebugReportType::Warning |
                gfx::core::DebugReportType::PerformanceWarning |
                gfx::core::DebugReportType::Error,
            gfx::debug::report::TermStdoutDebugReportHandler::new(),
        );
        instance_builder.enable_validation();
        instance_builder.enable_debug_marker();

        let gfx_instance = instance_builder.build().expect(
            "InstanceBuilder::build() have failed",
        );

        WorkspaceWindowSet {
            windows: HashSet::new(),
            device_windows: Vec::new(),
            gfx_instance,
        }
    }

    fn reconcile(
        &mut self,
        windows: Option<&NodeRef>,
        frame: &PresenterFrame,
        events_loop: &EventsLoop,
    ) {
        // Enumerate all window nodes
        let mut nodes = HashSet::new();
        if let Some(windows) = windows {
            for_each_node(windows, |node_ref_ref| { nodes.insert(node_ref_ref); });
        }

        // Insert new windows
        for new_node in nodes.iter() {
            if self.windows.contains(new_node) {
                continue;
            }

            let window: &super::Window = new_node.downcast_ref().expect(
                "The property 'windows' must specify a set of window nodes",
            );

            // Construct a `WorkspaceWindow`
            use gfx::core::{ImageFormat, ImageUsage};
            use gfx::wsi::ColorSpace;

            let flags = window.flags;
            let size = window.size.read_presenter(&frame).unwrap().cast::<u32>();
            let title = window.title.read_presenter(&frame).unwrap().to_owned();

            let desired_formats = [
                (
                    Some(ImageFormat::SrgbBgra8),
                    Some(ColorSpace::SrgbNonlinear),
                ),
                (
                    Some(ImageFormat::SrgbRgba8),
                    Some(ColorSpace::SrgbNonlinear),
                ),
                (None, Some(ColorSpace::SrgbNonlinear)),
            ];
            let sc_desc = gfx::wsi::SwapchainDescription {
                desired_formats: &desired_formats,
                image_usage: ImageUsage::ColorAttachment.into(),
            };
            let mut builder = winit::WindowBuilder::new()
                .with_transparency(flags.contains(WindowFlagsBit::Transparent))
                .with_decorations(!flags.contains(WindowFlagsBit::Borderless))
                .with_dimensions(size.x, size.y)
                .with_title(title);
            if !flags.contains(WindowFlagsBit::Resizable) {
                builder = builder.with_max_dimensions(size.x, size.y);
                builder = builder.with_min_dimensions(size.x, size.y);
            }
            // TODO: reuse existing `WorkspaceDevice` somehow
            let gfx_window = DefaultWindow::new(builder, events_loop, &self.gfx_instance, &sc_desc)
                .unwrap();

            // TODO: handle the creation error gracefully
            use gfx::wsi::Window;
            let device = WorkspaceDevice::new(Arc::clone(gfx_window.device()))
                .expect("failed to create `WorkspaceDevice`");

            let comp = device.get_library(&Compositor);

            let ww = WorkspaceWindow {
                gfx_window,
                compositor_window: CompositorWindow::new(Arc::clone(&comp)),
            };

            let mut dws = DeviceAndWindows {
                device: Arc::new(device),
                windows: HashMap::new(),
                compositor: comp,
            };
            dws.windows.insert(NodeRef::clone(new_node), ww);

            self.device_windows.push(dws);
            self.windows.insert(NodeRef::clone(new_node));
        }

        // Remove old windows
        for device_windows in self.device_windows.iter_mut() {
            device_windows.windows.retain(|k, _| nodes.contains(k));
        }
        self.device_windows.retain(|dw| dw.windows.len() > 0);
        self.windows.retain(|w| nodes.contains(w));
    }

    fn update(&mut self, frame: &PresenterFrame) {
        for device_windows in self.device_windows.iter_mut() {
            device_windows.update(frame);
        }
    }
}

impl<W: Window> DeviceAndWindows<W> {
    fn update(&mut self, frame: &PresenterFrame) {
        let mut context = CompositeContext {
            workspace_device: &self.device,
            schedule_next_frame: false,
            command_buffers: Vec::new(),
        };
        let mut drawables = Vec::new();

        for (node, ww) in self.windows.iter_mut() {
            let window: &super::Window = node.downcast_ref().unwrap();
            let ref mut gfx_window = ww.gfx_window;
            let root = Option::clone(window.child.read_presenter(frame).unwrap());

            loop {
                {
                    let swapchain = gfx_window.swapchain();
                    let gfx_frame = ww.compositor_window.frame_description();
                    let drawable = swapchain.next_drawable(&gfx_frame);

                    match drawable {
                        Ok(drawable) => {
                            ww.compositor_window.composite(
                                &mut context,
                                &root,
                                frame,
                                &drawable,
                                &swapchain.drawable_info(),
                            );
                            drawables.push(drawable);
                            break;
                        }
                        Err(gfx::wsi::SwapchainError::OutOfDate) => {
                            // The swapchain is out of date. Need to update the swapchain
                            // to match the latest state.
                        }
                        Err(e) => {
                            // TODO: handle the error gracefully
                            panic!("Failed to acquire the next drawable.: {:?}", e);
                        }
                    }
                }

                // We have to wait for the completion because we have to ensure all uses of
                // swapchain images are completed before updating the swapchain.
                // TODO: wait command buffer execution completion
                //       note: `Device::wait_idle` is insufficient!
                gfx_window.update_swapchain();
            }

            // TODO: use `schedule_next_frame` to reduce CPU load
        }

        let mut command_buffers_ref: Vec<_> = context.command_buffers.iter_mut().collect();
        self.device
            .objects()
            .gfx_device()
            .main_queue()
            .submit_commands(&mut command_buffers_ref[..], None)
            .expect("Command submission failed");

        for drawable in drawables {
            drawable.present();
        }
    }
}
