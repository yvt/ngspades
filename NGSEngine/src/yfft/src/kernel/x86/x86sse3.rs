//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

//! Defines FFT kernels optimized by using SSE3 instruction set.
//!
//! Performances
//! ------------
//!
//! Yet to be measured.

use super::{Kernel, KernelCreationParams, Num};

pub fn new_x86_sse3_kernel<T>(_: &KernelCreationParams) -> Option<Box<Kernel<T>>>
    where T : Num {

    None
}
