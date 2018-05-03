//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ngscom::{hresults, to_hresult, BString, BStringRef, ComPtr, HResult, IAny, IUnknown,
             IUnknownTrait, UnownedComPtr};
use tokenlock::TokenLock;
use {cggeom, cgmath, ngsbase, rgb};

use core::prelude::*;
use hresults::{E_PF_LOCKED, E_PF_NODE_MATERIALIZED, E_PF_NOT_IMAGE, E_PF_NOT_NODE};
use {core, viewport};
use {ComContext, ComImage, INgsPFLayer, INgsPFNodeGroup, INgsPFWindow, INgsPFWindowListener};

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

        fn create_node_ref() -> Result<core::NodeRef, HResult>;
    }
}

/// Thread-safe node data that can be either in the partial state (being
/// constructed) or in the materialized state.
///
/// The contained state is protected by a producer lock.
#[derive(Debug)]
struct NodeData<P, M> {
    context: ComPtr<IAny>,
    cell: TokenLock<NodeDataInner<P, M>>,
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
    fn new(context: ComPtr<IAny>, x: P) -> Self {
        let cell;
        {
            let context_ref: &ComContext = context.downcast_ref().unwrap();
            let token_ref = context_ref.token_ref();
            cell = TokenLock::new(token_ref.clone(), NodeDataInner::Partial(x));
        }
        Self { cell, context }
    }

    fn com_context(&self) -> &ComContext {
        self.context.downcast_ref().unwrap()
    }

    fn context(&self) -> &core::Context {
        self.com_context().context()
    }

    /// Transition a `NodeData` to the materialized state using the first
    /// supplied function. Call the second function on its materialized state
    /// data.
    fn with_materialized<
        C: FnOnce(P) -> M,
        F: FnOnce(&M, &mut core::ProducerFrame) -> Result<R, HResult>,
        R,
    >(
        &self,
        ctor: C,
        f: F,
    ) -> Result<R, HResult> {
        match self.with(move |state, frame| {
            if let Some(m) = state.materialized() {
                Ok(Ok(f(m, frame)?))
            } else {
                Ok(Err(f))
            }
        }) {
            Ok(Ok(x)) => Ok(x),
            Ok(Err(f)) => {
                self.materialize(ctor)?;
                self.with(move |state, frame| f(state.materialized().unwrap(), frame))
            }
            Err(e) => Err(e),
        }
    }

    /// Transition a `NodeData` to the materialized state using a supplied
    /// function that accepts the partial state data as its input.
    fn materialize<C: FnOnce(P) -> M>(&self, ctor: C) -> Result<(), HResult> {
        use std::mem::replace;

        let mut lock = self.com_context().lock_producer_frame()?;

        let state = self.cell.write(lock.token_mut()).unwrap();
        let new_state = match replace(state, NodeDataInner::Invalid) {
            NodeDataInner::Materialized(m) => NodeDataInner::Materialized(m),
            NodeDataInner::Partial(p) => NodeDataInner::Materialized(ctor(p)),
            _ => panic!("NodeData is poisoned"),
        };
        *state = new_state;

        Ok(())
    }

    /// Acquire a producer lock and call a supplied function with its
    /// partial or materialized state data.
    fn with<F: FnOnce(NodeDataState<&P, &M>, &mut core::ProducerFrame) -> Result<R, HResult>, R>(
        &self,
        f: F,
    ) -> Result<R, HResult> {
        let mut lock = self.com_context().lock_producer_frame()?;
        let producer_tokens = lock.get_mut();
        let producer_frame = producer_tokens.frame;

        let state = self.cell.read(producer_tokens.token).unwrap();
        match *state {
            NodeDataInner::Materialized(ref m) => f(NodeDataState::Materialized(m), producer_frame),
            NodeDataInner::Partial(ref p) => f(NodeDataState::Partial(p), producer_frame),
            _ => panic!("NodeData is poisoned"),
        }
    }

