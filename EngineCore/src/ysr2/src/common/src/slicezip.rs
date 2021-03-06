//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::marker;
use std::borrow::{Borrow, BorrowMut};
use std::ops::Range;

pub trait IndexByVal<T> {
    fn len(&self) -> usize;
    fn get(&self, index: usize) -> Option<T> {
        if index < self.len() {
            Some(unsafe { self.get_unchecked(index) })
        } else {
            None
        }
    }
    unsafe fn get_unchecked(&self, index: usize) -> T;
}

pub trait IndexByValMut<T>: IndexByVal<T> {
    fn set(&mut self, index: usize, value: T) {
        assert!(index < self.len());
        unsafe {
            self.set_unchecked(index, value);
        }
    }
    unsafe fn set_unchecked(&mut self, index: usize, value: T);
}

/// `x: Borrow<[T]>` where the value of `x.borrow().len()` does not have an
/// interior mutability.
pub unsafe trait ImmutableLen<T>: Borrow<[T]> {}

unsafe impl<T> ImmutableLen<T> for [T] {}
unsafe impl<'a, T> ImmutableLen<T> for &'a [T] {}
unsafe impl<'a, T> ImmutableLen<T> for &'a mut [T] {}
unsafe impl<T> ImmutableLen<T> for Vec<T> {}

#[derive(Debug, Clone)]
pub struct SliceZip<'a, TArray, TSlice: ImmutableLen<T> + Borrow<[T]> + 'a, T: 'a>(
    &'a [TSlice],
    Range<usize>,
    marker::PhantomData<(T, TArray)>
);

#[derive(Debug)]
pub struct SliceZipMut<'a, TArray, TSlice: ImmutableLen<T> + BorrowMut<[T]> + 'a, T: 'a>(
    &'a mut [TSlice],
    Range<usize>,
    marker::PhantomData<(T, TArray)>
);

