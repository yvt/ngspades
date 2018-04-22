//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use atomic_refcell::AtomicRefCell;
use ngscom::{hresults, BString, BStringRef, ComPtr, HResult, IUnknown, IUnknownTrait,
             UnownedComPtr};
use std::sync::Arc;
use {cggeom, cgmath, ngsbase, rgb};

use core::prelude::*;
use hresults::{E_PF_LOCKED, E_PF_NODE_MATERIALIZED, E_PF_NOT_NODE};
use {core, viewport};
use {ILayer, INodeGroup, IWindow, IWindowListener};

pub(crate) fn translate_context_error(e: core::ContextError) -> HResult {
    match e {
        core::ContextError::LockFailed => E_PF_LOCKED,
    }
}

com_iid!(
    IID_INODEREF = [
        0xbf41aa07,
        0x035e,
        0x46eb,
        [0xa6, 0x1b, 0x90, 0xca, 0xb2, 0x95, 0x56, 0x53],
    ]
);

com_interface! {
    /// COM interface that provides a method to create a `NodeRef` of a node.
    ///
    /// This interface is not exported because it would have no use outside the
    /// Rust environment.
    interface (INodeRef, INodeRefTrait): (IUnknown, IUnknownTrait) {
        iid: IID_INODEREF,
        vtable: INodeRefVTable,

        fn create_node_ref() -> core::NodeRef;
    }
}

/// Thread-safe node data that has the partial and materialized states.
#[derive(Debug)]
struct NodeData<P, M> {
    cell: AtomicRefCell<NodeDataInner<P, M>>,
}

#[derive(Debug)]
enum NodeDataInner<P, M> {
    Partial(P),
    Materialized(M),
    Invalid,
}

#[derive(Debug)]
enum NodeDataState<P, M> {
    Partial(P),
    Materialized(M),
}

impl<P, M> NodeData<P, M> {
    fn new(x: P) -> Self {
        Self {
            cell: AtomicRefCell::new(NodeDataInner::Partial(x)),
        }
    }

    fn with_materialized<C: FnOnce(P) -> M, F: FnOnce(&M) -> R, R>(&self, ctor: C, f: F) -> R {
        self.with(move |state| {
            if let Some(m) = state.materialized() {
                Ok(f(m))
            } else {
                Err(f)
            }
        }).unwrap_or_else(move |f| {
            self.materialize(ctor);
            self.with(move |state| f(state.materialized().unwrap()))
        })
    }

    fn materialize<C: FnOnce(P) -> M>(&self, ctor: C) {
        use std::mem::replace;
        let mut state = self.cell.borrow_mut();
        let new_state = match replace(&mut *state, NodeDataInner::Invalid) {
            NodeDataInner::Materialized(m) => NodeDataInner::Materialized(m),
            NodeDataInner::Partial(p) => NodeDataInner::Materialized(ctor(p)),
            _ => panic!("NodeData is poisoned"),
        };
        *state = new_state;
    }

    fn with<F: FnOnce(NodeDataState<&P, &M>) -> R, R>(&self, f: F) -> R {
        let state = self.cell.borrow();
        match *state {
            NodeDataInner::Materialized(ref m) => f(NodeDataState::Materialized(m)),
            NodeDataInner::Partial(ref p) => f(NodeDataState::Partial(p)),
            _ => panic!("NodeData is poisoned"),
        }
    }

    fn with_mut<F: FnOnce(NodeDataState<&mut P, &mut M>) -> R, R>(&self, f: F) -> R {
        let mut state = self.cell.borrow_mut();
        match *state {
            NodeDataInner::Materialized(ref mut m) => f(NodeDataState::Materialized(m)),
            NodeDataInner::Partial(ref mut p) => f(NodeDataState::Partial(p)),
            _ => panic!("NodeData is poisoned"),
        }
    }
}

impl<P, M> NodeDataState<P, M> {
    #[allow(dead_code)]
    fn partial(&self) -> Option<&P> {
        match self {
            &NodeDataState::Partial(ref p) => Some(p),
            _ => None,
        }
    }

    fn materialized(&self) -> Option<&M> {
        match self {
            &NodeDataState::Materialized(ref m) => Some(m),
            _ => None,
        }
    }
}

com_impl! {
    #[derive(Debug)]
    class ComNodeGroup {
        inode_group: INodeGroup;
        inoderef: INodeRef;
        @data: NodeData<Vec<core::NodeRef>, core::GroupRef>;
    }
}

impl ComNodeGroup {
    pub fn new() -> ComPtr<INodeGroup> {
        ComPtr::from(&ComNodeGroup::alloc(NodeData::new(Vec::new())))
    }
}

