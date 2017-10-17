//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::Arc;
use std::any::Any;
use std::collections::{HashMap, HashSet};

use winit::{self, EventsLoop};

use gfx;
use gfx::backends::{DefaultBackend, DefaultEnvironment};
use gfx::wsi::{DefaultWindow, NewWindow};
use gfx::core::{Environment, InstanceBuilder};
use gfx::prelude::*;

use context::{Context, KeyedProperty, NodeRef, KeyedPropertyAccessor, PropertyAccessor,
              for_each_node};
use super::{Window, WindowFlagsBit, WorkspaceDevice};
use prelude::*;

pub struct Workspace {
    events_loop: EventsLoop,
    context: Arc<Context>,
    workspace_windows: HashMap<NodeRef, WorkspaceWindow>,
    gfx_instance: <DefaultEnvironment as Environment>::Instance,
    root: RootRef,
}

#[derive(Debug)]
struct WorkspaceWindow {
    gfx_window: DefaultWindow,
    device: Arc<WorkspaceDevice<DefaultBackend>>,
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

        Ok(Self {
            events_loop,
            context,
            workspace_windows: HashMap::new(),
            gfx_instance,
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
                    let ref instance = self.gfx_instance;
                    reconcile_window_set(&mut self.workspace_windows, windows, |window| {
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
                        let gfx_window =
                            DefaultWindow::new(builder, events_loop, instance, &sc_desc).unwrap();

                        // TODO: handle the creation error gracefully
                        use gfx::wsi::Window;
                        let device = WorkspaceDevice::new(Arc::clone(gfx_window.device())).expect(
                            "failed to create `WorkspaceDevice`",
                        );

                        WorkspaceWindow {
                            gfx_window,
                            device: Arc::new(device),
                        }
                    });
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

fn reconcile_window_set<F>(
    ww_map: &mut HashMap<NodeRef, WorkspaceWindow>,
    windows: Option<&NodeRef>,
    mut window_factory: F,
) where
    F: FnMut(&Window) -> WorkspaceWindow,
{
    // Enumerate all window nodes
    let mut nodes = HashSet::new();
    if let Some(windows) = windows {
        for_each_node(windows, |node_ref_ref| { nodes.insert(node_ref_ref); });
    }

    // TODO: reuse existing `WorkspaceDevice`

    // Insert new windows
    for new_node in nodes.iter() {
        if ww_map.contains_key(new_node) {
            continue;
        }

        let window: &Window = Any::downcast_ref(&*new_node.0).expect(
            "The property 'windows' must specify a set of window nodes",
        );
        let ww = window_factory(window);
        assert!(ww_map.insert(NodeRef::clone(new_node), ww).is_none());
    }

    // Remove old windows
    ww_map.retain(|k, _| nodes.contains(k));
}