    /// Acquire a producer lock and call a supplied function with a
    /// mutable reference to its partial or materialized state data.
    fn with_mut<
        F: FnOnce(NodeDataState<&mut P, &mut M>, &mut core::ProducerFrame) -> Result<R, HResult>,
        R,
    >(
        &self,
        f: F,
    ) -> Result<R, HResult> {
        let mut lock = self.com_context().lock_producer_frame()?;
        let producer_tokens = lock.get_mut();
        let producer_frame = producer_tokens.frame;

        let state = self.cell.write(producer_tokens.token).unwrap();
        match *state {
            NodeDataInner::Materialized(ref mut m) => {
                f(NodeDataState::Materialized(m), producer_frame)
            }
            NodeDataInner::Partial(ref mut p) => f(NodeDataState::Partial(p), producer_frame),
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

/// Set the property in a way depending on the state of `NodeData`.
///
///  - If `$node_data` is in the partial state, pass the value to a builder.
///  - If `$node_data` is in the materialized state, set the property on the
///    node.
macro_rules! node_data_set_prop {
    ($node_data:expr, $name:ident = $value:expr) => {{
        let value = $value;
        $node_data.with_mut(|s, frame| match s {
            NodeDataState::Partial(builder) => {
                // Since they are consuming type of builders, we must first
                // `take` it
                let b = builder.take().unwrap();
                *builder = Some(b.$name(value));
                Ok(())
            }
            NodeDataState::Materialized(obj_ref) => {
                // `obj_ref`: `LayerRef`, etc.
                obj_ref.$name().set(frame, value).unwrap();
                Ok(())
            }
        })
    }};
}

/// Set the property in a way depending on the state of `NodeData`.
///
///  - If `$node_data` is in the partial state, pass the value to a builder.
///  - If `$node_data` is in the materialized state, return `Err(E_PF_NODE_MATERIALIZED)`.
macro_rules! node_data_set_prop_builder_only {
    ($node_data:expr, $name:ident = $value:expr) => {{
        let value = $value;
        $node_data.with_mut(|s, _| match s {
            NodeDataState::Partial(builder) => {
                // Since they are consuming type of builders, we must first
                // `take` it
                let b = builder.take().unwrap();
                *builder = Some(b.$name(value));
                Ok(())
            }
            NodeDataState::Materialized(_) => Err(E_PF_NODE_MATERIALIZED),
        })
    }};
}

com_impl! {
    #[derive(Debug)]
    class ComNodeGroup {
        inode_group: INgsPFNodeGroup;
        inoderef: INodeRef;
        @data: NodeData<Vec<core::NodeRef>, core::GroupRef>;
    }
}

impl ComNodeGroup {
    pub fn new(context: ComPtr<IAny>) -> ComPtr<INgsPFNodeGroup> {
        (&ComNodeGroup::alloc(NodeData::new(context, Vec::new()))).into()
    }
}

impl ngsbase::INgsPFNodeGroupTrait for ComNodeGroup {
    fn insert(&self, node: UnownedComPtr<IUnknown>) -> HResult {
        to_hresult(|| {
            self.data.with_mut(|s, _| match s {
                NodeDataState::Materialized(_) => Err(E_PF_NODE_MATERIALIZED),
                NodeDataState::Partial(p) => {
                    let inoderef: ComPtr<INodeRef> = (&*node).into();
                    if inoderef.is_null() {
                        return Err(E_PF_NOT_NODE);
                    }

                    p.push(inoderef.create_node_ref()?);
                    Ok(())
                }
            })
        })
    }
}

impl INodeRefTrait for ComNodeGroup {
    fn create_node_ref(&self) -> Result<core::NodeRef, HResult> {
        self.data.with_materialized(
            |p| core::GroupRef::new(p),
            |group_ref, _| Ok(group_ref.clone().into_node_ref()),
        )
    }
}

com_impl! {
    class ComLayer {
        ilayer: INgsPFLayer;
        inoderef: INodeRef;
        @data: NodeData<Option<viewport::LayerBuilder>, viewport::LayerRef>;
    }
}

impl ComLayer {
    pub fn new(context: ComPtr<IAny>) -> ComPtr<INgsPFLayer> {
        (&ComLayer::alloc(NodeData::new(context, Some(viewport::LayerBuilder::new())))).into()
    }
}

impl ngsbase::INgsPFLayerTrait for ComLayer {
    fn set_opacity(&self, value: f32) -> HResult {
        to_hresult(|| node_data_set_prop!(self.data, opacity = value))
    }

    fn set_transform(&self, value: cgmath::Matrix4<f32>) -> HResult {
        to_hresult(|| node_data_set_prop!(self.data, transform = value))
    }

    fn set_flags(&self, flags: ngsbase::LayerFlags) -> HResult {
        let mut value = viewport::LayerFlags::empty();
        if flags.contains(ngsbase::LayerFlagsItem::FlattenContents) {
            value |= viewport::LayerFlagsBit::FlattenContents;
        }

        to_hresult(|| node_data_set_prop!(self.data, flags = value))
    }

    fn set_bounds(&self, value: cggeom::Box2<f32>) -> HResult {
        to_hresult(|| node_data_set_prop!(self.data, bounds = value))
    }

    fn set_child(&self, value: UnownedComPtr<IUnknown>) -> HResult {
        to_hresult(|| {
            let value = if value.is_null() {
                None
            } else {
                let inoderef: ComPtr<INodeRef> = (&*value).into();
                if inoderef.is_null() {
                    return Err(E_PF_NOT_NODE);
                }
                Some(inoderef.create_node_ref()?)
            };
            node_data_set_prop!(self.data, child = value)
        })
    }

    fn set_mask(&self, value: UnownedComPtr<IUnknown>) -> HResult {
        to_hresult(|| {
            let value = if value.is_null() {
                None
            } else {
                let inoderef: ComPtr<INodeRef> = (&*value).into();
                if inoderef.is_null() {
                    return Err(E_PF_NOT_NODE);
                }
                Some(inoderef.create_node_ref()?)
            };

            node_data_set_prop!(self.data, mask = value)
        })
    }

    fn set_contents_back_drop(&self) -> HResult {
        let value = viewport::LayerContents::BackDrop;
        to_hresult(|| node_data_set_prop!(self.data, contents = value))
    }

    fn set_contents_empty(&self) -> HResult {
        let value = viewport::LayerContents::Empty;
        to_hresult(|| node_data_set_prop!(self.data, contents = value))
    }

    fn set_contents_image(
        &self,
        image: UnownedComPtr<IUnknown>,
        source: cggeom::Box2<f32>,
        wrap_mode: ngsbase::ImageWrapMode,
    ) -> HResult {
        to_hresult(|| {
            if image.is_null() {
                return Err(hresults::E_POINTER);
            }
            let image: ComPtr<IAny> = (&*image).into();
            if image.is_null() {
                return Err(E_PF_NOT_IMAGE);
            }
            let image: &ComImage = image.downcast_ref().ok_or(E_PF_NOT_IMAGE)?;
            let image = image.image_ref().clone();

            let wrap_mode = match wrap_mode.get().ok_or(hresults::E_INVALIDARG)? {
                ngsbase::ImageWrapModeItem::Repeat => viewport::ImageWrapMode::Repeat,
                ngsbase::ImageWrapModeItem::Clamp => viewport::ImageWrapMode::Clamp,
            };

            let value = viewport::LayerContents::Image {
                image,
                source,
                wrap_mode,
            };

            node_data_set_prop!(self.data, contents = value)
        })
    }

    fn set_contents_port(&self, port: UnownedComPtr<IUnknown>) -> HResult {
        to_hresult(|| {
            if port.is_null() {
                return Err(hresults::E_POINTER);
            }
            // We don't have an interface for getting a port yet
            unimplemented!()
        })
    }

    fn set_contents_solid_color(&self, value: rgb::RGBA<f32>) -> HResult {
        let value = viewport::LayerContents::Solid(value);
        to_hresult(|| node_data_set_prop!(self.data, contents = value))
    }
}

impl INodeRefTrait for ComLayer {
    fn create_node_ref(&self) -> Result<core::NodeRef, HResult> {
        self.data.with_materialized(
            |p| p.unwrap().build(self.data.context()),
            |layer_ref, _| Ok(layer_ref.clone().into_node_ref()),
        )
    }
}

com_impl! {
    class ComWindow {
        iwindow: INgsPFWindow;
        inoderef: INodeRef;
        @data: NodeData<Option<viewport::WindowBuilder>, viewport::WindowRef>;
    }
}

impl ComWindow {
    pub fn new(context: ComPtr<IAny>) -> ComPtr<INgsPFWindow> {
        (&ComWindow::alloc(NodeData::new(context, Some(viewport::WindowBuilder::new())))).into()
    }
}

impl ngsbase::INgsPFWindowTrait for ComWindow {
    fn set_flags(&self, flags: ngsbase::WindowFlags) -> HResult {
        to_hresult(|| {
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

            node_data_set_prop_builder_only!(self.data, flags = value)
        })
    }

    fn set_size(&self, value: cgmath::Vector2<f32>) -> HResult {
        to_hresult(|| node_data_set_prop!(self.data, size = value))
    }

    fn set_child(&self, value: UnownedComPtr<IUnknown>) -> HResult {
        to_hresult(|| {
            let value = if value.is_null() {
                None
            } else {
                let inoderef: ComPtr<INodeRef> = (&*value).into();
                if inoderef.is_null() {
                    return Err(E_PF_NOT_NODE);
                }
                Some(inoderef.create_node_ref()?)
            };

            node_data_set_prop!(self.data, child = value)
        })
    }

    fn set_title(&self, value: Option<&BString>) -> HResult {
        let value = if let Some(value) = value {
            value.as_str().to_owned()
        } else {
            String::new()
        };

        to_hresult(|| node_data_set_prop!(self.data, title = value))
    }

    fn set_listener(&self, value: UnownedComPtr<INgsPFWindowListener>) -> HResult {
        fn trans_mouse_pos(p: viewport::MousePosition) -> ngsbase::MousePosition {
            ngsbase::MousePosition {
                client: p.client,
                global: p.global,
            }
        }
        let value: Option<viewport::WindowListener> = if !value.is_null() {
            let listener: ComPtr<INgsPFWindowListener> = value.to_owned();
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

        to_hresult(|| node_data_set_prop!(self.data, listener = value))
    }
}

impl INodeRefTrait for ComWindow {
    fn create_node_ref(&self) -> Result<core::NodeRef, HResult> {
        self.data.with_materialized(
            |p| p.unwrap().build(self.data.context()),
            |window_ref, _| Ok(window_ref.clone().into_node_ref()),
        )
    }
}