impl ngsbase::INodeGroupTrait for ComNodeGroup {
    fn insert(&self, node: UnownedComPtr<IUnknown>) -> HResult {
        self.data.with_mut(|s| match s {
            NodeDataState::Materialized(_) => E_PF_NODE_MATERIALIZED,
            NodeDataState::Partial(p) => {
                let inoderef = ComPtr::<INodeRef>::from(&*node);
                if inoderef.is_null() {
                    return E_PF_NOT_NODE;
                }

                p.push(inoderef.create_node_ref());
                hresults::E_OK
            }
        })
    }
}

impl INodeRefTrait for ComNodeGroup {
    fn create_node_ref(&self) -> core::NodeRef {
        self.data.with_materialized(
            |p| core::GroupRef::new(p),
            |group_ref| group_ref.clone().into_node_ref(),
        )
    }
}

com_impl! {
    class ComLayer {
        ilayer: ILayer;
        inoderef: INodeRef;
        @data: (Arc<core::Context>, NodeData<Option<viewport::LayerBuilder>, viewport::LayerRef>);
    }
}

impl ComLayer {
    pub fn new(context: Arc<core::Context>) -> ComPtr<ILayer> {
        ComPtr::from(&ComLayer::alloc((
            context,
            NodeData::new(Some(viewport::LayerBuilder::new())),
        )))
    }
}

impl ngsbase::ILayerTrait for ComLayer {
    fn set_opacity(&self, value: f32) -> HResult {
        let ref context: core::Context = *self.data.0;
        self.data
            .1
            .with_mut(|s| match s {
                NodeDataState::Partial(builder) => {
                    let b: viewport::LayerBuilder = builder.take().unwrap();
                    *builder = Some(b.opacity(value));
                    Ok(())
                }
                NodeDataState::Materialized(layer) => {
                    let mut frame = context
                        .lock_producer_frame()
                        .map_err(translate_context_error)?;
                    layer.opacity().set(&mut frame, value).unwrap();
                    Ok(())
                }
            })
            .err()
            .unwrap_or(hresults::E_OK)
    }

    fn set_transform(&self, value: cgmath::Matrix4<f32>) -> HResult {
        let ref context: core::Context = *self.data.0;
        self.data
            .1
            .with_mut(|s| match s {
                NodeDataState::Partial(builder) => {
                    let b: viewport::LayerBuilder = builder.take().unwrap();
                    *builder = Some(b.transform(value));
                    Ok(())
                }
                NodeDataState::Materialized(layer) => {
                    let mut frame = context
                        .lock_producer_frame()
                        .map_err(translate_context_error)?;
                    layer.transform().set(&mut frame, value).unwrap();
                    Ok(())
                }
            })
            .err()
            .unwrap_or(hresults::E_OK)
    }

    fn set_flags(&self, flags: ngsbase::LayerFlags) -> HResult {
        let mut value = viewport::LayerFlags::empty();
        if flags.contains(ngsbase::LayerFlagsItem::FlattenContents) {
            value |= viewport::LayerFlagsBit::FlattenContents;
        }

        let ref context: core::Context = *self.data.0;
        self.data
            .1
            .with_mut(|s| match s {
                NodeDataState::Partial(builder) => {
                    let b: viewport::LayerBuilder = builder.take().unwrap();
                    *builder = Some(b.flags(value));
                    Ok(())
                }
                NodeDataState::Materialized(layer) => {
                    let mut frame = context
                        .lock_producer_frame()
                        .map_err(translate_context_error)?;
                    layer.flags().set(&mut frame, value).unwrap();
                    Ok(())
                }
            })
            .err()
            .unwrap_or(hresults::E_OK)
    }

    fn set_bounds(&self, value: cggeom::Box2<f32>) -> HResult {
        let ref context: core::Context = *self.data.0;
        self.data
            .1
            .with_mut(|s| match s {
                NodeDataState::Partial(builder) => {
                    let b: viewport::LayerBuilder = builder.take().unwrap();
                    *builder = Some(b.bounds(value));
                    Ok(())
                }
                NodeDataState::Materialized(layer) => {
                    let mut frame = context
                        .lock_producer_frame()
                        .map_err(translate_context_error)?;
                    layer.bounds().set(&mut frame, value).unwrap();
                    Ok(())
                }
            })
            .err()
            .unwrap_or(hresults::E_OK)
    }

    fn set_child(&self, value: UnownedComPtr<IUnknown>) -> HResult {
        let value = if value.is_null() {
            None
        } else {
            let inoderef = ComPtr::<INodeRef>::from(&*value);
            if inoderef.is_null() {
                return E_PF_NOT_NODE;
            }
            Some(inoderef.create_node_ref())
        };

        let ref context: core::Context = *self.data.0;
        self.data
            .1
            .with_mut(|s| match s {
                NodeDataState::Partial(builder) => {
                    let b: viewport::LayerBuilder = builder.take().unwrap();
                    *builder = Some(b.child(value));
                    Ok(())
                }
                NodeDataState::Materialized(layer) => {
                    let mut frame = context
                        .lock_producer_frame()
                        .map_err(translate_context_error)?;
                    layer.child().set(&mut frame, value).unwrap();
                    Ok(())
                }
            })
            .err()
            .unwrap_or(hresults::E_OK)
    }