macro_rules! slice_zip_impl {
    ($num:expr; ($($idx:expr),*)) => (
        impl<'a, T: 'a, TSlice: ImmutableLen<T> + Borrow<[T]>> SliceZip<'a, [T; $num], TSlice, T> {
            pub fn new(x: &'a[TSlice], range: Range<usize>) -> Self {
                assert_eq!(x.len(), $num);
                for i in x.iter() {
                    &i.borrow()[range.clone()];
                }
                SliceZip(x, range, marker::PhantomData)
            }

            pub fn len(&self) -> usize {
                self.1.len()
            }

            pub fn width(&self) -> usize {
                $num
            }

            pub unsafe fn get_slice_unchecked(&self, i: usize) -> &'a [T] {
                &self.0.get_unchecked(i).borrow()[self.1.clone()]
            }

            pub fn get_slice(&self, i: usize) -> Option<&'a [T]> {
                self.0.get(i).map(|x| &x.borrow()[self.1.clone()])
            }
        }

        impl<'a, T: 'a, TSlice: ImmutableLen<T> + Borrow<[T]>> IndexByVal<[&'a T; $num]> for SliceZip<'a, [T; $num], TSlice, T> {
            fn len(&self) -> usize {
                self.1.len()
            }
            #[allow(unused_variables)]
            unsafe fn get_unchecked(&self, i: usize) -> [&'a T; $num] {
                [
                    $(self.0.get_unchecked($idx)
                        .borrow().get_unchecked(i + self.1.start)),*
                ]
            }
        }

        impl<'a, T: 'a + Clone, TSlice: ImmutableLen<T> + Borrow<[T]>> IndexByVal<[T; $num]> for SliceZip<'a, [T; $num], TSlice, T> {
            fn len(&self) -> usize {
                self.1.len()
            }
            #[allow(unused_variables)]
            unsafe fn get_unchecked(&self, i: usize) -> [T; $num] {
                [
                    $(self.0.get_unchecked($idx)
                        .borrow().get_unchecked(i + self.1.start).clone()),*
                ]
            }
        }

        impl<'a, T: 'a, TSlice: ImmutableLen<T> + BorrowMut<[T]>> SliceZipMut<'a, [T; $num], TSlice, T> {
            pub fn new(x: &'a mut[TSlice], range: Range<usize>) -> Self {
                assert_eq!(x.len(), $num);
                for i in x.iter() {
                    &i.borrow()[range.clone()];
                }
                SliceZipMut(x, range, marker::PhantomData)
            }

            pub fn len(&self) -> usize {
                self.1.len()
            }

            pub unsafe fn get_slice_unchecked(&self, i: usize) -> &'a [T] {
                // TODO: do we really need `transmute_copy`?
                let arr: &'a [TSlice] = ::std::mem::transmute_copy(&self.0);
                &arr.get_unchecked(i).borrow()[self.1.clone()]
            }

            pub fn get_slice(&self, i: usize) -> Option<&'a [T]> {
                unsafe {
                    let arr: &'a [TSlice] = ::std::mem::transmute_copy(&self.0);
                    arr.get(i).map(|x| &x.borrow()[self.1.clone()])
                }
            }

            pub unsafe fn get_slice_unchecked_mut(&self, i: usize) -> &'a mut [T] {
                let arr: &'a mut [TSlice] = ::std::mem::transmute_copy(&self.0);
                &mut arr.get_unchecked_mut(i).borrow_mut()[self.1.clone()]
            }

            pub fn get_slice_mut(&self, i: usize) -> Option<&'a mut [T]> {
                unsafe {
                    let arr: &'a mut [TSlice] = ::std::mem::transmute_copy(&self.0);
                    arr.get_mut(i).map(|x| &mut x.borrow_mut()[self.1.clone()])
                }
            }
        }

        impl<'a, T: 'a, TSlice: ImmutableLen<T> + BorrowMut<[T]>> IndexByVal<[&'a T; $num]> for SliceZipMut<'a, [T; $num], TSlice, T> {
            fn len(&self) -> usize {
                self.1.len()
            }
            #[allow(unused_variables)]
            unsafe fn get_unchecked(&self, i: usize) -> [&'a T; $num] {
                let arr: &'a [TSlice] = ::std::mem::transmute_copy(&self.0);
                [
                    $(arr.get_unchecked($idx).borrow().get_unchecked(i + self.1.start)),*
                ]
            }
        }

        impl<'a, T: 'a + Clone, TSlice: ImmutableLen<T> + BorrowMut<[T]>> IndexByVal<[T; $num]> for SliceZipMut<'a, [T; $num], TSlice, T> {
            fn len(&self) -> usize {
                self.1.len()
            }
            #[allow(unused_variables)]
            unsafe fn get_unchecked(&self, i: usize) -> [T; $num] {
                [
                    $(self.0.get_unchecked($idx).borrow().get_unchecked(i + self.1.start).clone()),*
                ]
            }
        }

        impl<'a, T: 'a + Clone, TSlice: ImmutableLen<T> + BorrowMut<[T]>> IndexByValMut<[T; $num]> for SliceZipMut<'a, [T; $num], TSlice, T> {
            #[allow(unused_variables)]
            unsafe fn set_unchecked(&mut self, i: usize, value: [T; $num]) {
                $(
                    *self.0.get_unchecked_mut($idx)
                        .borrow_mut()
                        .get_unchecked_mut(i + self.1.start) = value[$idx].clone();
                )*
            }
        }
    )
}

slice_zip_impl! { 0; () }
slice_zip_impl! { 1; (0) }
slice_zip_impl! { 2; (0, 1) }
slice_zip_impl! { 3; (0, 1, 2) }
slice_zip_impl! { 4; (0, 1, 2, 3) }
slice_zip_impl! { 5; (0, 1, 2, 3, 4) }
slice_zip_impl! { 6; (0, 1, 2, 3, 4, 5) }
slice_zip_impl! { 7; (0, 1, 2, 3, 4, 5, 6) }
slice_zip_impl! { 8; (0, 1, 2, 3, 4, 5, 6, 7) }
