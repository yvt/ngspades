//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Nightingales Presentation Framework (NgsPF)
//! ===========================================
//!
//! todo
//!
#![feature(conservative_impl_trait)]
pub extern crate ngsgfx as gfx;
pub extern crate ngsbase;
extern crate arclock;
extern crate refeq;
extern crate tokenlock;
extern crate cgmath;
extern crate winit;
extern crate ngsenumflags;
#[macro_use]
extern crate ngsenumflags_derive;
pub extern crate atomic_refcell;
extern crate iterpool;

pub mod context;
pub mod viewport;

/// The NgsPF prelude.
pub mod prelude {
    pub use context::{PropertyProducerWrite, PropertyProducerRead, PropertyPresenterRead,
                      PropertyAccessor, RoPropertyAccessor};
}
