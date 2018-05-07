//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Raduga (радуга; rainbow) is a SIMD wrapper similar to [`faster`], but with
//! more explicit developer control.
//!
//! <a href="https://derpibooru.org/1155840">![Радуга Дэш](https://derpicdn.net/img/2016/5/17/1155840/large.png)</a>
//!
//! [`faster`]: https://docs.adamniederer.com/faster/index.html
#![feature(stdsimd)]
extern crate num_traits;

#[allow(dead_code)]
mod intrin;

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
