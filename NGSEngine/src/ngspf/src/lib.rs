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
//! ## Property Accessor
//!
//! Property accessors provide an easy way to access and modify properties of
//! nodes. They record a changeset to the frame automatically when updating a
//! property value.
//!
//! See the documentation of [`KeyedPropertyAccessor`] for the usage.
//!
//! [`KeyedPropertyAccessor`]: struct.KeyedPropertyAccessor.html
#![feature(conservative_impl_trait)]
extern crate ngsgfx;
extern crate arclock;
extern crate refeq;
extern crate tokenlock;
extern crate cgmath;

mod context;
pub mod image;
pub mod layer;

pub use self::context::*;

pub mod prelude {
    pub use {PropertyProducerWrite, PropertyProducerRead, PropertyPresenterRead, PropertyAccessor,
             RoPropertyAccessor};
}
