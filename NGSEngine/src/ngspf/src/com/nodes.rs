//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {ngsbase, cgmath};
use ngscom::{ComPtr, UnownedComPtr, HResult, hresults, IUnknown, BString};
use com::{INodeGroup, IWindow, ILayer, IWindowListener};

com_impl! {
    class NodeGroup {
        com_private: NodeGroupPrivate;
        inode_group: (INodeGroup, ngsbase::INodeGroupVtbl, NODE_GROUP_VTBL);
        // add custom fields here
    }
}

impl NodeGroup {
    pub fn new() -> ComPtr<INodeGroup> {
        ComPtr::from(
            &NodeGroup::alloc(NodeGroup { com_private: Self::new_private() }).0,
        )
    }
}

impl ngsbase::INodeGroupTrait for NodeGroup {
    fn insert(&self, _node: UnownedComPtr<IUnknown>) -> HResult {
        hresults::E_NOTIMPL
    }
}

com_impl! {
    class Layer {
        com_private: LayerPrivate;
        ilayer: (ILayer, ngsbase::ILayerVtbl, LAYER_VTBL);
        // add custom fields here
    }
}

impl Layer {
    pub fn new() -> ComPtr<ILayer> {
        ComPtr::from(&Layer::alloc(Layer { com_private: Self::new_private() }).0)
    }
}

impl ngsbase::ILayerTrait for Layer {
    fn set_opacity(&self, _value: f32) -> HResult {
        hresults::E_NOTIMPL
    }

    fn set_bounds(&self, _value: ngsbase::Box2<f32>) -> HResult {
        hresults::E_NOTIMPL
    }

    fn set_child(&self, _value: UnownedComPtr<IUnknown>) -> HResult {
        hresults::E_NOTIMPL
    }

    fn set_mask(&self, _value: UnownedComPtr<IUnknown>) -> HResult {
        hresults::E_NOTIMPL
    }
}

com_impl! {
    class Window {
        com_private: WindowPrivate;
        iwindow: (IWindow, ngsbase::IWindowVtbl, WINDOW_VTBL);
        // add custom fields here
    }
}

impl Window {
    pub fn new() -> ComPtr<IWindow> {
        ComPtr::from(
            &Window::alloc(Window { com_private: Self::new_private() }).0,
        )
    }
}

impl ngsbase::IWindowTrait for Window {
    fn set_flags(&self, _value: ngsbase::WindowFlags) -> HResult {
        hresults::E_NOTIMPL
    }

    fn set_size(&self, _value: cgmath::Vector2<f32>) -> HResult {
        hresults::E_NOTIMPL
    }

    fn set_child(&self, _value: UnownedComPtr<IUnknown>) -> HResult {
        hresults::E_NOTIMPL
    }

    fn set_title(&self, _value: Option<&BString>) -> HResult {
        hresults::E_NOTIMPL
    }

    fn set_listener(&self, _value: UnownedComPtr<IWindowListener>) -> HResult {
        hresults::E_NOTIMPL
    }
}