    fn set_mask(&self, value: UnownedComPtr<IUnknown>) -> HResult {
        let value = if value.is_null() {
            None
        } else {
            let inoderef = ComPtr::<INodeRef>::from(&*value);
            if inoderef.is_null() {
                return E_PF_NOT_NODE;
            }
            Some(inoderef.create_node_ref())
        };

        let ref context: core::Context = *self.data.0;
        self.data
            .1
            .with_mut(|s| match s {
                NodeDataState::Partial(builder) => {
                    let b: viewport::LayerBuilder = builder.take().unwrap();
                    *builder = Some(b.mask(value));
                    Ok(())
                }
                NodeDataState::Materialized(layer) => {
                    let mut frame = context
                        .lock_producer_frame()
                        .map_err(translate_context_error)?;
                    layer.mask().set(&mut frame, value).unwrap();
                    Ok(())
                }
            })
            .err()
            .unwrap_or(hresults::E_OK)
    }

    fn set_solid_color(&self, value: rgb::RGBA<f32>) -> HResult {
        let value = viewport::LayerContents::Solid(value);
        let ref context: core::Context = *self.data.0;
        self.data
            .1
            .with_mut(|s| match s {
                NodeDataState::Partial(builder) => {
                    let b: viewport::LayerBuilder = builder.take().unwrap();
                    *builder = Some(b.contents(value));
                    Ok(())
                }
                NodeDataState::Materialized(layer) => {
                    let mut frame = context
                        .lock_producer_frame()
                        .map_err(translate_context_error)?;
                    layer.contents().set(&mut frame, value).unwrap();
                    Ok(())
                }
            })
            .err()
            .unwrap_or(hresults::E_OK)
    }
}

impl INodeRefTrait for ComLayer {
    fn create_node_ref(&self) -> core::NodeRef {
        let ref context = self.data.0;
        self.data.1.with_materialized(
            |p| p.unwrap().build(context),
            |layer_ref| layer_ref.clone().into_node_ref(),
        )
    }
}

com_impl! {
    class ComWindow {
        iwindow: IWindow;
        inoderef: INodeRef;
        @data: (Arc<core::Context>, NodeData<Option<viewport::WindowBuilder>, viewport::WindowRef>);
    }
}

impl ComWindow {
    pub fn new(context: Arc<core::Context>) -> ComPtr<IWindow> {
        ComPtr::from(&ComWindow::alloc((
            context,
            NodeData::new(Some(viewport::WindowBuilder::new())),
        )))
    }
}

impl ngsbase::IWindowTrait for ComWindow {
    fn set_flags(&self, flags: ngsbase::WindowFlags) -> HResult {
        let mut value = viewport::WindowFlags::empty();
        if flags.contains(ngsbase::WindowFlagsItem::Resizable) {
            value |= viewport::WindowFlagsBit::Resizable;
        }
        if flags.contains(ngsbase::WindowFlagsItem::Borderless) {
            value |= viewport::WindowFlagsBit::Borderless;
        }
        if flags.contains(ngsbase::WindowFlagsItem::Transparent) {
            value |= viewport::WindowFlagsBit::Transparent;
        }
        if flags.contains(ngsbase::WindowFlagsItem::DenyUserClose) {
            value |= viewport::WindowFlagsBit::DenyUserClose;
        }

        self.data
            .1
            .with_mut(|s| match s {
                NodeDataState::Partial(builder) => {
                    let b: viewport::WindowBuilder = builder.take().unwrap();
                    *builder = Some(b.flags(value));
                    Ok(())
                }
                NodeDataState::Materialized(_window) => Err(E_PF_NODE_MATERIALIZED),
            })
            .err()
            .unwrap_or(hresults::E_OK)
    }

    fn set_size(&self, value: cgmath::Vector2<f32>) -> HResult {
        let ref context: core::Context = *self.data.0;
        self.data
            .1
            .with_mut(|s| match s {
                NodeDataState::Partial(builder) => {
                    let b: viewport::WindowBuilder = builder.take().unwrap();
                    *builder = Some(b.size(value));
                    Ok(())
                }
                NodeDataState::Materialized(window) => {
                    let mut frame = context
                        .lock_producer_frame()
                        .map_err(translate_context_error)?;
                    window.size().set(&mut frame, value).unwrap();
                    Ok(())
                }
            })
            .err()
            .unwrap_or(hresults::E_OK)
    }

