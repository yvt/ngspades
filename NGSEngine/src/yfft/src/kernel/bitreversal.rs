//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::{Kernel, KernelParams, SliceAccessor};

use super::super::Num;

/// Creates a bit reversal kernel.
///
/// A bit reversal kernel is, as its name implies, a special kernel that performs
/// the bit reversal operation. The kernel is supposed to be executed after DIF FFT
/// steps.
pub fn new_bit_reversal_kernel<T>(radixes: &[usize]) -> Box<Kernel<T>>
    where T : Num {

    let len = radixes.iter().product();
    let mut indices = vec![0; len];

    let mut digits = vec![0; radixes.len()];
    let mut factors = vec![0; radixes.len()];
    factors[0] = 1;
    for i in 0 .. radixes.len() - 1 {
        factors[i + 1] = factors[i] * radixes[i];
    }

    let mut cur: usize = 0;
    for i in 0 .. len {
        indices[cur] = i;
        if i < len - 1 {
            digits[radixes.len() - 1] += 1; cur += factors[radixes.len() - 1];
            for k in (0 .. radixes.len()).rev() {
                if digits[k] < radixes[k] {
                    break;
                }
                digits[k - 1] += 1;
                digits[k] = 0;
                cur -= factors[k] * radixes[k];
                cur += factors[k - 1];
            }
        }
    }

    Box::new(BitReversalKernel {
        indices: indices
    })
}

#[derive(Debug)]
struct BitReversalKernel {
    indices: Vec<usize>
}

impl<T> Kernel<T> for BitReversalKernel where T : Num {
    fn transform(&self, params: &mut KernelParams<T>) {
        let indices = SliceAccessor::new(&self.indices);
        let size = self.indices.len();
        let mut data = SliceAccessor::new(&mut params.coefs[0 .. size * 2]);
        let mut wa = SliceAccessor::new(&mut params.work_area[0 .. size * 2]);
        wa.copy_from_slice(*data);
        for i in 0 .. size {
            let index = indices[i];
            data[i * 2] = wa[index * 2];
            data[i * 2 + 1] = wa[index * 2 + 1];
        }
    }
    fn required_work_area_size(&self) -> usize {
        self.indices.len() * 2
    }
}