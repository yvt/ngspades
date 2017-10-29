//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::Arc;

use ngsbase;
use ngscom::{ComPtr, HResult, hresults};

use com::{IPresentationContext, INodeGroup, IWindow, ILayer};
use com::nodes;
use context::Context;

com_impl! {
    #[derive(Debug)]
    class ComContext {
        ipresentation_context: IPresentationContext;
        @data: ContextData;
    }
}

#[derive(Debug)]
struct ContextData {
    context: Arc<Context>,
}

impl ComContext {
    pub fn new(context: Arc<Context>) -> ComPtr<IPresentationContext> {
        ComPtr::from(&Self::alloc(ContextData { context }))
    }
}

impl ngsbase::IPresentationContextTrait for ComContext {
    fn create_node_group(&self, retval: &mut ComPtr<INodeGroup>) -> HResult {
        *retval = nodes::ComNodeGroup::new();
        hresults::E_OK
    }

    fn create_window(&self, retval: &mut ComPtr<IWindow>) -> HResult {
        *retval = nodes::ComWindow::new(Arc::clone(&self.data.context));
        hresults::E_OK
    }

    fn create_layer(&self, retval: &mut ComPtr<ILayer>) -> HResult {
        *retval = nodes::ComLayer::new(Arc::clone(&self.data.context));
        hresults::E_OK
    }
}
