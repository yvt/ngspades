//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! SIMD wrapper similar to [`faster`], but with more explicit developer control.
//!
//! [`faster`]: https://docs.adamniederer.com/faster/index.html
#![feature(cfg_target_feature)]
extern crate num_traits;
extern crate stdsimd;

#[macro_use]
mod packed;
pub use packed::*;

pub mod kernel;
pub mod simd16;
pub mod utils;

pub mod prelude {
    #[doc(no_inline)]
    pub use {kernel::*, utils::*, IntPacked, Packed, PackedI16, PackedU16, PackedU32, PackedU8,
             SimdMode};
}
