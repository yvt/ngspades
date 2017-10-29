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
        inode_group: (INodeGroup, ngsbase::INodeGroupVtbl);
        data: ();
    }
}

impl NodeGroup {
    pub fn new() -> ComPtr<INodeGroup> {
        ComPtr::from(&NodeGroup::alloc(()))
    }
}

impl ngsbase::INodeGroupTrait for NodeGroup {
    fn insert(&self, _node: UnownedComPtr<IUnknown>) -> HResult {
        hresults::E_NOTIMPL
    }
}

com_impl! {
    class Layer {
        ilayer: (ILayer, ngsbase::ILayerVtbl);
        data: ();
    }
}

impl Layer {
    pub fn new() -> ComPtr<ILayer> {
        ComPtr::from(&Layer::alloc(()))
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
        iwindow: (IWindow, ngsbase::IWindowVtbl);
        data: ();
    }
}

impl Window {
    pub fn new() -> ComPtr<IWindow> {
        ComPtr::from(&Window::alloc(()))
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
