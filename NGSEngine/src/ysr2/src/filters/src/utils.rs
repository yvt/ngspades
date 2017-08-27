//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

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

/// Process the signal in the sample-by-sample basis, in-place or out-place.
pub fn apply_by_sample<T, F>(to: &mut [T], from: Option<&[T]>, mut cb: F)
where
    T: Clone,
    F: FnMut(&T) -> T,
{
    if let Some(from) = from {
        assert!(to.len() == from.len());
        for i in 0..to.len() {
            to[i] = cb(&from[i]);
        }
    } else {
        for x in to.iter_mut() {
            *x = cb(x);
        }
    }
}
