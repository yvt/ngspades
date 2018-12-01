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
pub use ngspf_canvas as canvas;
pub use ngspf_core as core;
pub use ngspf_viewport as viewport;

pub use cggeom;

/// The NgsPF prelude.
pub mod prelude {
    #[doc(no_inline)]
    pub use ngspf_core::prelude::*;

    #[doc(no_inline)]
    pub use ngspf_canvas::prelude::*;
}
