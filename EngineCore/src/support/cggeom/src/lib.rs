//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! A helper library for `cgmath`.
//!
//! Provides additional types useful in computer graphics.
extern crate cgmath;

mod boxes;
mod elementwise;

pub use self::boxes::*;
pub use self::elementwise::*;

/// The prelude.
pub mod prelude {
    #[doc(no_inline)]
    pub use {AxisAlignedBox, ElementWiseOp, ElementWisePartialOrd};
}
