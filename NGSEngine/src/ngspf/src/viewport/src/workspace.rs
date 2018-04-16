//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use cgmath::Vector2;
use ngsenumflags::BitFlags;
use winit::{self, EventsLoop};

use super::compositor::{CompositeContext, Compositor, CompositorWindow};
use super::{Window, WindowActionBit, WindowFlagsBit};
use core::prelude::*;
use core::{Context, KeyedProperty, KeyedPropertyAccessor, NodeRef, PresenterFrame,
           ProducerDataCell, ProducerFrame, PropertyAccessor, PropertyError, UpdateId, WoProperty};

use super::wsi;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WorkspaceError {
    OsError(String),
}

#[derive(Debug)]
struct Root {
    windows: KeyedProperty<Option<NodeRef>>,
    exit_loop: WoProperty<bool>,
    exit_loop_update_id: ProducerDataCell<UpdateId>,
}

#[derive(Debug, Clone)]
pub struct RootRef(Arc<Root>);

impl RootRef {
    pub fn windows<'a>(&'a self) -> impl PropertyAccessor<Option<NodeRef>> + 'a {
        fn select(this: &Arc<Root>) -> &KeyedProperty<Option<NodeRef>> {
            &this.windows
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }

    pub fn exit_loop(&self, frame: &mut ProducerFrame) -> Result<(), PropertyError> {
        let update_id = *self.0.exit_loop_update_id.read_producer(frame)?;

        let new_id = frame.record_keyed_update(
            update_id,
            |_| true,
            || {
                let c = Arc::clone(&self.0);
                move |frame, value| {
                    *c.exit_loop.write_presenter(frame).unwrap() = value;
                }
            },
        );

        *self.0.exit_loop_update_id.write_producer(frame)? = new_id;

        Ok(())
    }
}

pub struct Workspace {
    events_loop: EventsLoop,
    context: Arc<Context>,
    window_set: WindowSet,
    root: RootRef,
}

