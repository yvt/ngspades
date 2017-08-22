//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate ysr2_common;
extern crate ysr2_kemar_data;
#[macro_use]
extern crate lazy_static;
extern crate cgmath;
extern crate yfft;

use std::{fmt, hash};

use ysr2_common::stream::Generator;
use ysr2_common::values::DynamicSlerpVector3;

pub mod equalpower;
pub mod hrtf;

pub trait Panner<T: Generator>: Generator {
    type SourceId: fmt::Debug + Eq + PartialEq + hash::Hash + Clone;

    fn insert(&mut self, generator: T) -> Self::SourceId;

    fn generator(&self, id: &Self::SourceId) -> Option<&T>;
    fn generator_mut(&mut self, id: &Self::SourceId) -> Option<&mut T>;
    fn direction(&self, id: &Self::SourceId) -> Option<&DynamicSlerpVector3>;
    fn direction_mut(&mut self, id: &Self::SourceId) -> Option<&mut DynamicSlerpVector3>;

    fn remove(&mut self, id: &Self::SourceId) -> Option<T>;

    // TODO: add method to update the listener orientation?
}
