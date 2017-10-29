//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate ngscom;
extern crate ngspf;

use std::sync::Arc;
use ngscom::{ComPtr, UnownedComPtr};
use ngscom::hresults::E_OK;
use ngspf::{com, context};

#[test]
fn create_context() {
    let pc: ComPtr<com::IPresentationContext> =
        com::ComContext::new(Arc::new(context::Context::new()));
    assert!(!pc.is_null());
}

#[test]
fn node_group() {
    let pc: ComPtr<com::IPresentationContext> =
        com::ComContext::new(Arc::new(context::Context::new()));
    let create_group = || {
        let mut g: ComPtr<com::INodeGroup> = ComPtr::null();
        assert_eq!(pc.create_node_group(&mut g), E_OK);
        assert!(!g.is_null());
        g
    };

    let g1 = create_group();
    assert_eq!(
        g1.insert(UnownedComPtr::from_comptr(&ComPtr::from(&create_group()))),
        E_OK
    );
    assert_eq!(
        g1.insert(UnownedComPtr::from_comptr(&ComPtr::from(&create_group()))),
        E_OK
    );

    let g2 = create_group();
    assert_eq!(g2.insert(UnownedComPtr::from_comptr(&ComPtr::from(&g1))), E_OK);

    assert_eq!(
        g1.insert(UnownedComPtr::from_comptr(&ComPtr::from(&create_group()))),
        com::hresults::E_PF_NODE_MATERIALIZED
    );
}