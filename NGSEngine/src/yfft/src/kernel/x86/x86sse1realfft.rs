//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::{Kernel, KernelParams, SliceAccessor};
use super::utils::{if_compatible};

use simd::f32x4;

use {mul_pos_i, Num, complex_from_slice, Complex};

/// Creates a real FFT post-processing or backward real FFT pre-processing kernel.
pub fn new_x86_sse_real_fft_pre_post_process_kernel<T>(len: usize, inverse: bool) -> Option<Box<Kernel<T>>>
    where T : Num
{
    None
}

pub(super) fn new_real_fft_coef_table(len: usize, inverse: bool) -> Vec<f32x4> {
    unimplemented!()
}

#[derive(Debug)]
struct SseRealFFTPrePostProcessKernel {
    len: usize,
    table: Vec<f32x4>,
    inverse: bool,
}

impl SseRealFFTPrePostProcessKernel {
    fn new(len: usize, inverse: bool) -> Self {
        Self {
            len,
            table: new_real_fft_coef_table(len, inverse),
            inverse,
        }
    }
}

impl Kernel<f32> for SseRealFFTPrePostProcessKernel {
    fn transform(&self, params: &mut KernelParams<f32>) {
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0 .. self.len]) };
        unimplemented!()
        /*let table = unsafe { SliceAccessor::new(&self.table[..]) };
        let len_2 = self.len / 2;
        if !self.inverse {
            // A(0) = (1 - j) / 2, B(0) = (1 + j) / 2
            // A(k) = (1 + j) / 2, B(k) = (1 - j) / 2
            // x A(0) + conj(x) B(0) = Re(x) + Im(x)
            // x A(k) + conj(x) B(k) = Re(x) - Im(x)
            // Store G(0) as X_r(0)
            // Store G(N/2) as X_i(0)
            // data[1] = data[0] - data[1];
            let (x1, x2) = (data[0], data[1]);
            data[0] = x1 + x2;
            data[1] = x1 - x2;
        } else {
            // A(0) = (1 + j) / 2, B(0) = (1 - j) / 2
            // A(k) = (1 - j) / 2, B(k) = (1 + j) / 2
            // Re(x) A(0) + Im(x) B(0) = (Re(x) + Im(x)) / 2 + j(Re(x) - Im(x)) /2
            let (x1, x2) = (data[0], data[1]);
            data[0] = (x1 + x2) * T::from(0.5).unwrap();
            data[1] = (x1 - x2) * T::from(0.5).unwrap();
        }
        for i in 1..len_2 {
            let a1 = complex_from_slice(&table[i * 4..]);
            let b1 = complex_from_slice(&table[i * 4 + 2..]);
            let a2 = complex_from_slice(&table[(len_2 - i) * 4..]);
            let b2 = complex_from_slice(&table[(len_2 - i) * 4 + 2..]);
            let x1 = complex_from_slice(&data[i * 2..]);
            let x2 = complex_from_slice(&data[(len_2 - i) * 2..]);
            let g1 = x1 * a1 + x2.conj() * b1;
            let g2 = x2 * a2 + x1.conj() * b2;
            data[i * 2] = g1.re;
            data[i * 2 + 1] = g1.im;
            data[(len_2 - i) * 2] = g2.re;
            data[(len_2 - i) * 2 + 1] = g2.im;
        } */
    }
}