    fn set_child(&self, value: UnownedComPtr<IUnknown>) -> HResult {
        let value = if value.is_null() {
            None
        } else {
            let inoderef = ComPtr::<INodeRef>::from(&*value);
            if inoderef.is_null() {
                return E_PF_NOT_NODE;
            }
            Some(inoderef.create_node_ref())
        };

        let ref context: core::Context = *self.data.0;
        self.data
            .1
            .with_mut(|s| match s {
                NodeDataState::Partial(builder) => {
                    let b: viewport::WindowBuilder = builder.take().unwrap();
                    *builder = Some(b.child(value));
                    Ok(())
                }
                NodeDataState::Materialized(window) => {
                    let mut frame = context
                        .lock_producer_frame()
                        .map_err(translate_context_error)?;
                    window.child().set(&mut frame, value).unwrap();
                    Ok(())
                }
            })
            .err()
            .unwrap_or(hresults::E_OK)
    }

    fn set_title(&self, value: Option<&BString>) -> HResult {
        let value = if let Some(value) = value {
            value.as_str().to_owned()
        } else {
            String::new()
        };
        let ref context: core::Context = *self.data.0;
        self.data
            .1
            .with_mut(|s| match s {
                NodeDataState::Partial(builder) => {
                    let b: viewport::WindowBuilder = builder.take().unwrap();
                    *builder = Some(b.title(value));
                    Ok(())
                }
                NodeDataState::Materialized(window) => {
                    let mut frame = context
                        .lock_producer_frame()
                        .map_err(translate_context_error)?;
                    window.title().set(&mut frame, value).unwrap();
                    Ok(())
                }
            })
            .err()
            .unwrap_or(hresults::E_OK)
    }

    fn set_listener(&self, value: UnownedComPtr<IWindowListener>) -> HResult {
        fn trans_mouse_pos(p: viewport::MousePosition) -> ngsbase::MousePosition {
            ngsbase::MousePosition {
                client: p.client,
                global: p.global,
            }
        }
        let value: Option<viewport::WindowListener> = if !value.is_null() {
            let listener: ComPtr<IWindowListener> = value.to_owned();
            Some(Box::new(move |e| {
                use viewport::WindowEvent::*;
                let result = match e {
                    &Resized(size) => listener.resized(size),
                    &Moved(position) => listener.moved(position),
                    &Close => listener.close(),
                    &Focused(focused) => listener.focused(focused),
                    &MouseButton(position, button, pressed) => {
                        use viewport::MouseButton::*;
                        listener.mouse_button(
                            trans_mouse_pos(position),
                            match button {
                                Left => 0,
                                Right => 1,
                                Middle => 2,
                                Other(i) => i,
                            },
                            pressed,
                        )
                    }
                    &MouseMotion(Some(position)) => {
                        listener.mouse_motion(trans_mouse_pos(position))
                    }
                    &MouseMotion(None) => listener.mouse_leave(),
                    &KeyboardInput(vkc, pressed, modifiers) => {
                        let mut flags = ngsbase::KeyModifierFlags::empty();
                        if modifiers.contains(viewport::KeyModifier::Shift) {
                            flags |= ngsbase::KeyModifierFlagsItem::Shift;
                        }
                        if modifiers.contains(viewport::KeyModifier::Control) {
                            flags |= ngsbase::KeyModifierFlagsItem::Control;
                        }
                        if modifiers.contains(viewport::KeyModifier::Alt) {
                            flags |= ngsbase::KeyModifierFlagsItem::Alt;
                        }
                        if modifiers.contains(viewport::KeyModifier::Meta) {
                            flags |= ngsbase::KeyModifierFlagsItem::Meta;
                        }
                        listener.keyboard_input(
                            Some(&*BStringRef::new(&format!("{:?}", vkc))),
                            pressed,
                            flags,
                        )
                    }
                };
                assert_eq!(result, hresults::E_OK);
            }))
        } else {
            None
        };
        let ref context: core::Context = *self.data.0;
        self.data
            .1
            .with_mut(|s| match s {
                NodeDataState::Partial(builder) => {
                    let b: viewport::WindowBuilder = builder.take().unwrap();
                    *builder = Some(b.listener(value));
                    Ok(())
                }
                NodeDataState::Materialized(window) => {
                    let mut frame = context
                        .lock_producer_frame()
                        .map_err(translate_context_error)?;
                    window.listener().set(&mut frame, value).unwrap();
                    Ok(())
                }
            })
            .err()
            .unwrap_or(hresults::E_OK)
    }
}

impl INodeRefTrait for ComWindow {
    fn create_node_ref(&self) -> core::NodeRef {
        let ref context = self.data.0;
        self.data.1.with_materialized(
            |p| p.unwrap().build(context),
            |window_ref| window_ref.clone().into_node_ref(),
        )
    }
}