impl Workspace {
    pub fn new() -> Result<Self, WorkspaceError> {
        let events_loop = EventsLoop::new();
        let context = Arc::new(Context::new());
        let root = Root {
            windows: KeyedProperty::new(&context, None),
            exit_loop: WoProperty::new(&context, false),
            exit_loop_update_id: ProducerDataCell::new(&context, UpdateId::new()),
        };

        let events_loop_proxy = events_loop.create_proxy();

        // Work-around for the issue caused by calling
        // `EventsLoopProxy::wakeup()` too early from a background thread.
        // (See: https://github.com/tomaka/winit/pull/456)
        let _ = events_loop_proxy.wakeup();

        {
            // Trigger window reconcilation whenever a new frame was submitted
            let events_loop_proxy = events_loop_proxy.clone();
            context.on_commit(move || {
                let _ = events_loop_proxy.wakeup();
            });
        }

        Ok(Self {
            events_loop,
            context,
            window_set: WindowSet::new(events_loop_proxy),
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
        let mut events = Vec::new();

        wsi::autorelease_pool_scope(|arp| loop {
            let ref mut events_loop = self.events_loop;

            // Wait until we receieve at least one event
            events_loop.run_forever(|e| {
                events.push(e);
                winit::ControlFlow::Break
            });

            {
                let mut frame = self.context
                    .lock_presenter_frame()
                    .expect("failed to acquire a presenter frame (locked by an external entity?)");

                {
                    let ref window_set = self.window_set;

                    for e in events.drain(..) {
                        match e {
                            winit::Event::WindowEvent { window_id, event } => {
                                window_set.handle_window_event(window_id, event, &mut frame);
                            }
                            _ => {}
                        }
                    }

                    events_loop.poll_events(|e| match e {
                        winit::Event::WindowEvent { window_id, event } => {
                            window_set.handle_window_event(window_id, event, &mut frame);
                        }
                        _ => {}
                    });
                }

                use std::mem::replace;
                if replace(
                    self.root.0.exit_loop.write_presenter(&mut frame).unwrap(),
                    false,
                ) {
                    return Ok(());
                }

                {
                    let windows = self.root.windows();
                    let windows = windows.get_presenter_ref(&frame).unwrap().as_ref();
                    self.window_set.reconcile(windows, &frame, events_loop);
                }
                self.window_set.update(&mut frame);
            }

            arp.drain();
        })
    }
}

#[derive(Debug)]
struct WindowSet {
    windows: HashMap<NodeRef, WorkspaceWindow>,
    wm: wsi::WindowManager<Painter>,
}

struct WorkspaceWindow {
    surface: wsi::SurfaceRef,
    winit_window_id: winit::WindowId,
}

impl ::Debug for WorkspaceWindow {
    fn fmt(&self, fmt: &mut ::fmt::Formatter) -> ::fmt::Result {
        fmt.debug_struct("WorkspaceWindow")
            .field("surface", &self.surface)
            .finish()
    }
}

impl WindowSet {
    fn new(events_loop_proxy: winit::EventsLoopProxy) -> Self {
        WindowSet {
            windows: HashMap::new(),
            wm: wsi::WindowManager::new(Painter::new(), events_loop_proxy),
        }
    }

    fn handle_window_event(
        &self,
        win_id: winit::WindowId,
        winit_event: winit::WindowEvent,
        frame: &mut PresenterFrame,
    ) {
        use super::{KeyModifier, KeyModifierFlags, MouseButton, MousePosition, WindowEvent};

        if let Some((node_ref, winit_win)) = self.node_ref_and_winit_win_with_window_id(win_id) {
            let win: &Window = node_ref.downcast_ref().unwrap();

            // Translate it to our `WindowEvent`
            let event = match winit_event {
                winit::WindowEvent::Resized(w, h) => {
                    let ratio = winit_win.hidpi_factor();
                    let size = Vector2::new(w, h).cast::<f32>();
                    Some(WindowEvent::Resized(size / ratio))
                }
                winit::WindowEvent::Moved(x, y) => {
                    // FIXME: Should be these coordinates divided by `ratio`? These are global
                    //        coordinates, not client ...
                    Some(WindowEvent::Moved(Vector2::new(x, y).cast()))
                }
                winit::WindowEvent::Closed => Some(WindowEvent::Close),
                winit::WindowEvent::MouseInput { state, button, .. } => {
                    win.mouse_pos.read_presenter(frame).unwrap().map(|pos| {
                        let button = match button {
                            winit::MouseButton::Left => MouseButton::Left,
                            winit::MouseButton::Right => MouseButton::Right,
                            winit::MouseButton::Middle => MouseButton::Middle,
                            winit::MouseButton::Other(x) => MouseButton::Other(x),
                        };
                        let pressed = state == winit::ElementState::Pressed;
                        WindowEvent::MouseButton(pos, button, pressed)
                    })
                }
                winit::WindowEvent::CursorMoved {
                    position: (x, y), ..
                } => {
                    // Translate the coordinate to `MousePosition`
                    let ratio = winit_win.hidpi_factor();
                    let client = Vector2::new(x, y).cast::<f32>() / ratio;
                    let (wx, wy) = winit_win.get_position().unwrap_or((0, 0));
                    let global = client + Vector2::new(wx, wy).cast();
                    let pos = Some(MousePosition { client, global });

                    // Update the internal cursor location
                    // (used to handle mouse press/release events)
                    *win.mouse_pos.write_presenter(frame).unwrap() = pos;

                    Some(WindowEvent::MouseMotion(pos))
                }
                winit::WindowEvent::CursorLeft { .. } => {
                    *win.mouse_pos.write_presenter(frame).unwrap() = None;
                    Some(WindowEvent::MouseMotion(None))
                }
                winit::WindowEvent::KeyboardInput { input, .. } => {
                    input.virtual_keycode.map(|vk| {
                        let mut keymod = KeyModifierFlags::empty();
                        if input.modifiers.shift {
                            keymod |= KeyModifier::Shift;
                        }
                        if input.modifiers.ctrl {
                            keymod |= KeyModifier::Control;
                        }
                        if input.modifiers.alt {
                            keymod |= KeyModifier::Alt;
                        }
                        if input.modifiers.logo {
                            keymod |= KeyModifier::Meta;
                        }

                        let pressed = input.state == winit::ElementState::Pressed;
                        WindowEvent::KeyboardInput(vk, pressed, keymod)
                    })
                }
                _ => None,
            };

            if let Some(ref listener) = *win.listener.read_presenter(frame).unwrap() {
                if let Some(ref event) = event {
                    listener(event);
                }
            }
        }
    }

    fn node_ref_and_winit_win_with_window_id(
        &self,
        id: winit::WindowId,
    ) -> Option<(&NodeRef, &winit::Window)> {
        for (node, workspace_window) in self.windows.iter() {
            if workspace_window.winit_window_id == id {
                let winit_window = self.wm.get_winit_window(workspace_window.surface).unwrap();
                return Some((node, winit_window));
            }
        }

        None
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
            windows.for_each_node(|node_ref_ref| {
                nodes.insert(node_ref_ref);
            });
        }

        // Insert new windows
        for new_node in nodes.iter() {
            if self.windows.contains_key(new_node) {
                continue;
            }

            let window: &Window = new_node
                .downcast_ref()
                .expect("The property 'windows' must specify a set of window nodes");

            // Construct a `WorkspaceWindow`
            let flags = window.flags;
            let size = window.size.read_presenter(&frame).unwrap().cast::<u32>();
            let title = window.title.read_presenter(&frame).unwrap().to_owned();

            let mut builder = winit::WindowBuilder::new()
                .with_transparency(flags.contains(WindowFlagsBit::Transparent))
                .with_decorations(!flags.contains(WindowFlagsBit::Borderless))
                .with_dimensions(size.x, size.y)
                .with_title(title);
            if !flags.contains(WindowFlagsBit::Resizable) {
                builder = builder.with_max_dimensions(size.x, size.y);
                builder = builder.with_min_dimensions(size.x, size.y);
            }

            let winit_window = builder
                .build(events_loop)
                .expect("failed to instantiate a window.");
            let winit_window_id = winit_window.id();

            let surface = self.wm.add_surface(winit_window, NodeRef::clone(new_node));

            let workspace_window = WorkspaceWindow {
                winit_window_id,
                surface,
            };

            self.windows
                .insert(NodeRef::clone(new_node), workspace_window);
        }

        // Remove old windows
        let ref mut wm = self.wm;
        self.windows.retain(|k, workspace_window| {
            nodes.contains(k) || {
                wm.remove_surface(workspace_window.surface);
                false
            }
        });
    }

    fn update(&mut self, frame: &mut PresenterFrame) {
        // Update window properties
        for (node, workspace_window) in self.windows.iter_mut() {
            let window: &Window = node.downcast_ref().unwrap();
            let winit_window = self.wm.get_winit_window(workspace_window.surface).unwrap();

            use std::mem::replace;
            let action = replace(
                window.action.write_presenter(frame).unwrap(),
                BitFlags::empty(),
            );
            if action.contains(WindowActionBit::ChangeSize) {
                let new_value = window.size.read_presenter(frame).unwrap().cast::<u32>();
                winit_window.set_inner_size(new_value.x, new_value.y);
            }
            if action.contains(WindowActionBit::ChangeTitle) {
                let new_value = window.title.read_presenter(frame).unwrap();
                winit_window.set_title(new_value);
            }
        }

        self.wm.update(frame);
    }
}

#[derive(Debug)]
struct Painter;

impl Painter {
    fn new() -> Self {
        Painter
    }
}

#[derive(Debug)]
struct PainterDeviceData {
    compositor: Rc<RefCell<Compositor>>,
}

#[derive(Debug)]
struct PainterSurfaceData {
    node_ref: NodeRef,
    compositor_window: CompositorWindow,
}

impl wsi::Painter for Painter {
    type DeviceData = PainterDeviceData;

