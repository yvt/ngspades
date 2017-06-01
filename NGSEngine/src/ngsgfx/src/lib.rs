//! NgsGFX − Nightingales Graphics Backend
//! ======================================
//!
//! Abstracts the interface to the graphics APIs (e.g., Vulkan, Metal).
//!
//! See the [`ngsgfx_core`][core] crate's documentation for the usage.
//!
//! [core]: ../ngsgfx_core/index.html

extern crate ngsgfx_core;
extern crate ngsgfx_metal;

pub use ::ngsgfx_core::*;

pub mod backends {
    pub mod metal {
        pub use ::ngsgfx_metal::*;
    }
}