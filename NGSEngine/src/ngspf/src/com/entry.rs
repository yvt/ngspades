//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ngsbase;
use ngscom::{ComPtr, HResult, hresults};
use com::{IPresentationFramework, INodeGroup, IWindow, ILayer};
use com::nodes;

lazy_static! {
    static ref ENTRY: ComPtr<IPresentationFramework> = Entry::new();
}

/// Retrive an object implementing the entry interface to NgsPF.
pub fn get_entry() -> &'static ComPtr<IPresentationFramework> {
    &*ENTRY
}

com_impl! {
    class Entry {
        ipresentation_framework: (IPresentationFramework, ngsbase::IPresentationFrameworkVtbl);
        data: ();
    }
}

impl Entry {
    fn new() -> ComPtr<IPresentationFramework> {
        ComPtr::from(&Self::alloc(()))
    }
}

impl ngsbase::IPresentationFrameworkTrait for Entry {
    fn create_node_group(&self, retval: &mut ComPtr<INodeGroup>) -> HResult {
        *retval = nodes::NodeGroup::new();
        hresults::E_OK
    }

    fn create_window(&self, retval: &mut ComPtr<IWindow>) -> HResult {
        *retval = nodes::Window::new();
        hresults::E_OK
    }

    fn create_layer(&self, retval: &mut ComPtr<ILayer>) -> HResult {
        *retval = nodes::Layer::new();
        hresults::E_OK
    }
}