    type SurfaceParam = NodeRef;

    type SurfaceData = PainterSurfaceData;

    type UpdateParam = PresenterFrame;

    fn add_device(&mut self, device: &wsi::WmDevice) -> Self::DeviceData {
        use port::GfxObjects;
        let compositor = Compositor::new(&GfxObjects {
            device: device.device.clone(),
            main_queue: device.main_queue.clone(),
            copy_queue: device.copy_queue.clone(),
        }).unwrap();

        PainterDeviceData {
            compositor: Rc::new(RefCell::new(compositor)),
        }
    }

    fn remove_device(&mut self, _device: &wsi::WmDevice, _data: Self::DeviceData) {}

    fn add_surface(
        &mut self,
        _device: &wsi::WmDevice,
        device_data: &mut Self::DeviceData,
        _surface: &wsi::SurfaceRef,
        param: Self::SurfaceParam,
        _surface_props: &wsi::SurfaceProps,
    ) -> Self::SurfaceData {
        let compositor_window = CompositorWindow::new(device_data.compositor.clone()).unwrap();
        PainterSurfaceData {
            node_ref: param,
            compositor_window,
        }
    }

    fn remove_surface(
        &mut self,
        _device: &wsi::WmDevice,
        _device_data: &mut Self::DeviceData,
        _surface: &wsi::SurfaceRef,
        data: Self::SurfaceData,
    ) -> Self::SurfaceParam {
        data.node_ref
    }

    fn update_surface(
        &mut self,
        _device: &wsi::WmDevice,
        _device_data: &mut Self::DeviceData,
        _surface: &wsi::SurfaceRef,
        _data: &mut Self::SurfaceData,
        _surface_props: &wsi::SurfaceProps,
    ) {
    }

    fn paint(
        &mut self,
        _device: &wsi::WmDevice,
        _device_data: &mut Self::DeviceData,
        _surface: &wsi::SurfaceRef,
        surface_data: &mut Self::SurfaceData,
        update_param: &Self::UpdateParam,
        drawable: &mut wsi::Drawable,
    ) {
        let frame = update_param;

        let window: &Window = surface_data
            .node_ref
            .downcast_ref()
            .expect("The property 'windows' must specify a set of window nodes");

        let window_root = window.child.read_presenter(frame).unwrap();

        surface_data
            .compositor_window
            .composite(
                &mut CompositeContext {
                    schedule_next_frame: false,
                    pixel_ratio: drawable.pixel_ratio(),
                },
                window_root,
                frame,
                drawable,
            )
            .unwrap();
    }
}
