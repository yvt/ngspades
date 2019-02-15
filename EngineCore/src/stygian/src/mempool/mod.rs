//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Memory page management for [`crate::gen`].

mod traits;
mod sys;
pub use self::{traits::*, sys::*};
