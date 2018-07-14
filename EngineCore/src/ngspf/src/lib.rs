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
pub extern crate ngspf_canvas as canvas;
pub extern crate ngspf_core as core;
pub extern crate ngspf_viewport as viewport;

pub extern crate cggeom;

/// The NgsPF prelude.
pub mod prelude {
    #[doc(no_inline)]
    pub use core::prelude::*;

    #[doc(no_inline)]
    pub use canvas::prelude::*;
}
