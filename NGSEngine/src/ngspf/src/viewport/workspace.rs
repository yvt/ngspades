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
use super::compositor::{Compositor, CompositeContext, CompositorWindow};
use super::uploader::Uploader;
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
    compositor: Arc<Compositor<W::Backend>>,
    uploader: Uploader<W::Backend>,
    events: EventRing<W::Backend>,
}

/// Set of NgsGFX `Event`s. Used to wait for the device to become idle when
/// updating a swapchain. (`CommandQueue::wait_idle` is insufficient for this
/// use case)
#[derive(Debug)]
struct EventRing<B: Backend> {
    events: Vec<B::Event>,
    next: usize,
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
                        "failed to acquire a presenter frame (locked by an external entity?)",
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
                .expect("failed to create WorkspaceDevice");

            let comp = Arc::new(Compositor::new(&device).expect(
                "failed to create Compositor",
            ));

            let uploader = Uploader::new(&device).expect("failed to create Uploader");

            let ww = WorkspaceWindow {
                gfx_window,
                compositor_window: CompositorWindow::new(Arc::clone(&comp)).expect(
                    "failed to create `CompositorWindow`",
                ),
            };

            let mut dws = DeviceAndWindows {
                events: EventRing::<DefaultBackend>::new(device.objects().gfx_device())
                    .expect("failed to create EventRing"),
                device: Arc::new(device),
                uploader,
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
            if device_windows.windows.len() == 0 {
                device_windows.wait_idle();
            }
        }
        self.device_windows.retain(|dw| dw.windows.len() > 0);
        self.windows.retain(|w| nodes.contains(w));
    }

    fn update(&mut self, frame: &PresenterFrame) {
        for device_windows in self.device_windows.iter_mut() {
            device_windows.update(frame).expect(
                "failed to update a device",
            );
        }
    }
}

impl<W: Window> DeviceAndWindows<W> {
    fn update(&mut self, frame: &PresenterFrame) -> gfx::core::Result<()> {
        let mut context = CompositeContext {
            workspace_device: &self.device,
            schedule_next_frame: false,
            command_buffers: Vec::new(),
        };
        let mut drawables = Vec::new();

        // Upload images
        self.uploader.clear_image_uses();
        for (node, _) in self.windows.iter_mut() {
            let window: &super::Window = node.downcast_ref().unwrap();
            let root = Option::clone(window.child.read_presenter(frame).unwrap());
            if let Some(ref root) = root {
                self.uploader.scan_nodes(root, frame);
            }
        }
        context.command_buffers = self.uploader.upload(frame)?;

        // Composite windows
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
                            )?;
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
                self.events.wait_idle()?;
                gfx_window.update_swapchain();
            }

            // TODO: use `schedule_next_frame` to reduce CPU load
        }

        let mut command_buffers_borrowed: Vec<_> = context
            .command_buffers
            .iter()
            .map(|cb_cell| cb_cell.borrow_mut())
            .collect();
        let mut command_buffers_ref: Vec<_> = command_buffers_borrowed
            .iter_mut()
            .map(|borrowed| &mut **borrowed)
            .collect();
        let event = self.events.get()?;
        self.device
            .objects()
            .gfx_device()
            .main_queue()
            .submit_commands(&mut command_buffers_ref[..], Some(event))?;

        for drawable in drawables {
            drawable.present();
        }

        Ok(())
    }

    fn wait_idle(&self) {
        let _ = self.events.wait_idle();
    }
}

impl<B: Backend> EventRing<B> {
    fn new(device: &B::Device) -> gfx::core::Result<Self> {
        Ok(Self {
            events: (0..16)
                .map(|_| {
                    device.factory().make_event(&gfx::core::EventDescription {
                        signaled: true,
                    })
                })
                .collect::<Result<_, _>>()?,
            next: 0,
        })
    }

    fn get(&mut self) -> gfx::core::Result<&B::Event> {
        use std::time::Duration;
        self.next += 1;
        if self.next >= self.events.len() {
            self.next = 0;
        }
        let ref e = self.events[self.next];
        while !e.wait(Duration::from_secs(1))? {}
        e.reset()?;
        Ok(e)
    }

    fn wait_idle(&self) -> gfx::core::Result<()> {
        use std::time::Duration;
        while !Event::wait_all(&self.events, Duration::from_secs(1))? {}
        Ok(())
    }
}
