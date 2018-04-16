//! Nightingales Base / Interop
//! =============================
//!
//! This crate includes basic data types and definitions of COM interfaces.

//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

#[macro_use] extern crate ngscom;
extern crate cgmath;
extern crate num_traits;
extern crate ngsenumflags;
#[macro_use]
extern crate ngsenumflags_derive;

mod interop;
mod geom;

pub use interop::*;
pub use geom::*;

/// The NgsBase prelude.
pub mod prelude {
    #[doc(no_inline)]
    pub use geom::{AxisAlignedBox, ElementWiseOp, ElementWisePartialOrd};
}
