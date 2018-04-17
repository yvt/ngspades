//! Nightingales Base / Interop
//! =============================
//!
//! This crate includes basic data types and definitions of COM interfaces.

//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

#[macro_use]
extern crate ngscom;
extern crate cgmath;
extern crate ngsenumflags;
extern crate num_traits;
#[macro_use]
extern crate ngsenumflags_derive;

mod geom;
mod interop;

pub use geom::*;
pub use interop::*;

/// The NgsBase prelude.
pub mod prelude {
    #[doc(no_inline)]
    pub use geom::{AxisAlignedBox, ElementWiseOp, ElementWisePartialOrd};
}
