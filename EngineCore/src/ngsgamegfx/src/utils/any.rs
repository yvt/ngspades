//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::any::Any;

/// Supports conversion to `dyn Any + Send + Sync`. This trait is automatically
/// implemented on every `impl Any + Send + Sync`.
pub trait AsAnySendSync: Any + Send + Sync {
    fn as_any(&self) -> &(dyn Any + Send + Sync);
    fn as_any_mut(&mut self) -> &mut (dyn Any + Send + Sync);
}

impl<T: Any + Send + Sync> AsAnySendSync for T {
    fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self
    }
    fn as_any_mut(&mut self) -> &mut (dyn Any + Send + Sync) {
        self
    }
}
