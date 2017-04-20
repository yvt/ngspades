//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::{Kernel, KernelParams, SliceAccessor};
use super::super::Num;

use simd::x86::sse2::u64x2;

use std::any::TypeId;
use std::mem;

pub fn new_x86_bit_reversal_kernel<T>(indices: &Vec<usize>) -> Option<Box<Kernel<T>>>
    where T : Num {

    if TypeId::of::<T>() == TypeId::of::<f32>() {
        let kern: Option<Box<Kernel<f32>>> = Some(Box::new(SseDWordBitReversalKernel{indices: indices.clone()}));

        unsafe { mem::transmute(kern) }
    } else {
        None
    }
}

#[derive(Debug)]
struct SseDWordBitReversalKernel {
    indices: Vec<usize>
}

impl<T: Num> Kernel<T> for SseDWordBitReversalKernel {
    fn transform(&self, params: &mut KernelParams<T>) {
        assert_eq!(mem::size_of::<T>(), 4);

        // TODO: check alignment

        let indices = unsafe { SliceAccessor::new(&self.indices) };
        let size = self.indices.len();
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0 .. size * 2]) };
        let mut wa = unsafe { SliceAccessor::new(&mut params.work_area[0 .. size * 2]) };
        wa.copy_from_slice(*data);
        let mut i = 0;
        while i + 3 < size {
            let index1 = indices[i];
            let index2 = indices[i + 1];
            let index3 = indices[i + 2];
            let index4 = indices[i + 3];

            let src1: *const u64 = &wa[index1 * 2] as *const T as *const u64;
            let src2: *const u64 = &wa[index2 * 2] as *const T as *const u64;
            let src3: *const u64 = &wa[index3 * 2] as *const T as *const u64;
            let src4: *const u64 = &wa[index4 * 2] as *const T as *const u64;
            let dest: *mut u64x2 = &mut data[i * 2] as *mut T as *mut u64x2;
            unsafe { *dest = u64x2::new(*src1, *src2); }
            unsafe { *dest.offset(1) = u64x2::new(*src3, *src4); }

            i += 4;
        }
        while i < size {
            let index = indices[i];

            let src: *const u64 = &wa[index * 2] as *const T as *const u64;
            let dest: *mut u64 = &mut data[i * 2] as *mut T as *mut u64;
            unsafe { *dest = *src; }

            i += 1;
        }
    }
    fn required_work_area_size(&self) -> usize {
        self.indices.len() * 2
    }
}