//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cmd::fence::Fence;

#[derive(Debug, Default)]
pub struct CmdBufferFenceSet {
    pub wait_fences: Vec<Fence>,
    pub signal_fences: Vec<Fence>,
}

impl CmdBufferFenceSet {
    pub fn new() -> Self {
        Default::default()
    }
}

