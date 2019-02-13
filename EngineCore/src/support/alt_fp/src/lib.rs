//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! This crate provides an alternative (faster in most cases) implementation for
//! floating-point operations.
pub mod cast;
pub mod cmp;
pub mod fma;

#[doc(no_inline)]
pub use self::{cast::*, cmp::*, fma::*};
