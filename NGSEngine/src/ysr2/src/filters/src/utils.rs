//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::{ptr, marker};

#[cfg(test)]
pub fn assert_num_slice_approx_eq(got: &[f32], expected: &[f32], releps: f32) {
    assert_eq!(got.len(), expected.len());
    let maxabs = expected.iter().map(|x| x.abs()).fold(
        ::std::f32::NAN,
        |x, y| x.max(y),
    ) + 0.01;
    let eps = maxabs * releps;
    for i in 0..got.len() {
        let a = got[i];
        let b = expected[i];
        if (a - b).abs() > eps {
            assert!(
                (a - b).abs() < eps,
                "assertion failed: `got almost equal to expected` \
                    (got: `{:?}`, expected: `{:?}`, diff=`{:?}`)",
                got,
                expected,
                (a - b).abs()
            );
        }
    }
}

pub struct ApplyBySample<'a, T: 'a> {
    to: *mut T,
    from: *const T,
    i: usize,
    len: usize,
    _marker: marker::PhantomData<&'a mut T>,
}

/// Call the given closure with an iterator that can be used to process the
/// signal in the sample-by-sample basis, in-place or out-place.
pub fn apply_by_sample<'a, T: 'a, F, R>(to: &'a mut [T], from: Option<&'a [T]>, cb: F) -> R
where
    T: Clone,
    F: FnOnce(ApplyBySample<'a, T>) -> R,
{
    let iter = ApplyBySample {
        to: to.as_mut_ptr(),
        from: if let Some(from) = from {
            assert!(to.len() == from.len());
            from.as_ptr()
        } else {
            ptr::null()
        },
        i: 0,
        len: to.len(),
        _marker: marker::PhantomData,
    };

    if iter.from.is_null() {
        cb(iter)
    } else {
        cb(iter)
    }
}

impl<'a, T> Iterator for ApplyBySample<'a, T>
where
    T: Clone,
{
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.len {
            None
        } else {
            unsafe {
                let r = &mut *self.to.offset(self.i as isize);
                if !self.from.is_null() {
                    *r = (&*self.from.offset(self.i as isize)).clone();
                }
                self.i += 1;
                Some(r)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<'a, T> ExactSizeIterator for ApplyBySample<'a, T>
where
    T: Clone,
{
    fn len(&self) -> usize {
        self.len - self.i
    }
}
