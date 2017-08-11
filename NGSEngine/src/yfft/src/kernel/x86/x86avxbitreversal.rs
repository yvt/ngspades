//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::{Kernel, KernelParams, SliceAccessor};
use super::utils::if_compatible;
use super::super::Num;

use simd::x86::avx::u64x4;

use std::mem;

pub fn new_x86_avx_bit_reversal_kernel<T>(indices: &Vec<usize>) -> Option<Box<Kernel<T>>>
where
    T: Num,
{
    if indices.len() < 8 {
        // doesn't benefit much
        return None;
    }

    if_compatible(|| {
        Some(Box::new(
            AvxDWordBitReversalKernel { indices: indices.clone() },
        ) as Box<Kernel<f32>>)
    })
}

#[derive(Debug)]
struct AvxDWordBitReversalKernel {
    indices: Vec<usize>,
}

impl<T: Num> Kernel<T> for AvxDWordBitReversalKernel {
    fn transform(&self, params: &mut KernelParams<T>) {
        assert_eq!(mem::size_of::<T>(), 4);

        // TODO: check alignment

        let indices = unsafe { SliceAccessor::new(&self.indices) };
        let size = self.indices.len();
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0..size * 2]) };
        let mut wa = unsafe { SliceAccessor::new(&mut params.work_area[0..size * 2]) };
        wa.copy_from_slice(*data);
        let mut i = 0;
        while i + 7 < size {
            let index1 = indices[i];
            let index2 = indices[i + 1];
            let index3 = indices[i + 2];
            let index4 = indices[i + 3];
            let index5 = indices[i + 4];
            let index6 = indices[i + 5];
            let index7 = indices[i + 6];
            let index8 = indices[i + 7];

            let src1: *const u64 = &wa[index1 * 2] as *const T as *const u64;
            let src2: *const u64 = &wa[index2 * 2] as *const T as *const u64;
            let src3: *const u64 = &wa[index3 * 2] as *const T as *const u64;
            let src4: *const u64 = &wa[index4 * 2] as *const T as *const u64;
            let src5: *const u64 = &wa[index5 * 2] as *const T as *const u64;
            let src6: *const u64 = &wa[index6 * 2] as *const T as *const u64;
            let src7: *const u64 = &wa[index7 * 2] as *const T as *const u64;
            let src8: *const u64 = &wa[index8 * 2] as *const T as *const u64;
            let dest: *mut u64x4 = &mut data[i * 2] as *mut T as *mut u64x4;
            unsafe {
                *dest = u64x4::new(*src1, *src2, *src3, *src4);
                *dest.offset(1) = u64x4::new(*src5, *src6, *src7, *src8);
            }

            i += 8;
        }
        while i < size {
            let index = indices[i];

            let src: *const u64 = &wa[index * 2] as *const T as *const u64;
            let dest: *mut u64 = &mut data[i * 2] as *mut T as *mut u64;
            unsafe {
                *dest = *src;
            }

            i += 1;
        }
    }
    fn required_work_area_size(&self) -> usize {
        self.indices.len() * 2
    }
}
