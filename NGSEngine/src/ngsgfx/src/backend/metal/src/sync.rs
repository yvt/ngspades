//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::Arc;

use core;

/// `Semaphore` implementation for Metal.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Semaphore {
    obj: Arc<()>,
}

impl Semaphore {
    pub(crate) fn make() -> Self {
        Self {
            obj: Arc::new(())
        }
    }
}

impl core::Semaphore for Semaphore {}

/// `Fence` implementation for Metal.
pub struct Fence {
}

impl Fence {
    pub(crate) fn make() -> Self {
        unimplemented!()
    }
}
