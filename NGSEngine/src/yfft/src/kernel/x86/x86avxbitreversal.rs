//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::{Kernel, KernelParams, SliceAccessor};
use super::utils::{if_compatible, AlignReqKernelWrapper, AlignReqKernel, AlignInfo};
use super::super::Num;

use simd::x86::sse2::u64x2;
use simd::x86::avx::u64x4;

use std::{mem, ptr};
use simdutils;

pub unsafe fn new_x86_avx_bit_reversal_kernel<T>(indices: &Vec<usize>) -> Option<Box<Kernel<T>>>
where
    T: Num,
{
    if indices.len() < 8 {
        // doesn't benefit much
        return None;
    }

    if_compatible(|| {
        Some(Box::new(AlignReqKernelWrapper::new(
            AvxDWordBitReversalKernel { indices: indices.clone() },
        )) as Box<Kernel<f32>>)
    })
}

#[derive(Debug)]
struct AvxDWordBitReversalKernel {
    indices: Vec<usize>,
}

impl<T: Num> AlignReqKernel<T> for AvxDWordBitReversalKernel {
    fn transform<I: AlignInfo>(&self, params: &mut KernelParams<T>) {
        assert_eq!(mem::size_of::<T>(), 4);

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
                I::write(dest, u64x4::new(*src1, *src2, *src3, *src4));
                I::write(dest.offset(1), u64x4::new(*src5, *src6, *src7, *src8));
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
    fn alignment_requirement(&self) -> usize {
        32
    }
}

pub unsafe fn new_x86_avx_radix2_bit_reversal_kernel<T>(indices: &Vec<usize>) -> Option<Box<Kernel<T>>>
where
    T: Num,
{
    if indices.len() < 8 || indices.len() % 8 != 0 {
        // doesn't benefit much / requires an unaligned access
        return None;
    }

    if_compatible(|| {
        Some(Box::new(AlignReqKernelWrapper::new(
            AvxDWordRadix2BitReversalKernel {
                indices: Vec::from(&indices[0..indices.len() / 2]),
            },
        )) as Box<Kernel<f32>>)
    })
}

#[derive(Debug)]
struct AvxDWordRadix2BitReversalKernel {
    indices: Vec<usize>,
}

impl<T: Num> AlignReqKernel<T> for AvxDWordRadix2BitReversalKernel {
    fn transform<I: AlignInfo>(&self, params: &mut KernelParams<T>) {
        assert_eq!(mem::size_of::<T>(), 4);

        let indices = unsafe { SliceAccessor::new(&self.indices) };
        let size = self.indices.len();
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0..size * 4]) };
        let mut wa = unsafe { SliceAccessor::new(&mut params.work_area[0..size * 4]) };
        wa.copy_from_slice(*data);
        let mut i = 0;
        while i + 3 < size {
            let index1 = indices[i];
            let index2 = indices[i + 1];
            let index3 = indices[i + 2];
            let index4 = indices[i + 3];

            let src1 = unsafe { ptr::read_unaligned(&wa[index1 * 2] as *const T as *const u64x2) };
            let src2 = unsafe { ptr::read_unaligned(&wa[index2 * 2] as *const T as *const u64x2) };
            let src3 = unsafe { ptr::read_unaligned(&wa[index3 * 2] as *const T as *const u64x2) };
            let src4 = unsafe { ptr::read_unaligned(&wa[index4 * 2] as *const T as *const u64x2) };

            let t1a = u64x2_shuffle!(src1, src2, [0, 2]); // unpcklpd
            let t2a = u64x2_shuffle!(src3, src4, [0, 2]); // unpcklpd

            let t1b = u64x2_shuffle!(src1, src2, [1, 3]); // unpckhpd
            let t2b = u64x2_shuffle!(src3, src4, [1, 3]); // unpckhpd

            let out1: u64x4 = unsafe { simdutils::simd_shuffle4(t1a, t2a, [0, 1, 2, 3]) }; // inserti128
            let out2: u64x4 = unsafe { simdutils::simd_shuffle4(t1b, t2b, [0, 1, 2, 3]) }; // inserti128

            let dest1: *mut u64x4 = &mut data[i * 2] as *mut T as *mut u64x4;
            let dest2: *mut u64x4 = &mut data[(i + size) * 2] as *mut T as *mut u64x4;

            unsafe {
                I::write(dest1, out1);
                I::write(dest2, out2);
            }

            i += 4;
        }
        while i < size {
            let index = indices[i];

            let src = unsafe { ptr::read_unaligned(&wa[index * 2] as *const T as *const u64x2) };
            let dest1: *mut u64 = &mut data[i * 2] as *mut T as *mut u64;
            let dest2: *mut u64 = &mut data[(i + size) * 2] as *mut T as *mut u64;
            unsafe {
                *dest1 = src.extract(0);
                *dest2 = src.extract(1);
            }

            i += 1;
        }
    }
    fn required_work_area_size(&self) -> usize {
        self.indices.len() * 4
    }
    fn alignment_requirement(&self) -> usize {
        32
    }
}

