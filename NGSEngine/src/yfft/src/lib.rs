//! yFFT
//! ====
//!
//! Simple FFT library written purely in Rust. Requires a Nightly Rust compiler for x86 intrinsics.
//!
//! Notes Regarding Compilation
//! ---------------------------
//!
//! As of the version 0.19.0 cargo doesn't support passing codegen flags to rustc. Because of this,
//! you need to pass the following flags via the `RUSTFLAGS` environemnt variable to enable AVX kernel:
//!
//! ```sh
//! export RUSTFLAGS='-Ctarget-feature=+avx,+sse3'
//! ```
//!
//! Note: this causes codegen to generate VEX prefixes to all SSE instructions and makes the binary
//! incompatibile with processors without AVX support.
//!
//! License
//! -------
//!
//! Follows the license of the parent project (Nightingales).

//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

#![feature(test, slice_get_slice, platform_intrinsics, cfg_target_feature)]

extern crate num_complex;
extern crate num_iter;
extern crate num_traits;
extern crate simd;

use std::ops::{AddAssign, SubAssign, MulAssign, DivAssign};
use std::fmt::Debug;

use num_complex::Complex;

#[macro_use] mod simdutils;
mod setup;
mod kernel;
mod env;

pub trait Num :
    Clone + Debug + AddAssign + SubAssign + MulAssign + DivAssign + Default +
    num_traits::Float + num_traits::FloatConst + num_traits::Zero + 'static {}
impl<T> Num for T where T :
    Clone + Debug + AddAssign + SubAssign + MulAssign + DivAssign + Default +
    num_traits::Float + num_traits::FloatConst + num_traits::Zero + 'static {}

#[inline]
fn complex_from_slice<T : Num>(x: &[T]) -> Complex<T> {
    Complex::new(x[0], x[1])
}

#[inline]
fn mul_pos_i<T : Num>(x: Complex<T>) -> Complex<T> { Complex::new(-x.im, x.re) }

pub use setup::{DataOrder, DataFormat, Options, Setup, PlanError};
pub use env::Env;

#[cfg(test)]
mod benchmark;
