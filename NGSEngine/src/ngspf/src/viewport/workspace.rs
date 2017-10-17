//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::{Arc, RwLock, Mutex};
use std::any::{Any, TypeId};
use std::{fmt, hash};
use std::collections::{HashMap, HashSet};

use winit::{self, EventsLoop};

use gfx;
use gfx::backends::{DefaultBackend, DefaultEnvironment};
use gfx::wsi::{DefaultWindow, NewWindow};
use gfx::core::{Environment, InstanceBuilder};
use gfx::prelude::*;

use context::{Context, KeyedProperty, NodeRef, KeyedPropertyAccessor, PropertyAccessor,
              for_each_node};
use super::Window;
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
    device: Arc<Device>,
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
                        let builder = winit::WindowBuilder::new();
                        let gfx_window =
                            DefaultWindow::new(builder, events_loop, instance, &sc_desc).unwrap();

                        let _size = *window.size.read_presenter(&frame).unwrap();

                        // TODO: handle the creation error gracefully
                        use gfx::wsi::Window;
                        let device = Device::new(Arc::clone(gfx_window.device())).expect(
                            "failed to create `Device`",
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

    // TODO: reuse existing `Device`

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

#[derive(Debug)]
pub struct Device {
    libraries: RwLock<LibraryMap>,
    objects: Arc<DeviceObjects>,
}

impl Device {
    fn new(
        gfx_device: Arc<<DefaultBackend as Backend>::Device>,
    ) -> Result<Self, gfx::core::GenericError> {
        use gfx::core::Device;
        let objects = DeviceObjects {
            heap: Arc::new(Mutex::new(gfx_device.factory().make_universal_heap()?)),
            gfx_device,
        };
        Ok(Self {
            libraries: RwLock::new(LibraryMap::new()),
            objects: Arc::new(objects),
        })
    }

    pub fn get_library<T: Library>(&self, library: &T) -> Arc<T::Instance> {
        if let Some(inst) = self.libraries.read().unwrap().get(library).cloned() {
            return inst;
        }

        self.libraries
            .write()
            .unwrap()
            .get_or_create(library, || library.make_instance(&self.objects))
            .clone()
    }
}

/// Dictionary of `Library::Instance`s.
///
/// Each entry contains the type ID of `T: Library` as its key and a boxed
/// `HashMap<T::LibraryId, Arc<T:Instance>>` as its value.
#[derive(Debug)]
struct LibraryMap(HashMap<TypeId, Box<Any>>);

impl LibraryMap {
    fn new() -> Self {
        LibraryMap(HashMap::new())
    }

    fn get<T: Library>(&self, library: &T) -> Option<&Arc<T::Instance>> {
        let type_id = TypeId::of::<T>();
        self.0.get(&type_id).and_then(|boxed_tlm| {
            let tlm: &HashMap<T::LibraryId, Arc<T::Instance>> = boxed_tlm.downcast_ref().unwrap();
            tlm.get(&library.id())
        })
    }

    fn get_or_create<T: Library, F>(&mut self, library: &T, factory: F) -> &Arc<T::Instance>
    where
        F: FnOnce() -> T::Instance,
    {
        let type_id = TypeId::of::<T>();
        let boxed_tlm = self.0.entry(type_id).or_insert_with(|| {
            Box::new(HashMap::<T::LibraryId, Arc<T::Instance>>::new())
        });
        let tlm: &mut HashMap<T::LibraryId, Arc<T::Instance>> = boxed_tlm.downcast_mut().unwrap();
        tlm.entry(library.id()).or_insert_with(
            || Arc::new(factory()),
        )
    }
}

/// NgsGFX objects associated with a certain NgsGFX device.
#[derive(Debug)]
pub struct DeviceObjects {
    gfx_device: Arc<<DefaultBackend as Backend>::Device>,
    heap: Arc<Mutex<<DefaultBackend as Backend>::UniversalHeap>>,
}

impl DeviceObjects {
    pub fn gfx_device(&self) -> &Arc<<DefaultBackend as Backend>::Device> {
        &self.gfx_device
    }

    pub fn heap(&self) -> &Arc<Mutex<<DefaultBackend as Backend>::UniversalHeap>> {
        &self.heap
    }
}

pub trait Library: Any + fmt::Debug {
    /// Identifier used to distingish multiple instances of this `Library`.
    type LibraryId: 'static + hash::Hash + Eq + fmt::Debug;

    /// Runtime data type associated with a specific `Device` and `Library`.
    type Instance: 'static + fmt::Debug;

    /// Get the `LibraryId` of the `Library`.
    fn id(&self) -> Self::LibraryId;

    /// Construct a `Instance` for a specific `Device`.
    fn make_instance(&self, device_objects: &Arc<DeviceObjects>) -> Self::Instance;
}