pub unsafe fn new_x86_avx_radix4_bit_reversal_kernel<T>(indices: &Vec<usize>) -> Option<Box<Kernel<T>>>
where
    T: Num,
{
    if indices.len() < 16 || indices.len() % 16 != 0 {
        // doesn't benefit much / requires an unaligned access
        return None;
    }

    if_compatible(|| {
        Some(Box::new(AlignReqKernelWrapper::new(
            AvxDWordRadix4BitReversalKernel {
                indices: Vec::from(&indices[0..indices.len() / 4]),
            },
        )) as Box<Kernel<f32>>)
    })
}

#[derive(Debug)]
struct AvxDWordRadix4BitReversalKernel {
    indices: Vec<usize>,
}

impl<T: Num> AlignReqKernel<T> for AvxDWordRadix4BitReversalKernel {
    fn transform<I: AlignInfo>(&self, params: &mut KernelParams<T>) {
        assert_eq!(mem::size_of::<T>(), 4);

        let indices = unsafe { SliceAccessor::new(&self.indices) };
        let size = self.indices.len();
        let mut data = unsafe { SliceAccessor::new(&mut params.coefs[0..size * 8]) };
        let mut wa = unsafe { SliceAccessor::new(&mut params.work_area[0..size * 8]) };
        wa.copy_from_slice(*data);
        let mut i = 0;
        while i + 3 < size {
            let index1 = indices[i];
            let index2 = indices[i + 1];
            let index3 = indices[i + 2];
            let index4 = indices[i + 3];

            let src1 = unsafe { ptr::read_unaligned(&wa[index1 * 2] as *const T as *const u64x4) };
            let src2 = unsafe { ptr::read_unaligned(&wa[index2 * 2] as *const T as *const u64x4) };
            let src3 = unsafe { ptr::read_unaligned(&wa[index3 * 2] as *const T as *const u64x4) };
            let src4 = unsafe { ptr::read_unaligned(&wa[index4 * 2] as *const T as *const u64x4) };

            let t1a = u64x4_shuffle!(src1, src2, [0, 4, 2, 6]); // unpcklpd
            let t2a = u64x4_shuffle!(src3, src4, [0, 4, 2, 6]); // unpcklpd

            let t1b = u64x4_shuffle!(src1, src2, [1, 5, 3, 7]); // unpckhpd
            let t2b = u64x4_shuffle!(src3, src4, [1, 5, 3, 7]); // unpckhpd

            let out1: u64x4 = u64x4_shuffle!(t1a, t2a, [0, 1, 4, 5]); // inserti128/perm2f128
            let out2: u64x4 = u64x4_shuffle!(t1b, t2b, [0, 1, 4, 5]); // inserti128/perm2f128
            let out3: u64x4 = u64x4_shuffle!(t1a, t2a, [2, 3, 6, 7]); // inserti128/perm2f128
            let out4: u64x4 = u64x4_shuffle!(t1b, t2b, [2, 3, 6, 7]); // inserti128/perm2f128

            let dest1: *mut u64x4 = &mut data[i * 2] as *mut T as *mut u64x4;
            let dest2: *mut u64x4 = &mut data[(i + size) * 2] as *mut T as *mut u64x4;
            let dest3: *mut u64x4 = &mut data[(i + size * 2) * 2] as *mut T as *mut u64x4;
            let dest4: *mut u64x4 = &mut data[(i + size * 3) * 2] as *mut T as *mut u64x4;

            unsafe {
                I::write(dest1, out1);
                I::write(dest2, out2);
                I::write(dest3, out3);
                I::write(dest4, out4);
            }

            i += 4;
        }
        while i < size {
            let index = indices[i];

            let src = unsafe { ptr::read_unaligned(&wa[index * 2] as *const T as *const u64x4) };
            let dest1: *mut u64 = &mut data[i * 2] as *mut T as *mut u64;
            let dest2: *mut u64 = &mut data[(i + size) * 2] as *mut T as *mut u64;
            let dest3: *mut u64 = &mut data[(i + size * 2) * 2] as *mut T as *mut u64;
            let dest4: *mut u64 = &mut data[(i + size * 3) * 2] as *mut T as *mut u64;
            unsafe {
                *dest1 = src.extract(0);
                *dest2 = src.extract(1);
                *dest3 = src.extract(2);
                *dest4 = src.extract(3);
            }

            i += 1;
        }
    }
    fn required_work_area_size(&self) -> usize {
        self.indices.len() * 8
    }
    fn alignment_requirement(&self) -> usize {
        32
    }
}
